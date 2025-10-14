// 引入所有模块
mod media;
mod screen_capture;
mod network;

use std::sync::Arc;
use screen_capture::ScreenCaptureState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化应用状态 - 只保留屏幕捕获状态
    let screen_capture_state = Arc::new(ScreenCaptureState {
        active_captures: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(screen_capture_state)
        // 所有命令处理器 - 只保留底层设备管理功能
        .invoke_handler(tauri::generate_handler![
            // 媒体设备相关命令
            media::enumerate_media_devices,
            media::get_user_media,
            // 屏幕共享相关命令
            screen_capture::enumerate_screens,
            screen_capture::start_screen_capture,
            screen_capture::stop_screen_capture,
            screen_capture::get_capture_status,
            // 网络配置命令
            network::get_server_config,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(debug_assertions)] // only in debug builds
            {
                window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
