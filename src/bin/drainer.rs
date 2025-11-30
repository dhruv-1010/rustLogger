// Separate drainer service - runs independently
// This can be deployed as a separate service/container
use log_pipelines::config::Config;
use log_pipelines::drainer::start_drainer;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("🔄 Starting Log Pipeline Drainer Service");
    println!("========================================\n");
    
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
    println!(
        "🔄 Drainer will run every {} seconds",
        config.drainer.interval_seconds
    );
    println!("🔍 Looking for keys matching: {}", config.drainer.log_pattern);
    println!("\nPress Ctrl+C to stop the drainer\n");
    
    // Start the drainer (this runs forever)
    let drainer_redis = Arc::new(redis_client);
    start_drainer(drainer_redis, config.drainer).await;
}

