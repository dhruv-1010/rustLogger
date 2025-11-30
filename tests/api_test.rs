// API integration tests
// Note: These tests require Redis to be running
// Run with: cargo test --test api_test -- --ignored

use log_pipelines::types::{AppState, LogEvent};
use log_pipelines::file_redis_layer::write_to_cache;
use log_pipelines::config::Config;
use std::sync::Arc;

#[tokio::test]
#[ignore]  // Ignore by default - requires Redis
async fn test_api_log_endpoint() {
    // This test requires Redis to be running
    // You can run it with: cargo test --test api_test -- --ignored
    
    let config = Config::default();
    let redis_client = redis::Client::open(config.redis.url.as_str())
        .expect("Failed to connect to Redis");
    
    let rate_limiter = Arc::new(log_pipelines::rate_limit::RateLimiter::new(
        config.server.rate_limit.clone(),
    ));
    
    let state = AppState {
        redis_client: Arc::new(redis_client),
        config: config.clone(),
        rate_limiter,
    };
    
    // Create test event
    let event = LogEvent {
        user_id: "test_user".to_string(),
        event: "test_event".to_string(),
        timestamp: 1712345678,
    };
    
    // Test writing to cache
    let result = write_to_cache(
        &state.redis_client,
        &event,
        config.redis.key_expiration_seconds,
        config.redis.disable_ttl,
    )
    .await;
    
    assert!(result.is_ok(), "Should successfully write to Redis");
}

#[tokio::test]
#[ignore]
async fn test_redis_connection() {
    let config = Config::default();
    let redis_client = redis::Client::open(config.redis.url.as_str())
        .expect("Failed to connect to Redis");
    
    let mut conn = redis_client
        .get_async_connection()
        .await
        .expect("Failed to get connection");
    
    // Test basic Redis operations
    use redis::AsyncCommands;
    
    let test_key = "test:connection";
    let _: () = conn.set(test_key, "test_value").await.unwrap();
    let value: String = conn.get(test_key).await.unwrap();
    assert_eq!(value, "test_value");
    
    // Cleanup
    let _: () = conn.del(test_key).await.unwrap();
}

