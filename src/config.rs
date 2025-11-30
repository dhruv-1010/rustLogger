// Configuration module - loads settings from config.toml
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub drainer: DrainerConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub rate_limit: RateLimitConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,  // Max requests per minute per IP
    pub burst_size: u32,           // Burst allowance
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub key_expiration_seconds: Option<u64>,  // How long keys stay in Redis before expiring (None = disabled)
    pub disable_ttl: bool,  // If true, don't set TTL (rely on drainer DELETE only)
}

/// Drainer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrainerConfig {
    pub interval_seconds: u64,  // How often to drain Redis to files
    pub log_pattern: String,     // Redis key pattern to match (e.g., "logs:user_*:*")
    pub max_retries: u32,        // Maximum retries for a failed key before giving up
    pub retry_delay_seconds: u64, // Delay between retries for failed keys
}

impl Config {
    /// Load configuration from config.toml file
    /// Falls back to defaults if file doesn't exist
    pub fn load() -> Self {
        let config_path = "config.toml";
        
        if Path::new(config_path).exists() {
            match fs::read_to_string(config_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(config) => {
                            println!("✅ Loaded configuration from {}", config_path);
                            return config;
                        }
                        Err(e) => {
                            eprintln!("⚠️  Failed to parse config.toml: {}. Using defaults.", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to read config.toml: {}. Using defaults.", e);
                }
            }
        } else {
            println!("ℹ️  config.toml not found. Using default configuration.");
        }
        
        // Return default configuration
        Self::default()
    }
    
    /// Get default configuration
    pub fn default() -> Self {
        let config = Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 100,  // 100 requests per minute
                    burst_size: 20,              // Allow 20 burst requests
                },
            },
            redis: RedisConfig {
                url: "redis://127.0.0.1:6379".to_string(),
                // TTL as safety net: Very long TTL (24 hours) acts as backup cleanup
                // Drainer handles normal cleanup (every 30s), TTL catches edge cases
                key_expiration_seconds: Some(86400),  // 24 hours - safety net
                disable_ttl: false,  // Keep TTL enabled as safety net
            },
            drainer: DrainerConfig {
                interval_seconds: 30,  // 30 seconds (more frequent to prevent data loss)
                log_pattern: "logs:user_*:*".to_string(),
                max_retries: 3,        // Retry failed keys 3 times
                retry_delay_seconds: 30, // Wait 30 seconds between retries
            },
        };
        
        // Validate TTL vs drainer interval
        config.validate_ttl_safety();
        
        config
    }
    
    /// Validate that TTL is safe (longer than drainer interval + buffer)
    /// This prevents data loss if drainer is slow or fails
    fn validate_ttl_safety(&self) {
        let drainer_interval = self.drainer.interval_seconds;
        
        if self.redis.disable_ttl {
            println!("⚠️  WARNING: TTL disabled - relying on drainer DELETE only");
            println!("   If drainer fails, keys will accumulate in Redis forever!");
            println!("   This can cause Redis memory issues. Consider enabling TTL as safety net.");
            println!("   Drainer runs every {} seconds", drainer_interval);
            return;
        }
        
        if let Some(ttl) = self.redis.key_expiration_seconds {
            // TTL should be MUCH longer than drainer interval
            // TTL acts as safety net, drainer handles normal cleanup
            // Recommended: TTL should be 100x+ drainer interval (e.g., 30s drainer → 3000s+ TTL)
            // This ensures drainer always gets multiple chances before TTL expires
            let recommended_min_ttl = drainer_interval * 100;  // 100x for safety
            let absolute_min_ttl = drainer_interval * 5;  // Absolute minimum
            
            if ttl < absolute_min_ttl {
                eprintln!(
                    "❌ ERROR: TTL ({}) is dangerously short for drainer interval ({}). Minimum: {}",
                    ttl, drainer_interval, absolute_min_ttl
                );
                eprintln!("   This WILL cause data loss if drainer is slow!");
                panic!("Unsafe TTL configuration detected! Fix config.toml before continuing.");
            } else if ttl < recommended_min_ttl {
                eprintln!(
                    "⚠️  WARNING: TTL ({}) is shorter than recommended ({}). Recommended: {}",
                    ttl, recommended_min_ttl, recommended_min_ttl
                );
                eprintln!("   TTL should be much longer than drainer interval to act as safety net.");
                eprintln!("   Current: TTL = {}x drainer interval", ttl / drainer_interval);
                eprintln!("   Recommended: TTL >= {}x drainer interval (e.g., {} seconds)", 
                    100, recommended_min_ttl);
            } else {
                println!(
                    "✅ TTL safety check passed: TTL ({}) = {}x drainer interval ({})",
                    ttl, ttl / drainer_interval, drainer_interval
                );
                println!("   TTL acts as safety net, drainer handles normal cleanup");
            }
        } else {
            println!("⚠️  WARNING: TTL not set - relying on drainer DELETE only");
            println!("   If drainer fails, keys will accumulate in Redis!");
        }
    }
}

