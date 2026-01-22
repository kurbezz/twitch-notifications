use std::env;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub twitch: TwitchConfig,
    pub telegram: TelegramConfig,
    pub discord: DiscordConfig,
    pub jwt: JwtConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub frontend_url: String,
    pub webhook_url: String,
    /// Whether to set the `Secure` flag on cookies.
    /// If `None`, the application may infer this from `frontend_url` (e.g. `https` -> true).
    /// Read from env var `COOKIE_SECURE` (accepted values: "true"/"false", "1"/"0", "yes"/"no").
    pub cookie_secure: Option<bool>,
    /// Preferred SameSite value for cookies. Read from env var `COOKIE_SAMESITE`
    /// (accepted values: "Lax", "Strict", "None").
    pub cookie_same_site: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TwitchConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Allowed requests per second (per IP) for auth endpoints (e.g. /api/auth/login)
    pub auth_per_second: u32,
    /// Burst size for auth endpoints
    pub auth_burst: u32,
    /// Allowed requests per second (per IP) for webhook endpoints (e.g. /webhooks/twitch)
    pub webhook_per_second: u32,
    /// Burst size for webhook endpoints
    pub webhook_burst: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        Ok(Config {
            server: ServerConfig {
                host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue("PORT".to_string()))?,
                frontend_url: env::var("FRONTEND_URL")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string()),
                webhook_url: env::var("WEBHOOK_URL")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string()),
                cookie_secure: match env::var("COOKIE_SECURE") {
                    Ok(v) => match v.to_lowercase().as_str() {
                        "1" | "true" | "yes" => Some(true),
                        "0" | "false" | "no" => Some(false),
                        _ => None,
                    },
                    Err(_) => None,
                },
                cookie_same_site: env::var("COOKIE_SAMESITE").ok(),
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "sqlite://data/app.db".to_string()),
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
            },
            twitch: TwitchConfig {
                client_id: env::var("TWITCH_CLIENT_ID")
                    .map_err(|_| ConfigError::MissingEnv("TWITCH_CLIENT_ID".to_string()))?,
                client_secret: env::var("TWITCH_CLIENT_SECRET")
                    .map_err(|_| ConfigError::MissingEnv("TWITCH_CLIENT_SECRET".to_string()))?,
                redirect_uri: env::var("TWITCH_REDIRECT_URI")
                    .unwrap_or_else(|_| "http://localhost:3000/auth/callback".to_string()),
            },
            telegram: TelegramConfig {
                bot_token: env::var("TELEGRAM_BOT_TOKEN").ok(),
            },
            discord: DiscordConfig {
                bot_token: env::var("DISCORD_BOT_TOKEN").ok(),
                client_id: env::var("DISCORD_CLIENT_ID").ok(),
                client_secret: env::var("DISCORD_CLIENT_SECRET").ok(),
            },
            jwt: JwtConfig {
                secret: env::var("JWT_SECRET")
                    .map_err(|_| ConfigError::MissingEnv("JWT_SECRET".to_string()))?,
                expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
            },
            rate_limit: RateLimitConfig {
                auth_per_second: env::var("RATE_LIMIT_AUTH_PER_SECOND")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                auth_burst: env::var("RATE_LIMIT_AUTH_BURST")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                webhook_per_second: env::var("RATE_LIMIT_WEBHOOKS_PER_SECOND")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                webhook_burst: env::var("RATE_LIMIT_WEBHOOKS_BURST")
                    .unwrap_or_else(|_| "50".to_string())
                    .parse()
                    .unwrap_or(50),
            },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnv(String),

    #[error("Invalid value for environment variable: {0}")]
    InvalidValue(String),
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                frontend_url: "http://localhost:3000".to_string(),
                webhook_url: "http://localhost:8080".to_string(),
                cookie_secure: None,
                cookie_same_site: None,
            },
            database: DatabaseConfig {
                url: "sqlite://data/app.db".to_string(),
                max_connections: 5,
            },
            twitch: TwitchConfig {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            },
            telegram: TelegramConfig { bot_token: None },
            discord: DiscordConfig {
                bot_token: None,
                client_id: None,
                client_secret: None,
            },
            jwt: JwtConfig {
                secret: String::new(),
                expiration_hours: 24,
            },
            rate_limit: RateLimitConfig {
                auth_per_second: 3,
                auth_burst: 10,
                webhook_per_second: 10,
                webhook_burst: 50,
            },
        }
    }
}
