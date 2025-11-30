// Integration tests for the log pipeline service
use log_pipelines::types::LogEvent;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper function to create a test log event
fn create_test_log_event(user_id: &str, event: &str) -> LogEvent {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    LogEvent {
        user_id: user_id.to_string(),
        event: event.to_string(),
        timestamp,
    }
}

#[tokio::test]
async fn test_log_event_serialization() {
    let event = create_test_log_event("123", "clicked_button");
    
    // Test serialization
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("user_id"));
    assert!(json.contains("event"));
    assert!(json.contains("timestamp"));
    
    // Test deserialization
    let deserialized: LogEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.user_id, "123");
    assert_eq!(deserialized.event, "clicked_button");
}

#[tokio::test]
async fn test_log_event_validation() {
    // Test that invalid JSON is rejected
    let invalid_json = json!({
        "user__id": "123",  // Wrong field name
        "event": "clicked_button",
        "timestamp": 1712345678
    });
    
    let result: Result<LogEvent, _> = serde_json::from_value(invalid_json);
    assert!(result.is_err(), "Should reject invalid field names");
}

#[tokio::test]
async fn test_log_event_missing_fields() {
    // Test missing required fields
    let incomplete_json = json!({
        "user_id": "123"
        // Missing event and timestamp
    });
    
    let result: Result<LogEvent, _> = serde_json::from_value(incomplete_json);
    assert!(result.is_err(), "Should reject missing required fields");
}

#[tokio::test]
async fn test_log_event_types() {
    // Test that wrong types are rejected
    let wrong_type_json = json!({
        "user_id": "123",
        "event": "clicked_button",
        "timestamp": "not_a_number"  // Should be number
    });
    
    let result: Result<LogEvent, _> = serde_json::from_value(wrong_type_json);
    assert!(result.is_err(), "Should reject wrong types");
}

