pub mod server;
pub mod security;
pub mod logging;

use std::path::Path;
use anyhow::Result;
use tracing::{info, warn};

use crate::config::server::ServerConfig;
use crate::config::security::SecurityConfig;
use crate::config::logging::LoggingConfig;

/// 应用配置结构
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}

impl AppConfig {
    /// 加载配置文件
    pub fn load() -> Result<Self> {
        let config_dir = Path::new("config");

        // 确保配置目录存在
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
            info!("Created config directory");
        }

        // 检查配置文件是否存在，如果不存在则创建默认配置
        let server_config = Self::ensure_server_config(&config_dir)?;
        let security_config = Self::ensure_security_config(&config_dir)?;
        let logging_config = Self::ensure_logging_config(&config_dir)?;

        info!("Configuration loaded successfully");

        Ok(AppConfig {
            server: server_config,
            security: security_config,
            logging: logging_config,
        })
    }

    /// 确保服务器配置文件存在
    fn ensure_server_config(config_dir: &Path) -> Result<ServerConfig> {
        let config_file = config_dir.join("server.toml");

        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            let config: ServerConfig = toml::from_str(&content)?;
            info!("Loaded server configuration from {}", config_file.display());
            Ok(config)
        } else {
            warn!("Server config file not found, creating default configuration");
            let default_config = ServerConfig::default();
            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_file, content)?;
            info!("Created default server configuration at {}", config_file.display());
            Ok(default_config)
        }
    }

    /// 确保安全配置文件存在
    fn ensure_security_config(config_dir: &Path) -> Result<SecurityConfig> {
        let config_file = config_dir.join("security.toml");

        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            let config: SecurityConfig = toml::from_str(&content)?;
            info!("Loaded security configuration from {}", config_file.display());
            Ok(config)
        } else {
            warn!("Security config file not found, creating default configuration");
            let default_config = SecurityConfig::default();
            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_file, content)?;
            info!("Created default security configuration at {}", config_file.display());
            Ok(default_config)
        }
    }

    /// 确保日志配置文件存在
    fn ensure_logging_config(config_dir: &Path) -> Result<LoggingConfig> {
        let config_file = config_dir.join("logging.toml");

        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            let config: LoggingConfig = toml::from_str(&content)?;
            info!("Loaded logging configuration from {}", config_file.display());
            Ok(config)
        } else {
            warn!("Logging config file not found, creating default configuration");
            let default_config = LoggingConfig::default();
            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_file, content)?;
            info!("Created default logging configuration at {}", config_file.display());
            Ok(default_config)
        }
    }

    /// 从环境变量覆盖配置
    pub fn load_with_env_overrides() -> Result<Self> {
        let mut config = Self::load()?;

        // 环境变量覆盖
        if let Ok(host) = std::env::var("HOST") {
            config.server.server.host = host;
        }
        if let Ok(port) = std::env::var("PORT") {
            config.server.server.port = port.parse()
                .map_err(|_| anyhow::anyhow!("Invalid PORT value"))?;
        }
        if let Ok(log_level) = std::env::var("RUST_LOG") {
            config.logging.level = log_level;
        }
        if let Ok(api_key) = std::env::var("API_KEY") {
            config.security.api.key = api_key;
        }

        info!("Environment overrides applied");
        Ok(config)
    }
}