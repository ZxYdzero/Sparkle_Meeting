mod config;
mod state;
mod ws_text;
mod ws;
mod routes;

use actix_web::{App, HttpServer, web, middleware, http::{Method, header::HeaderName}};
use actix_cors::Cors;
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载配置
    let config = config::AppConfig::load_with_env_overrides()
        .expect("Failed to load configuration");

    // 初始化日志系统
    config.logging.init().expect("Failed to initialize logging");

    tracing::info!("🚀 Starting SPK Meeting Server v0.1.0");
    tracing::info!("📝 Configuration loaded successfully");
    tracing::info!("🌐 Server will listen on {}:{}", config.server.server.host, config.server.server.port);
    tracing::info!("🔌 WebSocket endpoint: ws://{}:{}/ws", config.server.server.host, config.server.server.port);
    tracing::info!("🏠 Room API endpoint: http://{}:{}/rooms/<room>/members", config.server.server.host, config.server.server.port);

    // 检查 API 密钥配置
    if config.security.api.require_key && !config.security.api.key.is_empty() {
        tracing::info!("🔐 API Key authentication is enabled");
    } else if config.security.api.require_key {
        tracing::warn!("⚠️  API Key authentication required but no key provided");
    } else {
        tracing::info!("🔓 API Key authentication disabled");
    }

    let state = AppState::default();
    let addr = format!("{}:{}", config.server.server.host, config.server.server.port);

    // 创建 CORS 配置数据
    let cors_allowed_origins = config.server.cors.allowed_origins.clone();
    let cors_max_age = config.security.cors.max_age as usize;

    let server = HttpServer::new(move || {
        // 在 closure 内创建 CORS 中间件
        let allowed_origins = cors_allowed_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                let origin_str = origin.to_str().ok().map(|s| s.to_string());
                allowed_origins.iter().any(|allowed| {
                    match origin_str.as_ref() {
                        Some(o) => o == allowed,
                        None => false,
                    }
                }) || allowed_origins.contains(&"*".to_string())
            })
            .allowed_methods(vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allowed_headers(vec![HeaderName::from_static("*")])
            .supports_credentials()
            .max_age(cors_max_age);

        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(routes::websocket_index)
            .service(routes::room_members)
    })
    .workers(config.server.server.workers)
    .bind(&addr)
    .map_err(|e| {
        tracing::error!("💥 Failed to bind to {}: {}", addr, e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    tracing::info!("✅ Server successfully bound to {}", addr);
    tracing::info!("🎯 Server is ready to accept connections");
    tracing::info!("👥 Using {} worker threads", config.server.server.workers);

    // 启动服务器并等待结果
    let result = server.run().await;

    match result {
        Ok(_) => {
            tracing::info!("🛑 Server stopped gracefully");
        },
        Err(e) => {
            tracing::error!("💥 Server stopped with error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
