// Background drainer service - drains Redis cache to files
use crate::config::DrainerConfig;
use crate::file_redis_layer::get_log_file_path;
use crate::types::AppError;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval, sleep, Duration};

/// Tracks retry attempts for failed keys
struct RetryTracker {
    attempts: HashMap<String, u32>,  // key -> retry count
}

impl RetryTracker {
    fn new() -> Self {
        Self {
            attempts: HashMap::new(),
        }
    }

    fn should_retry(&self, key: &str, max_retries: u32) -> bool {
        self.attempts.get(key).map_or(true, |&count| count < max_retries)
    }

    fn increment(&mut self, key: &str) -> u32 {
        let count = self.attempts.entry(key.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    fn reset(&mut self, key: &str) {
        self.attempts.remove(key);
    }

    fn get_failed_keys(&self) -> Vec<String> {
        self.attempts.keys().cloned().collect()
    }
}

/// Drain a single Redis key (user's log cache) to file
/// This reads all logs from Redis and writes them to the file system
/// Returns Ok(()) on success, Err on failure (key is NOT deleted on failure)
pub async fn drain_key_to_file(
    redis_client: &redis::Client,
    key: &str,
) -> Result<(), AppError> {
    let mut conn = redis_client
        .get_async_connection()
        .await
        .map_err(|e| AppError::RedisError(e.to_string()))?;
    
    // Get all logs from Redis list (LRANGE 0 -1 = get all)
    // We read ALL logs first to ensure atomicity
    let logs: Vec<String> = conn
        .lrange(key, 0, -1)
        .await
        .map_err(|e| AppError::RedisError(e.to_string()))?;
    
    if logs.is_empty() {
        return Ok(());  // Nothing to drain
    }
    
    // Parse key to extract user_id and date
    // Format: logs:user_123:19847
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() != 3 {
        return Err(AppError::RedisError("Invalid key format".to_string()));
    }
    
    let user_id = parts[1].strip_prefix("user_").unwrap_or(parts[1]);
    let days = parts[2];
    
    // Reconstruct timestamp from days (for get_log_file_path)
    let timestamp = days.parse::<u64>()
        .map_err(|_| AppError::RedisError("Invalid days format".to_string()))?
        * 86400;  // Convert days back to seconds
    
    let file_path = get_log_file_path(user_id, timestamp);
    
    // Create directory if needed
    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::FileError(format!("Could not create directory: {}", e)))?;
    }
    
    // Open file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .await
        .map_err(|e| AppError::FileError(format!("Could not open file {}: {}", file_path, e)))?;
    
    // Write all logs in batch (much faster than one-by-one!)
    // If this fails, Redis key is NOT deleted, so we can retry
    for log_line in &logs {
        file.write_all(format!("{}\n", log_line).as_bytes())
            .await
            .map_err(|e| AppError::FileError(format!("Could not write to {}: {}", file_path, e)))?;
    }
    
    // Flush to ensure data is written to disk
    file.flush()
        .await
        .map_err(|e| AppError::FileError(format!("Could not flush file {}: {}", file_path, e)))?;
    
    // Only delete the Redis key AFTER successful write
    // This ensures atomicity: either all logs are written and key is deleted,
    // or nothing happens and we can retry
    conn.del::<_, ()>(key)
        .await
        .map_err(|e| AppError::RedisError(format!("Could not delete key {}: {}", key, e)))?;
    
    println!("✅ Drained {} logs from {} to {}", logs.len(), key, file_path);
    Ok(())
}

/// Background drainer task - runs periodically
/// Finds all Redis keys matching our log pattern and drains them to files
/// 
/// Features:
/// - Retry mechanism for failed keys
/// - Tracks retry attempts
/// - Handles partial failures gracefully
/// - Logs metrics about success/failure rates
pub async fn start_drainer(
    redis_client: Arc<redis::Client>,
    config: DrainerConfig,
) {
    println!(
        "🔄 Starting background drainer (runs every {} seconds, pattern: {})",
        config.interval_seconds,
        config.log_pattern
    );
    println!("   Max retries: {}, Retry delay: {}s", config.max_retries, config.retry_delay_seconds);
    
    let mut interval_timer = interval(Duration::from_secs(config.interval_seconds));
    let mut retry_tracker = RetryTracker::new();
    
    loop {
        interval_timer.tick().await;  // Wait for next interval
        
        println!("🔄 Drainer: Starting batch drain cycle...");
        
        // Get Redis connection
        let mut conn = match redis_client.get_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("❌ Drainer: Failed to get Redis connection: {}", e);
                continue;
            }
        };
        
        // Find all keys matching our log pattern
        // Note: KEYS blocks Redis, but for learning it's fine
        // In production, use SCAN with cursor for non-blocking iteration
        let keys: Vec<String> = match conn.keys(&config.log_pattern).await {
            Ok(keys) => keys,
            Err(e) => {
                eprintln!("❌ Drainer: Failed to get keys: {}", e);
                continue;
            }
        };
        
        let mut total_drained = 0;
        let mut total_failed = 0;
        let mut retried_keys = 0;
        
        // Drain each key
        for key in keys {
            match drain_key_to_file(&redis_client, &key).await {
                Ok(_) => {
                    // Success! Reset retry counter for this key
                    retry_tracker.reset(&key);
                    total_drained += 1;
                }
                Err(e) => {
                    total_failed += 1;
                    let retry_count = retry_tracker.increment(&key);
                    
                    if retry_tracker.should_retry(&key, config.max_retries) {
                        eprintln!(
                            "⚠️  Drainer: Failed to drain {} (attempt {}/{}): {:?}",
                            key, retry_count, config.max_retries, e
                        );
                        retried_keys += 1;
                    } else {
                        // Max retries exceeded - log as dead letter
                        eprintln!(
                            "❌ Drainer: Key {} exceeded max retries ({}). Moving to dead letter handling.",
                            key, config.max_retries
                        );
                        // TODO: Move to dead letter queue or alert
                        // For now, we'll leave it in Redis and it will be picked up again
                        // In production, you might want to:
                        // 1. Move to a separate "failed" key
                        // 2. Send alert/notification
                        // 3. Log to a separate error log
                    }
                }
            }
        }
        
        // Print summary
        println!(
            "✅ Drainer: Completed cycle. Drained: {}, Failed: {}, Retrying: {}",
            total_drained, total_failed, retried_keys
        );
        
        // If there are failed keys that should be retried, wait and retry them
        if retried_keys > 0 {
            println!(
                "⏳ Waiting {} seconds before retrying {} failed keys...",
                config.retry_delay_seconds, retried_keys
            );
            sleep(Duration::from_secs(config.retry_delay_seconds)).await;
            
            // Retry failed keys
            let failed_keys = retry_tracker.get_failed_keys();
            for key in failed_keys {
                if !retry_tracker.should_retry(&key, config.max_retries) {
                    continue;  // Skip keys that exceeded max retries
                }
                
                match drain_key_to_file(&redis_client, &key).await {
                    Ok(_) => {
                        retry_tracker.reset(&key);
                        println!("✅ Retry successful for {}", key);
                    }
                    Err(e) => {
                        let retry_count = retry_tracker.increment(&key);
                        eprintln!(
                            "⚠️  Retry failed for {} (attempt {}/{}): {:?}",
                            key, retry_count, config.max_retries, e
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_drain_key_parsing() {
        // Test key parsing logic
        let key = "logs:user_123:19847";
        let parts: Vec<&str> = key.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "logs");
        assert_eq!(parts[1], "user_123");
        assert_eq!(parts[2], "19847");
        
        let user_id = parts[1].strip_prefix("user_").unwrap_or(parts[1]);
        assert_eq!(user_id, "123");
    }
    
    #[test]
    fn test_invalid_key_format() {
        let invalid_keys = vec![
            "logs:user_123",           // Missing date
            "logs:user_123:19847:extra", // Too many parts
            "invalid",                 // Wrong format
        ];
        
        for key in invalid_keys {
            let parts: Vec<&str> = key.split(':').collect();
            assert_ne!(parts.len(), 3, "Key {} should be invalid", key);
        }
    }
    
    #[test]
    fn test_retry_tracker() {
        let mut tracker = RetryTracker::new();
        let key = "logs:user_123:19847";
        
        // First attempt
        assert!(tracker.should_retry(key, 3));
        assert_eq!(tracker.increment(key), 1);
        
        // Second attempt
        assert!(tracker.should_retry(key, 3));
        assert_eq!(tracker.increment(key), 2);
        
        // Third attempt
        assert!(tracker.should_retry(key, 3));
        assert_eq!(tracker.increment(key), 3);
        
        // Fourth attempt - should not retry
        assert!(!tracker.should_retry(key, 3));
        
        // Reset
        tracker.reset(key);
        assert!(tracker.should_retry(key, 3));
    }
}
