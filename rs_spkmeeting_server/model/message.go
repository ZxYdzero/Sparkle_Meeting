package model

import "encoding/json"

// MessageType 消息类型
type MessageType string

const (
	// WebRTC 信令
	SignalTypeOffer     MessageType = "offer"
	SignalTypeAnswer    MessageType = "answer"
	SignalTypeCandidate MessageType = "candidate"

	// 房间消息
	SignalTypeJoin    MessageType = "join"
	SignalTypeBye     MessageType = "bye"
	SignalTypeWelcome MessageType = "welcome"

	// 聊天消息
	SignalTypeChat MessageType = "chat"

	// 控制消息
	SignalTypeMute        MessageType = "mute"
	SignalTypeUnmute      MessageType = "unmute"
	SignalTypeScreenShare MessageType = "screen_share"
)

// SignalMessage 信令消息
type SignalMessage struct {
	Type     MessageType     `json:"type"`
	Data     json.RawMessage `json:"data,omitempty"`
	FromUser string          `json:"from_user,omitempty"`
	FromName string          `json:"from_name,omitempty"`
	ToUser   string          `json:"to_user,omitempty"`
	RoomID   string          `json:"room_id,omitempty"`
	Time     int64           `json:"time,omitempty"`
}

// GetDataString 获取 Data 的字符串形式
func (m *SignalMessage) GetDataString() string {
	return string(m.Data)
}

// ParseData 解析 Data 到指定结构
func (m *SignalMessage) ParseData(v interface{}) error {
	return json.Unmarshal(m.Data, v)
}

// SessionDescription SDP 会话描述
type SessionDescription struct {
	Type string `json:"type"`
	Sdp  string `json:"sdp"`
}

// IceCandidate ICE 候选者
type IceCandidate struct {
	Candidate     string  `json:"candidate"`
	SdpMLineIndex *uint16 `json:"sdpMLineIndex,omitempty"`
	SdpMid        *string `json:"sdpMid,omitempty"`
}

// ChatMessage 聊天消息
type ChatMessage struct {
	Content string `json:"content"`
}

// ControlMessage 控制消息
type ControlMessage struct {
	Target string `json:"target"` // "audio", "video", "screen"
	Action string `json:"action"` // "on", "off", "toggle"
}
