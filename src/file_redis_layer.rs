// Redis cache layer - handles all Redis operations
use crate::types::{AppError, LogEvent};
use redis::AsyncCommands;

/// Get Redis key for a user's log cache
/// Format: logs:user_{user_id}:{days_since_epoch}
pub fn get_redis_key(user_id: &str, timestamp: u64) -> String {
    let days_since_epoch = timestamp / 86400;  // 86400 seconds in a day
    format!("logs:user_{}:{}", user_id, days_since_epoch)
}

/// Get file path for a user's log file
/// Format: logs/user_{user_id}/{days_since_epoch}.jsonl
pub fn get_log_file_path(user_id: &str, timestamp: u64) -> String {
    let days_since_epoch = timestamp / 86400;
    format!("logs/user_{}/{}.jsonl", user_id, days_since_epoch)
}

/// Write log event to Redis cache
/// This is FAST - Redis is in-memory, so writes are instant
pub async fn write_to_cache(
    redis_client: &redis::Client,
    event: &LogEvent,
    expiration_seconds: Option<u64>,
    disable_ttl: bool,
) -> Result<(), AppError> {
    // Get Redis connection from pool
    let mut conn = redis_client
        .get_async_connection()
        .await
        .map_err(|e| AppError::RedisError(e.to_string()))?;
    
    // Get the Redis key for this user's log cache
    let key = get_redis_key(&event.user_id, event.timestamp);
    
    // Serialize event to JSON
    let json_line = serde_json::to_string(event)
        .map_err(|e| AppError::SerializationError(e.to_string()))?;
    
    // Append to Redis list (RPUSH = append to end of list)
    // Redis lists are perfect for log streams!
    conn.rpush::<_, _, ()>(&key, &json_line)
        .await
        .map_err(|e| AppError::RedisError(e.to_string()))?;
    
    // Set expiration on the key (only if TTL is enabled)
    if !disable_ttl {
        if let Some(ttl) = expiration_seconds {
            conn.expire::<_, ()>(&key, ttl as i64)
                .await
                .map_err(|e| AppError::RedisError(e.to_string()))?;
        }
    }
    // If disable_ttl is true, we rely on drainer DELETE only (safest)
    
    Ok(())
}

/// Read logs from Redis cache for a specific user
/// Returns all logs in the cache for that user's key
#[allow(dead_code)]  // Reserved for future use (e.g., stats endpoint)
pub async fn read_from_cache(
    redis_client: &redis::Client,
    user_id: &str,
    timestamp: u64,
) -> Result<Vec<String>, AppError> {
    let mut conn = redis_client
        .get_async_connection()
        .await
        .map_err(|e| AppError::RedisError(e.to_string()))?;
    
    let key = get_redis_key(user_id, timestamp);
    
    // Get all logs from Redis list (LRANGE 0 -1 = get all)
    let logs: Vec<String> = conn
        .lrange(&key, 0, -1)
        .await
        .map_err(|e| AppError::RedisError(e.to_string()))?;
    
    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_redis_key() {
        let user_id = "123";
        let timestamp = 1712345678;  // Some timestamp
        let days = timestamp / 86400;
        
        let key = get_redis_key(user_id, timestamp);
        assert_eq!(key, format!("logs:user_{}:{}", user_id, days));
    }
    
    #[test]
    fn test_get_log_file_path() {
        let user_id = "456";
        let timestamp = 1712345678;
        let days = timestamp / 86400;
        
        let path = get_log_file_path(user_id, timestamp);
        assert_eq!(path, format!("logs/user_{}/{}.jsonl", user_id, days));
    }
    
    #[test]
    fn test_key_and_path_consistency() {
        // Key and path should use the same day calculation
        let user_id = "789";
        let timestamp = 1712345678;
        
        let key = get_redis_key(user_id, timestamp);
        let path = get_log_file_path(user_id, timestamp);
        
        // Extract days from both
        let key_days: Vec<&str> = key.split(':').collect();
        let path_days: Vec<&str> = path.split('/').collect();
        
        assert_eq!(key_days[2], path_days[2].strip_suffix(".jsonl").unwrap());
    }
}
