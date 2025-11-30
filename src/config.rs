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
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub key_expiration_seconds: u64,  // How long keys stay in Redis before expiring
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
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            redis: RedisConfig {
                url: "redis://127.0.0.1:6379".to_string(),
                key_expiration_seconds: 600,  // 10 minutes
            },
            drainer: DrainerConfig {
                interval_seconds: 60,  // 1 minute (for testing)
                log_pattern: "logs:user_*:*".to_string(),
                max_retries: 3,        // Retry failed keys 3 times
                retry_delay_seconds: 30, // Wait 30 seconds between retries
            },
        }
    }
    
}

