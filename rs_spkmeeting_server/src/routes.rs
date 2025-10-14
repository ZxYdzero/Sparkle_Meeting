use actix_web::{get, web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;
use crate::state::AppState;
use crate::ws::WsConn;
use tracing::info;

#[get("/ws")]
pub async fn websocket_index(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    info!("🔌 New WebSocket connection request from: {:?}", req.connection_info().peer_addr());

    let state = data.get_ref().clone();
    let ws = WsConn::new(state);

    match ws::start(ws, &req, stream) {
        Ok(response) => {
            tracing::debug!("✅ WebSocket handshake successful");
            Ok(response)
        },
        Err(e) => {
            tracing::error!("❌ WebSocket handshake failed: {}", e);
            Err(e)
        }
    }
}

/// return members of a room as JSON
#[get("/rooms/{room}/members")]
pub async fn room_members(req: HttpRequest, path: web::Path<(String,)>, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let room = &path.0;
    let conn_info = req.connection_info();
    let client_ip = conn_info.peer_addr().unwrap_or("unknown");

    tracing::info!("🏠 API request: Get members for room '{}' from client: {}", room, client_ip);

    // optional API key check
    if let Ok(key) = std::env::var("API_KEY") {
        if !key.is_empty() {
            let provided = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()).unwrap_or("");
            if provided != key {
                tracing::warn!("🔒 Unauthorized API request to room '{}' from client: {} (invalid API key)", room, client_ip);
                return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Invalid or missing API key"
                })));
            } else {
                tracing::debug!("🔐 API key validated for client: {}", client_ip);
            }
        }
    }

    // 使用无锁的 DashMap 查询房间成员
    if let Some(set) = data.rooms.get(room) {
        let members: Vec<String> = set.iter().cloned().collect();
        tracing::info!("✅ Room '{}' has {} members: {:?}", room, members.len(), members);

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "room": room,
            "members": members,
            "count": members.len()
        })))
    } else {
        tracing::info!("📭 Room '{}' not found or empty", room);
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "room": room,
            "members": [],
            "count": 0,
            "message": "Room not found or empty"
        })))
    }
}
