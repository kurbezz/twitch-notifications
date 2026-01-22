use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication required")]
    Unauthorized,

    #[error("Access denied")]
    Forbidden,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Twitch API error: {0}")]
    TwitchApi(String),

    #[error("Telegram error: {0}")]
    Telegram(String),

    #[error("Discord error: {0}")]
    Discord(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("External service unavailable: {0}")]
    ServiceUnavailable(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                self.to_string(),
            ),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "A database error occurred".to_string(),
                )
            }
            AppError::Jwt(e) => {
                tracing::warn!("JWT error: {:?}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    "INVALID_TOKEN",
                    "Invalid or expired token".to_string(),
                )
            }
            AppError::Request(e) => {
                tracing::error!("HTTP request error: {:?}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    "EXTERNAL_REQUEST_FAILED",
                    "Failed to communicate with external service".to_string(),
                )
            }
            AppError::TwitchApi(msg) => {
                tracing::error!("Twitch API error: {}", msg);
                (StatusCode::BAD_GATEWAY, "TWITCH_API_ERROR", msg.clone())
            }
            AppError::Telegram(msg) => {
                tracing::error!("Telegram error: {}", msg);
                (StatusCode::BAD_GATEWAY, "TELEGRAM_ERROR", msg.clone())
            }
            AppError::Discord(msg) => {
                tracing::error!("Discord error: {}", msg);
                (StatusCode::BAD_GATEWAY, "DISCORD_ERROR", msg.clone())
            }
            AppError::Config(msg) => {
                tracing::error!("Configuration error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONFIG_ERROR",
                    "Server configuration error".to_string(),
                )
            }
            AppError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                msg.clone(),
            ),
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message,
                details: None,
            },
        };

        (status, Json(body)).into_response()
    }
}

impl AppError {
    pub fn with_details(self, details: serde_json::Value) -> AppErrorWithDetails {
        AppErrorWithDetails {
            error: self,
            details: Some(details),
        }
    }
}

pub struct AppErrorWithDetails {
    error: AppError,
    details: Option<serde_json::Value>,
}

impl IntoResponse for AppErrorWithDetails {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.error {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                self.error.to_string(),
            ),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", self.error.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                self.error.to_string(),
            ),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An internal error occurred".to_string(),
            ),
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message,
                details: self.details,
            },
        };

        (status, Json(body)).into_response()
    }
}

impl From<AppError> for AppErrorWithDetails {
    fn from(error: AppError) -> Self {
        AppErrorWithDetails {
            error,
            details: None,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
