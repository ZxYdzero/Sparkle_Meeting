package webrtc

import (
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"webrtc/config"
	"webrtc/model"

	"github.com/pion/webrtc/v3"
)

// SFUPeer SFU 中的单个客户端连接
type SFUPeer struct {
	peerConnection       *webrtc.PeerConnection
	id                   string
	room                 *SFURoom
	mu                   sync.RWMutex
	pendingCandidates    []webrtc.ICECandidateInit
	remoteDescriptionSet bool
	forwardedTracks      map[string]bool
	renegotiationTimer   *time.Timer // renegotiation 延迟定时器
	renegotiationTimerMu sync.Mutex  // 保护定时器
}

// SFURoom SFU 房间，管理所有 Peer
type SFURoom struct {
	peers                 map[string]*SFUPeer
	mu                    sync.RWMutex
	api                   *webrtc.API
	roomID                string
	forwardedTracks       map[string]*trackForward
	onRenegotiationNeeded func(peerID string, offer string)
}

// trackForward 轨道转发信息
type trackForward struct {
	localTrack  *webrtc.TrackLocalStaticRTP
	remoteTrack *webrtc.TrackRemote
	stopChan    chan struct{}
	senderID    string
	senders     map[string]*webrtc.RTPSender
}

// NewSFURoom 创建 SFU 房间
func NewSFURoom(roomID string) *SFURoom {
	// 加载配置
	cfg := config.Get()

	// 验证配置
	if err := cfg.Validate(); err != nil {
		fmt.Printf("[SFU] 配置验证失败: %v\n", err)
	}

	mediaEngine := &webrtc.MediaEngine{}
	if err := mediaEngine.RegisterDefaultCodecs(); err != nil {
		panic(err)
	}

	// 设置网络策略
	settingEngine := webrtc.SettingEngine{}

	// 从配置获取 UDP 端口范围
	udpMin, udpMax := cfg.GetUDPPortRange()
	if udpMin > 0 && udpMax > 0 {
		settingEngine.SetEphemeralUDPPortRange(uint16(udpMin), uint16(udpMax))
		fmt.Printf("[SFU] UDP 端口范围: %d-%d\n", udpMin, udpMax)
	}

	// 设置 NAT 1:1 IP 映射
	publicIP := cfg.GetPublicIP()
	if publicIP != "" {
		settingEngine.SetNAT1To1IPs([]string{publicIP}, webrtc.ICECandidateTypeHost)
		fmt.Printf("[SFU] 公网 IP: %s\n", publicIP)
	}

	// 创建 WebRTC API
	api := webrtc.NewAPI(
		webrtc.WithMediaEngine(mediaEngine),
		webrtc.WithSettingEngine(settingEngine),
	)

	fmt.Printf("[SFU] SFU 房间已创建: %s\n", roomID)

	return &SFURoom{
		peers:           make(map[string]*SFUPeer),
		api:             api,
		roomID:          roomID,
		forwardedTracks: make(map[string]*trackForward),
	}
}

// AddPeer 添加新 Peer
func (r *SFURoom) AddPeer(peerID string) (*SFUPeer, error) {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.peers[peerID]; exists {
		return nil, fmt.Errorf("peer already exists: %s", peerID)
	}

	// 创建 PeerConnection
	peerConnection, err := r.api.NewPeerConnection(webrtc.Configuration{})
	if err != nil {
		return nil, err
	}

	peer := &SFUPeer{
		peerConnection:       peerConnection,
		id:                   peerID,
		room:                 r,
		pendingCandidates:    make([]webrtc.ICECandidateInit, 0),
		remoteDescriptionSet: false,
		forwardedTracks:      make(map[string]bool),
	}

	// 监听轨道
	peerConnection.OnTrack(func(track *webrtc.TrackRemote, receiver *webrtc.RTPReceiver) {
		// 使用 streamID 作为前后端一致的标识
		streamID := track.StreamID()
		trackID := track.ID()
		fmt.Printf("[SFU] Peer %s 收到轨道: streamID=%s, trackID=%s, kind=%s\n", peerID, streamID, trackID, track.Kind().String())
		r.BroadcastTrack(track, peerID)
	})

	// 监听 ICE 连接状态
	peerConnection.OnICEConnectionStateChange(func(state webrtc.ICEConnectionState) {
		fmt.Printf("[SFU] Peer %s ICE 状态: %s\n", peerID, state.String())
		// 当 ICE 连接建立后，检查是否有待转发的轨道需要重新协商
		if state == webrtc.ICEConnectionStateConnected || state == webrtc.ICEConnectionStateCompleted {
			// 延迟一点触发，确保远程描述已设置
			go func() {
				time.Sleep(500 * time.Millisecond)
				r.mu.RLock()
				peer := r.peers[peerID]
				r.mu.RUnlock()

				if peer == nil || !peer.remoteDescriptionSet {
					return
				}

				// 检查是否有转发的轨道
				hasForwardedTracks := false
				for trackID := range r.forwardedTracks {
					if !peer.forwardedTracks[trackID] {
						hasForwardedTracks = true
						break
					}
				}

				if hasForwardedTracks {
					fmt.Printf("[SFU] ICE 已连接，为 Peer %s 触发 renegotiation 以获取现有轨道\n", peerID)
					r.createAndSendOffer(peer)
				}
			}()
		}
	})

	r.peers[peerID] = peer
	return peer, nil
}

// RemovePeer 移除 Peer
func (r *SFURoom) RemovePeer(peerID string) {
	r.mu.Lock()
	defer r.mu.Unlock()

	if peer, exists := r.peers[peerID]; exists {
		peer.peerConnection.Close()
		delete(r.peers, peerID)
		fmt.Printf("[SFU] Peer %s 已移除\n", peerID)
	}
}

// CreateOffer 创建 Offer
func (p *SFUPeer) CreateOffer() (string, error) {
	offer, err := p.peerConnection.CreateOffer(nil)
	if err != nil {
		return "", err
	}

	// 收集 ICE 候选
	gatherComplete := webrtc.GatheringCompletePromise(p.peerConnection)
	p.peerConnection.SetLocalDescription(offer)
	<-gatherComplete

	return p.peerConnection.LocalDescription().SDP, nil
}

// SetRemoteDescription设置远程描述
func (p *SFUPeer) SetRemoteDescription(sdp string, typeStr string) error {
	p.mu.Lock()
	defer p.mu.Unlock()

	var sdType webrtc.SDPType
	switch typeStr {
	case "offer":
		sdType = webrtc.SDPTypeOffer
	case "answer":
		sdType = webrtc.SDPTypeAnswer
	default:
		return fmt.Errorf("unknown SDP type: %s", typeStr)
	}

	err := p.peerConnection.SetRemoteDescription(webrtc.SessionDescription{
		SDP:  sdp,
		Type: sdType,
	})

	if err == nil {
		p.remoteDescriptionSet = true
		// 远程描述设置后，处理缓存的 ICE candidates
		p.processPendingCandidates()
	}

	return err
}

// CreateAnswer 创建 Answer
func (p *SFUPeer) CreateAnswer() (string, error) {
	answer, err := p.peerConnection.CreateAnswer(nil)
	if err != nil {
		return "", err
	}

	// 收集 ICE 候选
	gatherComplete := webrtc.GatheringCompletePromise(p.peerConnection)
	p.peerConnection.SetLocalDescription(answer)
	<-gatherComplete

	return p.peerConnection.LocalDescription().SDP, nil
}

// OnICECandidate 监听 ICE 候选
func (p *SFUPeer) OnICECandidate(handler func(candidate string, sdpMid *string, sdpMLineIndex *uint16)) {
	p.peerConnection.OnICECandidate(func(c *webrtc.ICECandidate) {
		if c == nil {
			return
		}
		handler(c.ToJSON().Candidate, c.ToJSON().SDPMid, c.ToJSON().SDPMLineIndex)
	})
}

// AddICECandidate 添加 ICE 候选
func (p *SFUPeer) AddICECandidate(candidateStr string, sdpMid *string, sdpMLineIndex *uint16) error {
	p.mu.Lock()
	defer p.mu.Unlock()

	candidate := webrtc.ICECandidateInit{
		Candidate:     candidateStr,
		SDPMid:        sdpMid,
		SDPMLineIndex: sdpMLineIndex,
	}

	// 如果远程描述还没设置，先缓存
	if !p.remoteDescriptionSet {
		p.pendingCandidates = append(p.pendingCandidates, candidate)
		fmt.Printf("[SFU] 缓存 ICE 候选，等待远程描述\n")
		return nil
	}

	return p.peerConnection.AddICECandidate(candidate)
}

// processPendingCandidates 处理缓存的 ICE 候选
func (p *SFUPeer) processPendingCandidates() {
	fmt.Printf("[SFU] 处理 %d 个缓存的 ICE 候选\n", len(p.pendingCandidates))
	for _, candidate := range p.pendingCandidates {
		if err := p.peerConnection.AddICECandidate(candidate); err != nil {
			fmt.Printf("[SFU] 添加 ICE 候选失败: %v\n", err)
		}
	}
	p.pendingCandidates = nil
}

// BroadcastTrack 广播轨道给其他 Peer
func (r *SFURoom) BroadcastTrack(remoteTrack *webrtc.TrackRemote, senderID string) {
	trackID := remoteTrack.ID()

	r.mu.Lock()
	// 检查是否已经为这个轨道创建了转发
	if _, exists := r.forwardedTracks[trackID]; exists {
		r.mu.Unlock()
		fmt.Printf("[SFU] 轨道 %s 已有转发，跳过\n", trackID)
		return
	}

	// 创建本地轨道用于转发
	localTrack, err := webrtc.NewTrackLocalStaticRTP(
		remoteTrack.Codec().RTPCodecCapability,
		remoteTrack.ID(),
		remoteTrack.StreamID(),
	)
	if err != nil {
		r.mu.Unlock()
		fmt.Printf("[SFU] 创建本地轨道失败: %v\n", err)
		return
	}

	// 为当前所有已有 peer 添加轨道
	senders := make(map[string]*webrtc.RTPSender)
	for peerID, peer := range r.peers {
		if peerID == senderID {
			continue
		}

		if sender, err := r.addTrackToPeer(peer, localTrack, trackID); err != nil {
			fmt.Printf("[SFU] 为 Peer %s 添加轨道失败: %v\n", peerID, err)
		} else if sender != nil {
			senders[peerID] = sender
		}
	}

	// 启动转发协程
	stopChan := make(chan struct{})
	r.forwardedTracks[trackID] = &trackForward{
		localTrack:  localTrack,
		remoteTrack: remoteTrack,
		stopChan:    stopChan,
		senderID:    senderID, // 记录轨道的原始发送者
		senders:     senders,
	}
	r.mu.Unlock()

	fmt.Printf("[SFU] 启动轨道 %s 的转发\n", trackID)

	// 启动 RTP 转发协程
	go func() {
		defer r.cleanupTrack(trackID) // 确保轨道结束时清理资源

		rtpBuf := make([]byte, 1500)
		for {
			select {
			case <-stopChan:
				fmt.Printf("[SFU] 停止轨道 %s 的转发\n", trackID)
				return
			default:
				n, _, err := remoteTrack.Read(rtpBuf)
				if err != nil {
					fmt.Printf("[SFU] 轨道 %s 结束: %v\n", trackID, err)
					return
				}
				if _, err := localTrack.Write(rtpBuf[:n]); err != nil {
					// 写入失败
					fmt.Printf("[SFU] 写入轨道 %s 失败: %v\n", trackID, err)
				}
			}
		}
	}()
}

// cleanupTrack 清理轨道转发资源
func (r *SFURoom) cleanupTrack(trackID string) {
	r.mu.Lock()
	defer r.mu.Unlock()

	forward, exists := r.forwardedTracks[trackID]
	if !exists {
		return
	}

	// 关闭 stopChan
	select {
	case <-forward.stopChan:
		// 已经关闭
	default:
		close(forward.stopChan)
	}

	// 从所有 peer 移除 sender
	for peerID, sender := range forward.senders {
		if peer := r.peers[peerID]; peer != nil {
			// 移除轨道
			if err := peer.peerConnection.RemoveTrack(sender); err != nil {
				fmt.Printf("[SFU] 移除轨道 %s 从 Peer %s 失败: %v\n", trackID, peerID, err)
			} else {
				fmt.Printf("[SFU] 移除轨道 %s 从 Peer %s\n", trackID, peerID)
			}
			// 清理 peer 的 forwardedTracks 记录
			delete(peer.forwardedTracks, trackID)
		}
	}

	// 清理转发记录
	delete(r.forwardedTracks, trackID)
	fmt.Printf("[SFU] 清理轨道 %s 的转发资源\n", trackID)
}

// addTrackToPeer 为指定 peer 添加轨道
func (r *SFURoom) addTrackToPeer(peer *SFUPeer, localTrack *webrtc.TrackLocalStaticRTP, trackID string) (*webrtc.RTPSender, error) {
	// 检查是否已经添加过
	if peer.forwardedTracks[trackID] {
		return nil, nil
	}

	rtpSender, err := peer.peerConnection.AddTrack(localTrack)
	if err != nil {
		return nil, err
	}

	peer.forwardedTracks[trackID] = true
	fmt.Printf("[SFU] 添加轨道 %s 到 Peer %s\n", trackID, peer.id)

	// 启动 RTCP 处理协程
	go func() {
		rtcpBuf := make([]byte, 1500)
		for {
			if _, _, err := rtpSender.Read(rtcpBuf); err != nil {
				return
			}
		}
	}()

	// 使用 debounce 延迟触发 renegotiation，避免 audio/video 同时到达时触发多次
	peer.scheduleRenegotiation(r, 300*time.Millisecond)

	return rtpSender, nil
}

// scheduleRenegotiation 延迟触发 renegotiation（debounce 机制）
func (p *SFUPeer) scheduleRenegotiation(room *SFURoom, delay time.Duration) {
	p.renegotiationTimerMu.Lock()
	defer p.renegotiationTimerMu.Unlock()

	// 停止之前的定时器
	if p.renegotiationTimer != nil {
		p.renegotiationTimer.Stop()
	}

	// 创建新的定时器
	p.renegotiationTimer = time.AfterFunc(delay, func() {
		if room.onRenegotiationNeeded != nil {
			room.createAndSendOffer(p)
		}
	})
}

// createAndSendOffer 为指定 peer 创建并发送新 Offer（用于 renegotiation）
func (r *SFURoom) createAndSendOffer(peer *SFUPeer) {
	peer.mu.Lock()
	if !peer.remoteDescriptionSet {
		peer.mu.Unlock()
		// 远程描述还没设置，不能 renegotiation
		return
	}
	peer.mu.Unlock()

	// 检查信令状态，避免在初始连接期间触发 renegotiation
	currentSignalingState := peer.peerConnection.SignalingState()
	if currentSignalingState != webrtc.SignalingStateStable {
		fmt.Printf("[SFU] Peer %s 信令状态为 %s，跳过 renegotiation\n", peer.id, currentSignalingState.String())
		return
	}

	// 检查 ICE 连接状态，只有在 connected 时才 renegotiation
	iceState := peer.peerConnection.ICEConnectionState()
	if iceState != webrtc.ICEConnectionStateConnected && iceState != webrtc.ICEConnectionStateCompleted {
		fmt.Printf("[SFU] Peer %s ICE 状态为 %s，跳过 renegotiation\n", peer.id, iceState.String())
		return
	}

	offer, err := peer.peerConnection.CreateOffer(nil)
	if err != nil {
		fmt.Printf("[SFU] 创建 Offer 失败: %v\n", err)
		return
	}

	// 收集 ICE 候选
	gatherComplete := webrtc.GatheringCompletePromise(peer.peerConnection)
	peer.peerConnection.SetLocalDescription(offer)
	<-gatherComplete

	offerSDP := peer.peerConnection.LocalDescription().SDP
	fmt.Printf("[SFU] 为 Peer %s 创建新 Offer (renegotiation)\n", peer.id)

	// 通过回调发送 Offer
	if r.onRenegotiationNeeded != nil {
		r.onRenegotiationNeeded(peer.id, offerSDP)
	}
}

// SetRenegotiationCallback 设置 renegotiation 回调（只设置一次）
func (r *SFURoom) SetRenegotiationCallback(callback func(peerID string, offer string)) {
	r.mu.Lock()
	defer r.mu.Unlock()
	// 只在回调未设置时才设置，避免被覆盖
	if r.onRenegotiationNeeded == nil {
		r.onRenegotiationNeeded = callback
	}
}

// HandleSignalMessage 处理信令消息
func (r *SFURoom) HandleSignalMessage(peerID string, msg *model.SignalMessage) ([]byte, error) {
	r.mu.RLock()
	peer, exists := r.peers[peerID]
	r.mu.RUnlock()

	if !exists {
		fmt.Printf("[SFU] Peer %s 不存在 (消息类型: %s)\n", peerID, msg.Type)
		return nil, fmt.Errorf("peer not found: %s", peerID)
	}

	switch msg.Type {
	case model.SignalTypeOffer:
		// 设置远程 Offer 并创建 Answer
		var sdp model.SessionDescription
		if err := msg.ParseData(&sdp); err != nil {
			return nil, err
		}

		if err := peer.SetRemoteDescription(sdp.Sdp, "offer"); err != nil {
			return nil, err
		}

		r.mu.RLock()
		for trackID, forward := range r.forwardedTracks {
			// 跳过当前 peer 自己发送的轨道
			if forward.senderID == peerID {
				continue
			}

			// 检查当前 peer 是否已经有这个轨道
			peer.mu.RLock()
			alreadyForwarded := peer.forwardedTracks[trackID]
			peer.mu.RUnlock()

			if !alreadyForwarded {
				if _, err := r.addTrackToPeer(peer, forward.localTrack, trackID); err != nil {
					fmt.Printf("[SFU] 为 Peer %s 添加轨道 %s 失败: %v\n", peerID, trackID, err)
				}
			}
		}
		r.mu.RUnlock()

		answerSDP, err := peer.CreateAnswer()
		if err != nil {
			return nil, err
		}

		resp := model.SignalMessage{
			Type: model.SignalTypeAnswer,
			Data: mustMarshal(model.SessionDescription{Type: "answer", Sdp: answerSDP}),
		}
		return json.Marshal(resp)

	case model.SignalTypeAnswer:
		var sdp model.SessionDescription
		if err := msg.ParseData(&sdp); err != nil {
			return nil, err
		}

		if err := peer.SetRemoteDescription(sdp.Sdp, "answer"); err != nil {
			return nil, err
		}
		return nil, nil

	case model.SignalTypeCandidate:
		var cand model.IceCandidate
		if err := msg.ParseData(&cand); err != nil {
			return nil, err
		}

		if err := peer.AddICECandidate(cand.Candidate, cand.SdpMid, cand.SdpMLineIndex); err != nil {
			return nil, err
		}
		return nil, nil

	default:
		return nil, fmt.Errorf("unknown message type: %s", msg.Type)
	}
}

// Close 关闭房间
func (r *SFURoom) Close() {
	r.mu.Lock()
	defer r.mu.Unlock()

	for _, peer := range r.peers {
		peer.peerConnection.Close()
	}
	r.peers = make(map[string]*SFUPeer)
}

func mustMarshal(v interface{}) json.RawMessage {
	data, _ := json.Marshal(v)
	return data
}
