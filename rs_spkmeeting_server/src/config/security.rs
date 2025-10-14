use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub api: ApiConfig,
    pub rate_limiting: RateLimitConfig,
    pub authentication: AuthenticationConfig,
    pub cors: CorsSecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub require_key: bool,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub session_timeout: u64,
    pub max_concurrent_sessions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsSecurityConfig {
    pub allow_credentials: bool,
    pub max_age: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            api: ApiConfig::default(),
            rate_limiting: RateLimitConfig::default(),
            authentication: AuthenticationConfig::default(),
            cors: CorsSecurityConfig::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            require_key: false,
            key: String::new(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_minute: 100,
            burst_size: 10,
        }
    }
}

impl Default for AuthenticationConfig {
    fn default() -> Self {
        Self {
            session_timeout: 3600, // 1 hour
            max_concurrent_sessions: 5,
        }
    }
}

impl Default for CorsSecurityConfig {
    fn default() -> Self {
        Self {
            allow_credentials: true,
            max_age: 86400, // 24 hours
        }
    }
}