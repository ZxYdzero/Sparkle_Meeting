package webrtc

import (
	"encoding/json"

	"webrtc/model"
)

// Handler WebRTC 信令处理器
type Handler struct{}

// NewHandler 创建信令处理器
func NewHandler() *Handler {
	return &Handler{}
}

// HandleSignal 处理信令消息
func (h *Handler) HandleSignal(msg *model.SignalMessage) []byte {
	// 根据消息类型处理
	switch msg.Type {
	case model.SignalTypeOffer, model.SignalTypeAnswer:
		return h.handleSessionDescription(msg)
	case model.SignalTypeCandidate:
		return h.handleIceCandidate(msg)
	case model.SignalTypeChat:
		return h.handleChat(msg)
	case model.SignalTypeMute, model.SignalTypeUnmute:
		return h.handleControl(msg)
	case model.SignalTypeScreenShare:
		return h.handleScreenShare(msg)
	default:
		// 未知类型，直接转发
		data, _ := json.Marshal(msg)
		return data
	}
}

// handleSessionDescription 处理 SDP offer/answer
func (h *Handler) handleSessionDescription(msg *model.SignalMessage) []byte {
	var sdp model.SessionDescription
	if err := msg.ParseData(&sdp); err != nil {
		return nil
	}

	data, _ := json.Marshal(msg)
	return data
}

// handleIceCandidate 处理 ICE candidate
func (h *Handler) handleIceCandidate(msg *model.SignalMessage) []byte {
	var candidate model.IceCandidate
	if err := msg.ParseData(&candidate); err != nil {
		return nil
	}

	data, _ := json.Marshal(msg)
	return data
}

// handleChat 处理聊天消息
func (h *Handler) handleChat(msg *model.SignalMessage) []byte {
	var chat model.ChatMessage
	if err := msg.ParseData(&chat); err != nil {
		return nil
	}

	data, _ := json.Marshal(msg)
	return data
}

// handleControl 处理控制消息
func (h *Handler) handleControl(msg *model.SignalMessage) []byte {
	var control model.ControlMessage
	if err := msg.ParseData(&control); err != nil {
		return nil
	}

	data, _ := json.Marshal(msg)
	return data
}

// handleScreenShare 处理屏幕共享
func (h *Handler) handleScreenShare(msg *model.SignalMessage) []byte {
	data, _ := json.Marshal(msg)
	return data
}
