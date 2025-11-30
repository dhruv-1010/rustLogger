// Main server - API handlers and entry point
mod types;
mod file_redis_layer;
mod drainer;
mod config;
mod rate_limit;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    middleware,
    routing::post,
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;

// Import our modules
use types::{AppError, AppState, LogEvent};
use file_redis_layer::write_to_cache;
use config::Config;
use rate_limit::rate_limit_middleware;

// ============================================
// API HANDLERS
// ============================================

/// Handler that writes to Redis cache (FAST!)
/// This is the main endpoint for ingesting logs
async fn handle_log(
    State(state): State<AppState>,
    Json(payload): Json<LogEvent>,
) -> Result<StatusCode, AppError> {
    // Write to Redis cache - this is instant!
    write_to_cache(
        &state.redis_client,
        &payload,
        state.config.redis.key_expiration_seconds,
        state.config.redis.disable_ttl,
    )
    .await?;
    
    Ok(StatusCode::OK)
}

// ============================================
// MAIN
// ============================================

#[tokio::main]
async fn main() {
    // Load configuration
    let config = Config::load();
    
    // Connect to Redis
    let redis_client = redis::Client::open(config.redis.url.as_str())
        .expect("Failed to connect to Redis");
    
    // Test connection
    let _test_conn = redis_client
        .get_async_connection()
        .await
        .expect("Failed to get Redis connection");
    
    println!("✅ Connected to Redis at {}", config.redis.url);
    
    // Create rate limiter
    let rate_limiter = Arc::new(rate_limit::RateLimiter::new(
        config.server.rate_limit.clone(),
    ));
    
    // Create app state
    let state = AppState {
        redis_client: Arc::new(redis_client.clone()),
        config: config.clone(),
        rate_limiter: rate_limiter.clone(),
    };
    
    // Note: Drainer is now a separate service
    // Run it with: cargo run --bin drainer
    // This allows the drainer to be scaled independently
    
    // Create router with rate limiting middleware
    let app = Router::new()
        .route("/log", post(handle_log))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .with_state(state);  // Share state with handlers
    
    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("🚀 Server running on http://{}", addr);
    println!("📝 POST /log - Writes to Redis cache (fast!)");
    println!("\n💡 Architecture:");
    println!("   1. Write → Redis (instant, in-memory)");
    println!("   2. Separate drainer service → Files (run with: cargo run --bin drainer)");
    if let Some(ttl) = config.redis.key_expiration_seconds {
        println!("   3. Redis auto-expires keys after {} seconds (safety net)", ttl);
    } else {
        println!("   3. Redis TTL disabled - drainer handles all cleanup");
    }
    println!("\n📁 Module Structure:");
    println!("   • types.rs - Shared types (LogEvent, AppError, AppState)");
    println!("   • file_redis_layer.rs - Redis cache operations");
    println!("   • drainer.rs - Background drainer service");
    println!("   • config.rs - Configuration management");
    println!("\n🔄 To start the drainer service:");
    println!("   cargo run --bin drainer");
    println!("   • main.rs - API server");
    
    axum::serve(listener, app).await.unwrap();
}
