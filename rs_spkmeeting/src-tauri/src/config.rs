use serde::{Deserialize, Serialize};

/// 初始化配置文件
pub fn init_config(_app: &tauri::AppHandle) -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe parent directory")?
        .to_path_buf();

    let config_path = exe_dir.join("config.toml");

    // 如果配置文件已存在，不需要做任何事情
    if config_path.exists() {
        return Ok(());
    }

    // 创建配置目录
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // 创建默认配置文件
    let default_config = AppConfiguration::default();
    let config_toml = toml::to_string_pretty(&default_config)
        .map_err(|e| format!("Failed to serialize default config: {}", e))?;

    std::fs::write(&config_path, config_toml.as_bytes())
        .map_err(|e| format!("Failed to write default config file: {}", e))?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfiguration {
    pub server: ServerConfig,
    pub ice_servers: Vec<IceServer>,
    pub default_audio_input: Option<String>,
    pub default_audio_output: Option<String>,
    pub default_volume: u32,
}

impl Default for AppConfiguration {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 9090,
            },
            ice_servers: vec![],
            default_audio_input: None,
            default_audio_output: None,
            default_volume: 50,
        }
    }
}

#[tauri::command]
pub async fn save_config(_app: tauri::AppHandle, config: AppConfiguration) -> Result<bool, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe parent directory")?
        .to_path_buf();

    let config_path = exe_dir.join("config.toml");

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config_toml = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(config_path, config_toml.as_bytes())
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(true)
}

#[tauri::command]
pub async fn load_config(_app: tauri::AppHandle) -> Result<AppConfiguration, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe parent directory")?
        .to_path_buf();

    let config_path = exe_dir.join("config.toml");

    let config_content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: AppConfiguration = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to deserialize config: {}", e))?;

    Ok(config)
}

#[tauri::command]
pub async fn reset_config(_app: tauri::AppHandle) -> Result<bool, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe parent directory")?
        .to_path_buf();

    let config_path = exe_dir.join("config.toml");

    if config_path.exists() {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to remove config file: {}", e))?;
    }

    Ok(true)
}