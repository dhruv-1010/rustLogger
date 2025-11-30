// Shared types used across the application
use axum::{
    http::StatusCode,
    response::Json as ResponseJson,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Log event structure
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LogEvent {
    pub user_id: String,
    pub event: String,
    pub timestamp: u64,
}

// Custom error type
#[derive(Debug)]
pub enum AppError {
    #[allow(dead_code)]  // Reserved for future use
    JsonParseError(String),
    FileError(String),
    SerializationError(String),
    RedisError(String),
}

// Error response struct
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: String,
}

// Convert AppError to HTTP response
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_msg, details) = match self {
            AppError::JsonParseError(msg) => (
                StatusCode::BAD_REQUEST,
                "Invalid JSON format".to_string(),
                format!("Failed to parse JSON: {}", msg),
            ),
            AppError::FileError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "File operation failed".to_string(),
                format!("Could not write to file: {}", msg),
            ),
            AppError::SerializationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Serialization failed".to_string(),
                format!("Could not serialize data: {}", msg),
            ),
            AppError::RedisError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cache operation failed".to_string(),
                format!("Redis error: {}", msg),
            ),
        };

        let body = ResponseJson(ErrorResponse {
            error: error_msg,
            details,
        });
        (status, body).into_response()
    }
}

// Application state - shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub redis_client: Arc<redis::Client>,
    pub config: crate::config::Config,
}

