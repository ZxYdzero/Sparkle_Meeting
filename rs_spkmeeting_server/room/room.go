package room

import (
	"encoding/json"
	"sync"

	"webrtc/model"
	"webrtc/webrtc"

	"github.com/google/uuid"
)

// Room 房间
type Room struct {
	uuid   string
	name   string
	master string
	limits int
	member map[int]*Member
	lock   sync.RWMutex
	sfu    *webrtc.SFURoom // SFU 媒体转发
}

// NewRoom 创建新房间
func NewRoom(name string, masterID string, limits int) *Room {
	return &Room{
		uuid:   uuid.NewString(),
		name:   name,
		master: masterID,
		limits: limits,
		member: make(map[int]*Member),
		sfu:    webrtc.NewSFURoom(uuid.NewString()), // TODO: 使用房间 ID
	}
}

// ID 获取房间ID
func (r *Room) ID() string {
	return r.uuid
}

// Name 获取房间名称
func (r *Room) Name() string {
	return r.name
}

// Master 获取房主ID
func (r *Room) Master() string {
	return r.master
}

// SFU 获取 SFU 实例
func (r *Room) SFU() *webrtc.SFURoom {
	if r == nil {
		return nil
	}
	return r.sfu
}

// Join 成员加入房间
func (r *Room) Join(m *Member) bool {
	r.lock.Lock()
	defer r.lock.Unlock()

	if m.roomIndex > 0 {
		return false
	}
	if r.limits <= len(r.member) {
		return false
	}
	for i := 1; i <= r.limits; i++ {
		if _, empty := r.member[i]; !empty {
			m.room = r         // 设置房间指针
			m.roomIndex = i
			r.member[i] = m
			return true
		}
	}
	return false
}

// Leave 成员离开房间
func (r *Room) Leave(m *Member) bool {
	r.lock.Lock()
	defer r.lock.Unlock()

	if m.roomIndex < 0 || m.room != r {
		return false
	}

	// 房主离开，解散房间
	if m.index == r.master {
		for _, mem := range r.member {
			close(mem.send)
			mem.room = nil
			mem.roomIndex = -1
		}
		r.member = make(map[int]*Member)
		r.sfu.Close() // 关闭 SFU
		return true
	}

	// 普通成员离开
	if _, exists := r.member[m.roomIndex]; exists {
		delete(r.member, m.roomIndex)
		close(m.send)
		m.room = nil
		m.roomIndex = -1
	}

	// 房间为空，返回 true 以便删除
	if len(r.member) == 0 {
		r.sfu.Close() // 关闭 SFU
		return true
	}

	return false
}

// Broadcast 广播消息给房间内所有成员（除发送者外）
func (r *Room) Broadcast(msg []byte, senderID string) {
	r.lock.RLock()
	defer r.lock.RUnlock()

	for _, m := range r.member {
		if m.ID() == senderID {
			continue
		}

		select {
		case m.send <- msg:
		default:
			// 发送缓冲区满，丢弃
		}
	}
}

// SendWelcome 发送欢迎消息
func (r *Room) SendWelcome(m *Member) {
	welcomeMsg := model.SignalMessage{
		Type:     model.SignalTypeWelcome,
		FromUser: m.ID(),
	}
	if bytes, err := json.Marshal(welcomeMsg); err == nil {
		m.send <- bytes
	}
}

// SendJoinNotify 发送加入通知
func (r *Room) SendJoinNotify(m *Member) {
	joinMsg := model.SignalMessage{
		Type:     model.SignalTypeJoin,
		FromUser: m.ID(),
		FromName: m.Name(),
	}
	if bytes, err := json.Marshal(joinMsg); err == nil {
		r.Broadcast(bytes, m.ID())
	}
}

// MemberCount 获取成员数量
func (r *Room) MemberCount() int {
	r.lock.RLock()
	defer r.lock.RUnlock()
	return len(r.member)
}

// GetMembers 获取所有成员
func (r *Room) GetMembers() []*Member {
	r.lock.RLock()
	defer r.lock.RUnlock()
	members := make([]*Member, 0, len(r.member))
	for _, m := range r.member {
		members = append(members, m)
	}
	return members
}

// GetMember 获取成员
func (r *Room) GetMember(userID string) *Member {
	r.lock.RLock()
	defer r.lock.RUnlock()
	for _, m := range r.member {
		if m.index == userID {
			return m
		}
	}
	return nil
}
