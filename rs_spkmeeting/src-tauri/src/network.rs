use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub websocket_url: String,
    pub api_url: String,
}

#[tauri::command]
pub async fn get_server_config() -> Result<ServerConfig, String> {
    Ok(ServerConfig {
        websocket_url: "ws://127.0.0.1:8081/ws".to_string(),
        api_url: "http://127.0.0.1:8081".to_string(),
    })
}


