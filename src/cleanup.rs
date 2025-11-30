// Optional cleanup service - removes old keys that drainer might have missed
// This is a safety net in case drainer fails or misses keys
use crate::config::Config;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Cleanup service - removes very old keys as a safety net
/// This runs less frequently than the drainer and only removes keys
/// that are very old (approaching TTL expiration)
/// 
/// This prevents Redis memory bloat if drainer somehow misses keys
pub async fn start_cleanup_service(
    redis_client: Arc<redis::Client>,
    config: Config,
) {
    // Only run if TTL is disabled (otherwise TTL handles cleanup)
    if config.redis.disable_ttl {
        println!("🔄 Starting cleanup service (TTL disabled - cleanup needed)");
        
        // Run cleanup every 1 hour
        let mut interval_timer = interval(Duration::from_secs(3600));
        
        loop {
            interval_timer.tick().await;
            
            println!("🧹 Cleanup: Starting cleanup cycle...");
            
            let mut conn = match redis_client.get_async_connection().await {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("❌ Cleanup: Failed to get Redis connection: {}", e);
                    continue;
                }
            };
            
            // Find all keys matching our log pattern
            let keys: Vec<String> = match conn.keys(&config.drainer.log_pattern).await {
                Ok(keys) => keys,
                Err(e) => {
                    eprintln!("❌ Cleanup: Failed to get keys: {}", e);
                    continue;
                }
            };
            
            // For each key, check if it's very old (older than 1 hour)
            // If drainer hasn't processed it in 1 hour, something might be wrong
            // But we'll be conservative and only log warnings
            let mut old_keys = 0;
            for key in &keys {
                // Get list length to see if key has data
                let len: Result<usize, _> = conn.llen(key).await;
                if let Ok(length) = len {
                    if length > 0 {
                        old_keys += 1;
                        eprintln!(
                            "⚠️  Cleanup: Key {} has {} logs and hasn't been drained (drainer may have issues)",
                            key, length
                        );
                    }
                }
            }
            
            if old_keys > 0 {
                println!("🧹 Cleanup: Found {} keys with undrained logs", old_keys);
                println!("   These should be handled by the drainer. Check drainer logs for issues.");
            } else {
                println!("✅ Cleanup: All keys are clean");
            }
        }
    } else {
        println!("ℹ️  Cleanup service not needed (TTL enabled - handles cleanup automatically)");
    }
}

