package router

import (
	"encoding/json"
	"log"
	"net/http"

	"webrtc/model"
	"webrtc/room"
	"webrtc/webrtc"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

// CreateRoomReq 创建房间请求
type CreateRoomReq struct {
	Name   string `json:"name"`
	Limits int    `json:"limits"`
}

// CreateRoomResp 创建房间响应
type CreateRoomResp struct {
	RoomID   string `json:"room_id"`
	MasterID string `json:"master_id"`
}

// StatsResp 统计响应
type StatsResp struct {
	RoomCount   int `json:"room_count"`
	UserCount   int `json:"user_count"`
	ActiveRooms int `json:"active_rooms"`
}

// ICEServer ICE 服务器配置
type ICEServer struct {
	URLs       []string `json:"urls"`
	Username   string   `json:"username,omitempty"`
	Credential string   `json:"credential,omitempty"`
}

// ConfigResp WebRTC 配置响应
type ConfigResp struct {
	ICEServers []ICEServer `json:"iceServers"`
}

// UpdateBandwidthReq 更新带宽请求
type UpdateBandwidthReq struct {
	RoomID        string `json:"room_id"`
	VideoBitrate  uint64 `json:"video_bitrate"`  // 目标视频码率 (bps)
	AudioPriority bool   `json:"audio_priority"` // 是否优先保障音频
}

// Router 路由器
type Router struct {
	roomMgr *room.Manager
	signalH *webrtc.Handler
}

// NewRouter 创建路由器
func NewRouter(roomMgr *room.Manager) *Router {
	return &Router{
		roomMgr: roomMgr,
		signalH: webrtc.NewHandler(),
	}
}

// RegisterRoutes 注册路由
func (r *Router) RegisterRoutes(rg *gin.RouterGroup) {
	api := rg.Group("/api")
	{
		api.POST("/create", r.CreateRoom)
		api.GET("/stats", r.GetStats)
		api.GET("/config", r.GetConfig)
		api.GET("/ws", r.ServeWebSocket)
	}
}

// CreateRoom 创建房间
func (r *Router) CreateRoom(c *gin.Context) {
	c.Header("Access-Control-Allow-Origin", "*")
	c.Header("Content-Type", "application/json")

	var req CreateRoomReq
	if err := c.ShouldBindJSON(&req); err != nil {
		req.Name = "默认房间"
		req.Limits = 10
	}

	masterID := uuid.NewString()
	room := r.roomMgr.CreateRoom(masterID, req.Name, req.Limits)

	log.Printf("创建房间: roomID=%s, masterID=%s, name=%s", room.ID(), masterID, req.Name)

	c.JSON(http.StatusOK, CreateRoomResp{
		RoomID:   room.ID(),
		MasterID: masterID,
	})
}

// GetStats 获取房间统计信息
func (r *Router) GetStats(c *gin.Context) {
	c.Header("Access-Control-Allow-Origin", "*")
	c.Header("Content-Type", "application/json")

	roomCount := r.roomMgr.RoomCount()
	userCount := r.roomMgr.UserCount()

	c.JSON(http.StatusOK, StatsResp{
		RoomCount:   roomCount,
		UserCount:   userCount,
		ActiveRooms: roomCount,
	})
}

// GetConfig 获取 WebRTC 配置（ICE 服务器）
func (r *Router) GetConfig(c *gin.Context) {
	c.Header("Access-Control-Allow-Origin", "*")
	c.Header("Content-Type", "application/json")

	// 返回 ICE 服务器配置
	// TODO: 可以从配置文件或环境变量读取
	config := ConfigResp{
		ICEServers: []ICEServer{
			// STUN 服务器
			{URLs: []string{"stun:115.190.111.141:3478"}},
			// TURN 服务器 - 暂时使用静态凭证，生产环境建议使用动态凭证
			{
				URLs:       []string{"turn:115.190.111.141:3478?transport=udp"},
				Username:   "coturn",
				Credential: "coturn",
			},
			// 添加 TCP 支持（某些企业网络只允许 TCP）
			{
				URLs:       []string{"turn:115.190.111.141:3478?transport=tcp"},
				Username:   "coturn",
				Credential: "coturn",
			},
		},
	}

	c.JSON(http.StatusOK, config)
}

// ServeWebSocket WebSocket 连接处理
func (r *Router) ServeWebSocket(c *gin.Context) {
	roomID := c.Query("room_id")
	userID := c.Query("user_id")
	name := c.Query("name")

	log.Printf("WebSocket连接请求: roomID=%s, userID=%q, name=%s", roomID, userID, name)

	if roomID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "Room ID required"})
		return
	}

	rm := r.roomMgr.GetRoom(roomID)
	if rm == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "Room not found"})
		return
	}

	conn, err := upgrader.Upgrade(c.Writer, c.Request, nil)
	if err != nil {
		return
	}

	member := room.NewMember(userID, name, conn, r.roomMgr)

	if rm.Join(member) {
		// 在 SFU 中创建 Peer
		sfuRoom := rm.SFU()
		if sfuRoom == nil {
			log.Printf("SFU Room 为 nil，房间 ID: %s", rm.ID())
			conn.Close()
			return
		}

		// 设置 renegotiation 回调
		sfuRoom.SetRenegotiationCallback(func(sfuPeerID string, offerSDP string) {
			// 动态获取房间引用，遍历所有成员找到对应的 peer
			currentRoom := r.roomMgr.GetRoom(rm.ID())
			if currentRoom == nil {
				return
			}
			for _, m := range currentRoom.GetMembers() {
				if m.SFUPeerID() == sfuPeerID {
					offerMsg := model.SignalMessage{
						Type: model.SignalTypeOffer,
						Data: mustMarshal(model.SessionDescription{Type: "offer", Sdp: offerSDP}),
					}
					if bytes, err := json.Marshal(offerMsg); err == nil {
						m.Send(bytes)
						log.Printf("[SFU] 发送 renegotiation Offer 给 Peer %s", sfuPeerID)
					}
					break
				}
			}
		})

		sfuPeer, err := sfuRoom.AddPeer(member.SFUPeerID())
		if err != nil {
			log.Printf("SFU Peer 创建失败: %v", err)
			conn.Close()
			return
		}

		// 监听 ICE 候选并发送给客户端
		sfuPeer.OnICECandidate(func(candidate string, sdpMid *string, sdpMLineIndex *uint16) {
			candMsg := model.SignalMessage{
				Type: model.SignalTypeCandidate,
				Data: mustMarshal(model.IceCandidate{
					Candidate:     candidate,
					SdpMid:        sdpMid,
					SdpMLineIndex: sdpMLineIndex,
				}),
			}
			if bytes, err := json.Marshal(candMsg); err == nil {
				member.Send(bytes)
			}
		})

		// 发送欢迎消息
		welcomeMsg := model.SignalMessage{
			Type:     model.SignalTypeWelcome,
			FromUser: member.ID(),
		}
		if bytes, err := json.Marshal(welcomeMsg); err == nil {
			member.Send(bytes)
		}

		// 启动写入循环
		go member.WritePump()

		// 发送加入通知
		rm.SendJoinNotify(member)

		// 启动读取循环
		member.ReadPump()
	} else {
		conn.WriteMessage(websocket.TextMessage, []byte("Room is full"))
		conn.Close()
	}
}

func mustMarshal(v interface{}) json.RawMessage {
	data, _ := json.Marshal(v)
	return data
}
