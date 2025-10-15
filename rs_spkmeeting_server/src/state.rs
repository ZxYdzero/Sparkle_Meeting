use std::sync::Arc;

use actix::prelude::*;
use tracing::{info, debug};
use crate::ws_text::WsText;
use dashmap::DashMap;

/// 全局应用状态：房间与 peers 映射 (完全无锁版本)
#[derive(Clone, Default)]
pub struct AppState {
    /// room -> set of users (使用 DashMap<String, DashMap<String, ()>> 完全无锁)
    pub rooms: Arc<DashMap<String, DashMap<String, ()>>>,
    /// user -> Recipient<WsText> (使用 DashMap 无锁 HashMap)
    pub peers: Arc<DashMap<String, Recipient<WsText>>>,
}

impl AppState {
    pub fn new() -> Self {
        info!("🏗️  Creating new application state (无锁版本)");
        let state = Self::default();

        // Log initial state
        debug!("📊 Initial state: rooms={}, peers={}",
                   state.rooms.len(),
                   state.peers.len());

        state
    }

    /// 获取统计信息 (无锁版本)
    pub fn get_stats(&self) -> (usize, usize) {
        let rooms_count = self.rooms.len();
        let peers_count = self.peers.len();
        (rooms_count, peers_count)
    }

    /// 获取房间详细信息 (完全无锁版本)
    pub fn get_room_details(&self) -> std::collections::HashMap<String, Vec<String>> {
        self.rooms.iter()
            .map(|entry| (entry.key().clone(), entry.value().iter().map(|user_entry| user_entry.key().clone()).collect()))
            .collect()
    }
}
