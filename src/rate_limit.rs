// Rate limiting middleware for the log endpoint
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

use crate::config::RateLimitConfig;
use crate::types::AppState;

/// Rate limiter using token bucket algorithm
#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    // Simple in-memory rate limiter (per IP would need a HashMap)
    // For production, use Redis-based rate limiting
    tokens: Arc<Mutex<TokenBucket>>,
}

struct TokenBucket {
    tokens: u32,
    last_refill: SystemTime,
    requests_per_minute: u32,
    burst_size: u32,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config: config.clone(),
            tokens: Arc::new(Mutex::new(TokenBucket {
                tokens: config.burst_size,
                last_refill: SystemTime::now(),
                requests_per_minute: config.requests_per_minute,
                burst_size: config.burst_size,
            })),
        }
    }

    /// Check if request is allowed (token bucket algorithm)
    pub async fn check(&self) -> bool {
        let mut bucket = self.tokens.lock().await;
        let now = SystemTime::now();
        
        // Calculate time since last refill
        let elapsed = now
            .duration_since(bucket.last_refill)
            .unwrap_or(Duration::from_secs(0));
        
        // Refill tokens based on elapsed time
        // Refill rate: requests_per_minute / 60 seconds
        if elapsed.as_secs() > 0 {
            let tokens_to_add = (bucket.requests_per_minute as u64 * elapsed.as_secs()) / 60;
            bucket.tokens = (bucket.tokens + tokens_to_add as u32).min(bucket.burst_size);
            bucket.last_refill = now;
        }
        
        // Check if we have tokens
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    // Extract IP from headers (for per-IP rate limiting)
    let _ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");
    
    // Use rate limiter from state
    if !state.rate_limiter.check().await {
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("x-ratelimit-limit", state.config.server.rate_limit.requests_per_minute.to_string())
            .header("retry-after", "60")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({
                    "error": "Rate limit exceeded",
                    "details": format!("Maximum {} requests per minute", state.config.server.rate_limit.requests_per_minute)
                }).to_string()
            ))
            .unwrap()
            .into();
    }
    
    next.run(request).await
}
