mod config;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            config::save_config,
            config::load_config,
            config::reset_config,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // 启动时自动创建默认配置文件
            if let Err(e) = config::init_config(app.handle()) {
                eprintln!("初始化配置失败: {}", e);
            }

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
