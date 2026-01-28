package room

import "sync"

// Manager 房间管理器
type Manager struct {
	rooms map[string]*Room
	lock  sync.RWMutex
}

// NewManager 创建房间管理器
func NewManager() *Manager {
	return &Manager{
		rooms: make(map[string]*Room),
	}
}

// CreateRoom 创建房间
func (m *Manager) CreateRoom(masterID string, name string, limits int) *Room {
	m.lock.Lock()
	defer m.lock.Unlock()
	room := NewRoom(name, masterID, limits)
	m.rooms[room.ID()] = room
	return room
}

// GetRoom 获取房间
func (m *Manager) GetRoom(roomID string) *Room {
	m.lock.RLock()
	defer m.lock.RUnlock()
	if room, ok := m.rooms[roomID]; ok {
		return room
	}
	return nil
}

// DeleteRoom 删除房间
func (m *Manager) DeleteRoom(roomID string) {
	m.lock.Lock()
	defer m.lock.Unlock()
	delete(m.rooms, roomID)
}

// RoomCount 获取房间数量
func (m *Manager) RoomCount() int {
	m.lock.RLock()
	defer m.lock.RUnlock()
	return len(m.rooms)
}

// UserCount 获取所有房间总用户数
func (m *Manager) UserCount() int {
	m.lock.RLock()
	defer m.lock.RUnlock()
	count := 0
	for _, room := range m.rooms {
		count += room.MemberCount()
	}
	return count
}
