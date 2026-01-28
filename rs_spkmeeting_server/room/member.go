package room

import (
	"encoding/json"
	"log"

	"webrtc/model"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

// Member 房间成员
type Member struct {
	index     string
	name      string
	conn      *websocket.Conn
	room      *Room
	roomIndex int
	send      chan []byte
	manager   *Manager
	sfuPeerID string // SFU Peer ID
}

// NewMember 创建新成员
func NewMember(userID string, name string, conn *websocket.Conn, mgr *Manager) *Member {
	if userID == "" {
		userID = uuid.NewString()
	}
	return &Member{
		index:     userID,
		name:      name,
		conn:      conn,
		room:      nil,
		roomIndex: -1,
		send:      make(chan []byte, 256),
		manager:   mgr,
		sfuPeerID: uuid.NewString(),
	}
}

// ID 获取成员ID
func (m *Member) ID() string {
	return m.index
}

// Name 获取成员名称
func (m *Member) Name() string {
	return m.name
}

// RoomID 获取房间ID
func (m *Member) RoomID() string {
	if m.room != nil {
		return m.room.ID()
	}
	return ""
}

// SFUPeerID 获取 SFU Peer ID
func (m *Member) SFUPeerID() string {
	return m.sfuPeerID
}

// Send 发送消息
func (m *Member) Send(msg []byte) bool {
	select {
	case m.send <- msg:
		return true
	default:
		return false
	}
}

// Close 关闭连接
func (m *Member) Close() {
	close(m.send)
}

// WritePump 写入循环
func (m *Member) WritePump() {
	defer func() {
		m.conn.Close()
	}()

	for {
		select {
		case message, ok := <-m.send:
			if !ok {
				m.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			// 每条消息单独发送，避免 JSON 粘连
			if err := m.conn.WriteMessage(websocket.TextMessage, message); err != nil {
				return
			}

			// 处理缓冲区中等待的消息
			for i := 0; i < len(m.send); i++ {
				msg := <-m.send
				if err := m.conn.WriteMessage(websocket.TextMessage, msg); err != nil {
					return
				}
			}
		}
	}
}

// ReadPump 读取循环
func (m *Member) ReadPump() {
	defer func() {
		if m.room != nil {
			// 发送离开消息
			byeMsg := model.SignalMessage{
				Type:     model.SignalTypeBye,
				FromUser: m.index,
				FromName: m.name,
			}
			if bytes, err := json.Marshal(byeMsg); err == nil {
				m.room.Broadcast(bytes, m.index)
			}

			// 从 SFU 移除
			if sfuRoom := m.room.SFU(); sfuRoom != nil {
				sfuRoom.RemovePeer(m.sfuPeerID)
			}

			// 离开房间前先保存 roomID，因为 Leave 后 m.room 可能为 nil
			roomID := m.room.ID()
			if m.room.Leave(m) {
				m.manager.DeleteRoom(roomID)
			}
		}
		m.conn.Close()
	}()

	for {
		_, message, err := m.conn.ReadMessage()
		if err != nil {
			break
		}

		room := m.room
		if room == nil {
			continue
		}

		var signal model.SignalMessage
		if err := json.Unmarshal(message, &signal); err != nil {
			log.Printf("JSON 解析错误: %v", err)
			continue
		}

		signal.FromUser = m.index
		signal.FromName = m.name

		// 处理 SFU 信令
		if signal.Type == model.SignalTypeOffer ||
			signal.Type == model.SignalTypeAnswer ||
			signal.Type == model.SignalTypeCandidate {

			// 使用 SFU 处理
			response, err := room.SFU().HandleSignalMessage(m.sfuPeerID, &signal)
			if err != nil {
				log.Printf("SFU 处理错误: %v", err)
				continue
			}

			// 如果有响应 发送回客户端
			if response != nil && signal.Type == model.SignalTypeOffer {
				m.Send(response)
			}
		} else {
			// 其他消息直接转发
			responseBytes, err := json.Marshal(signal)
			if err != nil {
				continue
			}
			room.Broadcast(responseBytes, m.index)
		}
	}
}
