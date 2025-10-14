use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub console: ConsoleConfig,
    pub file: FileConfig,
    pub rotation: RotationConfig,
    pub filters: FiltersConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    pub enabled: bool,
    pub colored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub enabled: bool,
    pub path: String,
    pub max_size: String,
    pub max_files: u32,
    pub append: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    pub daily: bool,
    pub hourly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiltersConfig {
    pub exclude_paths: Vec<String>,
    pub log_request_body: bool,
    pub log_response_body: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            console: ConsoleConfig::default(),
            file: FileConfig::default(),
            rotation: RotationConfig::default(),
            filters: FiltersConfig::default(),
        }
    }
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            colored: true,
        }
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "logs/spkmeeting.log".to_string(),
            max_size: "10MB".to_string(),
            max_files: 5,
            append: true,
        }
    }
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            daily: true,
            hourly: false,
        }
    }
}

impl Default for FiltersConfig {
    fn default() -> Self {
        Self {
            exclude_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
            ],
            log_request_body: false,
            log_response_body: false,
        }
    }
}

impl LoggingConfig {
    /// 初始化日志系统
    pub fn init(&self) -> anyhow::Result<()> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.level));

        // 控制台日志
        if self.console.enabled {
            match self.format.as_str() {
                "json" => {
                    tracing_subscriber::fmt()
                        .with_env_filter(env_filter)
                        .json()
                        .with_writer(std::io::stdout)
                        .with_ansi(self.console.colored)
                        .init();
                },
                "compact" => {
                    tracing_subscriber::fmt()
                        .with_env_filter(env_filter)
                        .compact()
                        .with_writer(std::io::stdout)
                        .with_ansi(self.console.colored)
                        .init();
                },
                _ => {
                    tracing_subscriber::fmt()
                        .with_env_filter(env_filter)
                        .pretty()
                        .with_writer(std::io::stdout)
                        .with_ansi(self.console.colored)
                        .init();
                }
            }
        } else {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_ansi(false)
                .init();
        }

        Ok(())
    }
}