use actix::prelude::*;
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};
use crate::state::AppState;
use crate::ws_text::WsText;
use dashmap::DashMap;

/// 信令消息类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalMessage {
    Join { room: String, user: String, session_id: Option<String> },
    Leave { room: String, user: String, session_id: Option<String> },
    Offer { to: String, from: String, sdp: String },
    Answer { to: String, from: String, sdp: String },
    Ice { to: String, from: String, candidate: String },
    TriggerOffer { target_user: String, room: String },
    Ping,
}

pub struct WsConn {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub state: AppState,
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("🔌 New WebSocket connection established from: {:?}", ctx.address());
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(id) = &self.id {
            info!("👋 User '{}' disconnected from WebSocket", id);

            // remove from peers
            let removed_peer = self.state.peers.remove(id);
            if removed_peer.is_some() {
                debug!("🗑️  Removed user '{}' from peers registry", id);
            }

            // remove from any rooms and collect rooms left (无锁操作)
            let mut left_rooms: Vec<String> = Vec::new();
            // 先收集所有房间名
            let room_names: Vec<String> = self.state.rooms.iter().map(|r| r.key().clone()).collect();

            for room_name in room_names {
                let mut should_remove_room = false;
            
                if let Some(mut members) = self.state.rooms.get_mut(&room_name) {
                    if members.remove(id).is_some() {
                        left_rooms.push(room_name.clone());
                        if members.is_empty() {
                            should_remove_room = true;
                        }
                    }
                }
            
                if should_remove_room {
                    self.state.rooms.remove(&room_name);
                }
            }

            if !left_rooms.is_empty() {
                info!("🚪 User '{}' left rooms: {:?}", id, left_rooms);
            }

            // notify remaining peers in each room (无锁操作)
            for room in left_rooms {
                let note = serde_json::json!({"type":"leave","user":id,"room":room}).to_string();

                // 获取房间内剩余的成员 (无锁操作)
                let members_to_notify: Vec<String> = {
                    if let Some(members) = self.state.rooms.get(&room) {
                        members.iter().map(|entry| entry.key().clone()).collect()
                    } else {
                        Vec::new()
                    }
                };

                // 通知剩余成员 (无锁操作)
                debug!("📢 Notifying {} members in room '{}' about user '{}' leaving", members_to_notify.len(), room, id);
                for member in members_to_notify {
                    if let Some(addr) = self.state.peers.get(&member) {
                        addr.do_send(WsText(note.clone()));
                    }
                }
            }
        } else {
            info!("🔌 Anonymous WebSocket connection closed");
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => {
                debug!("📨 Received message from WebSocket: {}", text);
                match serde_json::from_str::<SignalMessage>(&text) {
                    Ok(msg) => {
                        debug!("✅ Successfully parsed message: {:?}", msg);
                        self.handle_signal(msg, ctx);
                    },
                    Err(e) => {
                        warn!("❌ Invalid message format: {} -> {}", e, text);
                    }
                }
            }
            Ok(ws::Message::Ping(_)) => {
                debug!("🏓 Received ping, sending pong");
                ctx.pong(&[]);
            },
            Ok(ws::Message::Pong(_)) => {
                debug!("🏓 Received pong");
            },
            Ok(ws::Message::Binary(_)) => {
                warn!("⚠️  Received unexpected binary message");
            },
            Ok(ws::Message::Close(reason)) => {
                info!("🔌 WebSocket closing: {:?}", reason);
                ctx.stop();
            },
            _ => {
                warn!("⚠️  Received unhandled WebSocket message type");
            }
        }
    }
}

impl WsConn {
    pub fn new(state: AppState) -> Self { Self { id: None, session_id: None, state } }

    fn handle_signal(&mut self, msg: SignalMessage, ctx: &mut ws::WebsocketContext<Self>) {
        match msg {
            SignalMessage::Join { room, user, session_id } => {
                info!("👤 User '{}' attempting to join room '{}' with session_id: {:?}", user, room, session_id);

                // 记录连接尝试时间戳，用于调试房间锁定问题
                let join_attempt_time = std::time::Instant::now();

                self.id = Some(user.clone());
                self.session_id = session_id.clone();

                // 增强的重复连接检测 - 修复房间锁定问题
                let user_already_in_room = {
                    if let Some(members) = self.state.rooms.get(&room) {
                        members.contains_key(&user)
                    } else {
                        false
                    }
                };

                let user_has_active_connection = {
                    if let Some(_) = self.state.peers.get(&user) {
                        warn!("⚠️ User '{}' already has an active peer connection. This might be a room lock issue.", user);
                        true
                    } else {
                        false
                    }
                };

                // 如果用户已经在房间中或有活跃连接，需要清理
                if user_already_in_room || user_has_active_connection {
                    warn!("⚠️ ROOM LOCK ISSUE: User '{}' has existing state (in_room: {}, has_connection: {}). Cleaning up previous session first.",
                          user, user_already_in_room, user_has_active_connection);

                    // 强制清理用户之前的会话
                    self.cleanup_user_session(&user, &room);

                    // 添加延迟以确保清理完成
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    info!("✅ Previous session cleanup completed for user '{}', proceeding with join", user);

                    // 强制清理房间内所有可能的问题连接
                    self.cleanup_problematic_connections_in_room(&room, &user);
                }

                // 记录Join成功，用于调试
                info!("🔐 Join processing took {}ms for user '{}' in room '{}'",
                      join_attempt_time.elapsed().as_millis(), user, room);

                // add to room (完全无锁操作)
                info!("🔓 Adding user '{}' to room '{}' (完全无锁操作)", user, room);
                let member_count = {
                    let members = self.state.rooms.entry(room.clone()).or_insert_with(DashMap::new);
                    let _was_empty = members.is_empty();
                    members.insert(user.clone(), ());
                    info!("✅ User '{}' successfully joined room '{}'. Room now has {} members",
                               user, room, members.len());
                    members.len()
                };

                // register Recipient in peers map (无锁操作)
                info!("🔓 Registering user '{}' in peers registry (无锁操作)", user);
                let recipient = ctx.address().recipient::<WsText>();
                self.state.peers.insert(user.clone(), recipient);
                info!("📝 User '{}' registered in peers registry", user);

                // 向自己发送加入成功的确认消息
                let join_success_msg = serde_json::json!({"type":"join","user":user,"room":room}).to_string();
                info!("📤 Preparing to send join confirmation to user '{}' for room '{}'", user, room);
                info!("📤 Message content: {}", join_success_msg);
                ctx.text(join_success_msg.clone());
                info!("📤 Sent join confirmation to user '{}' for room '{}'", user, room);

                // notify existing room members about new user and trigger Offer sending (无锁操作)
                if member_count > 1 {
                    info!("🔓 Notifying existing members about new user '{}' and triggering Offer sending (无锁操作)", user);

                    // 获取房间内需要通知的成员 (除了新加入的用户)
                    let members_to_notify: Vec<String> = {
                        if let Some(members) = self.state.rooms.get(&room) {
                            members.iter()
                                .filter(|entry| entry.key() != &user)
                                .map(|entry| entry.key().clone())
                                .collect()
                        } else {
                            Vec::new()
                        }
                    };

                    if !members_to_notify.is_empty() {
                        let join_msg = serde_json::json!({"type":"join","user":user,"room":room}).to_string();
                        let trigger_msg = serde_json::json!({
                            "type":"trigger_offer",
                            "new_user": user,
                            "room": room,
                            "action": "send_offer"
                        }).to_string();

                        let mut notified_count = 0;
                        for member in members_to_notify {
                            if let Some(addr) = self.state.peers.get(&member) {
                                // 1. 发送join通知
                                addr.do_send(WsText(join_msg.clone()));

                                // 2. 发送触发Offer的消息
                                addr.do_send(WsText(trigger_msg.clone()));
                                notified_count += 1;
                            }
                        }

                        if notified_count > 0 {
                            info!("📢 Notified {} existing members about user '{}' joining and triggered Offer sending for room '{}'",
                                      notified_count, user, room);
                        }
                    }
                } else {
                    info!("🏠 User '{}' is the first member in room '{}'", user, room);
                }
            }
            SignalMessage::Leave { room, user, session_id } => {
                info!("👤 User '{}' leaving room '{}' with session_id: {:?}", user, room, session_id);

                // 验证会话ID（如果提供的话）
                if let Some(current_session_id) = &self.session_id {
                    if let Some(provided_session_id) = &session_id {
                        if current_session_id != provided_session_id {
                            warn!("⚠️ Session ID mismatch for user '{}'. Current: {:}, Provided: {:?}",
                                  user, current_session_id, provided_session_id);
                            // 可以选择是否继续处理Leave消息
                        }
                    }
                }

                // 从房间中移除用户 (完全无锁操作)
                let should_remove_room = {
                    if let Some(members) = self.state.rooms.get_mut(&room) {
                        let member_count_before = members.len();
                        members.remove(&user);
                        let member_count_after = members.len();

                        if members.is_empty() {
                            info!("🏠 Room '{}' is now empty and will be removed", room);
                            true
                        } else {
                            info!("🏠 Room '{}' now has {} members (was {})", room, member_count_after, member_count_before);
                            false
                        }
                    } else {
                        info!("⚠️ Room '{}' not found when user '{}' tried to leave", room, user);
                        false
                    }
                };

                // 如果房间为空，删除房间
                if should_remove_room {
                    self.state.rooms.remove(&room);
                }

                // 从 peers 注册表中移除用户 (无锁操作)
                let removed = self.state.peers.remove(&user);
                if removed.is_some() {
                    debug!("📝 User '{}' removed from peers registry", user);
                }

                // 通知房间内剩余成员 (完全无锁操作) - 排除离开的用户
                let members_to_notify: Vec<String> = {
                    if let Some(members) = self.state.rooms.get(&room) {
                        members.iter()
                            .filter(|entry| entry.key() != &user)  // 排除离开的用户
                            .map(|entry| entry.key().clone())
                            .collect()
                    } else {
                        Vec::new()
                    }
                };

                if !members_to_notify.is_empty() {
                    let note = serde_json::json!({"type":"leave","user":user.clone(),"room":room.clone()}).to_string();
                    let mut notified_count = 0;

                    for member in members_to_notify {
                        if let Some(addr) = self.state.peers.get(&member) {
                            addr.do_send(WsText(note.clone()));
                            notified_count += 1;
                        }
                    }

                    if notified_count > 0 {
                        info!("📢 Notified {} remaining members about user '{}' leaving room '{}'",
                                  notified_count, user, room);
                    }
                } else {
                    info!("🏠 No remaining members to notify in room '{}'", room);
                }
            }
            SignalMessage::Offer { to, from, sdp } => {
                debug!("📤 Forwarding WebRTC Offer from '{}' to '{}'", from, to);
                if let Some(recipient) = self.state.peers.get(&to) {
                    let forward = serde_json::json!({"type":"Offer","from":from,"sdp":sdp}).to_string();
                    recipient.do_send(WsText(forward));
                    debug!("✅ Successfully forwarded Offer from '{}' to '{}'", from, to);
                } else {
                    warn!("⚠️  Cannot forward Offer - target user '{}' not found", to);
                }
            }
            SignalMessage::Answer { to, from, sdp } => {
                debug!("📤 Forwarding WebRTC Answer from '{}' to '{}'", from, to);
                if let Some(recipient) = self.state.peers.get(&to) {
                    let forward = serde_json::json!({"type":"Answer","from":from,"sdp":sdp}).to_string();
                    recipient.do_send(WsText(forward));
                    debug!("✅ Successfully forwarded Answer from '{}' to '{}'", from, to);
                } else {
                    warn!("⚠️  Cannot forward Answer - target user '{}' not found", to);
                }
            }
            SignalMessage::Ice { to, from, candidate } => {
                debug!("📤 Forwarding ICE Candidate from '{}' to '{}'", from, to);
                if let Some(recipient) = self.state.peers.get(&to) {
                    let forward = serde_json::json!({"type":"ice","from":from,"candidate":candidate}).to_string();
                    recipient.do_send(WsText(forward));
                    debug!("✅ Successfully forwarded ICE Candidate from '{}' to '{}'", from, to);
                } else {
                    warn!("⚠️  Cannot forward ICE Candidate - target user '{}' not found", to);
                }
            }
            SignalMessage::Ping => {
                debug!("🏓 Received Ping, sending Pong response");
                ctx.text(serde_json::json!({"type":"pong"}).to_string());
            }
            SignalMessage::TriggerOffer { .. } => {
                warn!("⚠️  Received unexpected TriggerOffer message - this should only be sent from server to client");
            }
        }
    }
}

impl WsConn {
    /// 清理用户之前的会话 - 修复房间锁定问题
    fn cleanup_user_session(&mut self, user: &str, room: &str) {
        info!("🧹 Cleaning up previous session for user '{}' in room '{}'", user, room);

        // 从peers注册表中移除旧的连接
        if let Some((_, old_addr)) = self.state.peers.remove(user) {
            info!("🗑️ Removed old peer connection for user '{}'", user);

            // 通知旧连接断开（如果还在连接的话）
            let disconnect_msg = serde_json::json!({
                "type": "session_conflict",
                "user": user,
                "room": room,
                "message": "New connection detected, disconnecting old session"
            }).to_string();

            if let Err(e) = old_addr.try_send(WsText(disconnect_msg)) {
                warn!("⚠️ Failed to notify old connection about session conflict: {}", e);
            }
        }

        // 从房间中移除用户（如果需要的话）
        if let Some(members) = self.state.rooms.get_mut(room) {
            members.remove(user);
            info!("🗑️ Removed user '{}' from room '{}' during cleanup", user, room);
        }

        info!("✅ Session cleanup completed for user '{}'", user);
    }

    /// 清理房间内所有有问题的连接 - 彻底解决房间锁定问题
    fn cleanup_problematic_connections_in_room(&mut self, room: &str, joining_user: &str) {
        info!("🔧 Cleaning up problematic connections in room '{}' for user '{}'", room, joining_user);

        // 获取房间内的所有成员
        let room_members: Vec<String> = {
            if let Some(members) = self.state.rooms.get(room) {
                members.iter().map(|entry| entry.key().clone()).collect()
            } else {
                Vec::new()
            }
        };

        // 检查每个成员的连接状态
        let mut problematic_users: Vec<String> = Vec::new();
        for member in &room_members {
            if let Some(addr) = self.state.peers.get(member) {
                // 尝试发送ping消息来检查连接状态
                let ping_msg = serde_json::json!({"type":"ping"}).to_string();
                if let Err(e) = addr.try_send(WsText(ping_msg)) {
                    warn!("⚠️ User '{}' in room '{}' has broken connection (cannot send ping): {}", member, room, e);
                    problematic_users.push(member.clone());
                }
            } else {
                warn!("⚠️ User '{}' in room '{}' has no peer registry entry", member, room);
                problematic_users.push(member.clone());
            }
        }

        // 清理有问题的用户
        for problematic_user in &problematic_users {
            warn!("🔧 Removing problematic user '{}' from room '{}'", problematic_user, room);

            // 从peers中移除
            self.state.peers.remove(problematic_user);

            // 从房间中移除
            if let Some(members) = self.state.rooms.get_mut(room) {
                members.remove(problematic_user);
            }

            // 通知其他用户（如果还有的话）
            let disconnect_msg = serde_json::json!({
                "type": "user_disconnected",
                "user": problematic_user,
                "room": room,
                "reason": "Connection problem, removed from room"
            }).to_string();

            for member in &room_members {
                if member != problematic_user {
                    if let Some(addr) = self.state.peers.get(member) {
                        if let Err(e) = addr.try_send(WsText(disconnect_msg.clone())) {
                            warn!("⚠️ Failed to notify user '{}' about disconnect: {}", member, e);
                        }
                    }
                }
            }
        }

        if problematic_users.len() > 0 {
            info!("🧹 Cleaned up {} problematic users from room '{}': {:?}", problematic_users.len(), room, problematic_users);
        } else {
            info!("✅ All connections in room '{}' appear to be healthy", room);
        }
    }
}

impl Handler<WsText> for WsConn { type Result = (); fn handle(&mut self, msg: WsText, ctx: &mut ws::WebsocketContext<Self>) -> Self::Result { ctx.text(msg.0); } }
