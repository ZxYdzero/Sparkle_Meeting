use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfiguration {
    pub signal_server: String,
    pub turn_server: Option<String>,
    pub turn_username: Option<String>,
    pub turn_password: Option<String>,
    pub default_audio_input: Option<String>,
    pub default_audio_output: Option<String>,
    pub default_volume: u32,
}

impl Default for AppConfiguration {
    fn default() -> Self {
        Self {
            signal_server: "ws://localhost:8080".to_string(),
            turn_server: None,
            turn_username: None,
            turn_password: None,
            default_audio_input: None,
            default_audio_output: None,
            default_volume: 50,
        }
    }
}

#[tauri::command]
pub async fn save_config(app: tauri::AppHandle, config: AppConfiguration) -> Result<bool, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let config_path = config_dir.join("config.json");

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(config_path, config_json.as_bytes())
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(true)
}

#[tauri::command]
pub async fn load_config(app: tauri::AppHandle) -> Result<AppConfiguration, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let config_path = config_dir.join("config.json");

    if !config_path.exists() {
        return Ok(AppConfiguration::default());
    }

    let config_content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: AppConfiguration = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to deserialize config: {}", e))?;

    Ok(config)
}

#[tauri::command]
pub async fn reset_config(app: tauri::AppHandle) -> Result<bool, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let config_path = config_dir.join("config.json");

    if config_path.exists() {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to remove config file: {}", e))?;
    }

    Ok(true)
}