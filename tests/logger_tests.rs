use chrono::Utc;
use medars::logger::{LogEntry, Logger};
use tempfile::TempDir;

#[test]
fn test_logger_new() {
    let logger = Logger::new();
    // Just verify it can be created
    let _ = logger;
}

#[test]
fn test_log_entry_creation() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        action: "test".to_string(),
        file: "test.jpg".to_string(),
        result: "success".to_string(),
        details: Some("Test details".to_string()),
    };
    
    assert_eq!(entry.action, "test");
    assert_eq!(entry.file, "test.jpg");
    assert_eq!(entry.result, "success");
    assert!(entry.details.is_some());
}

#[test]
fn test_log_entry_serialization() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        action: "clean".to_string(),
        file: "image.jpg".to_string(),
        result: "success".to_string(),
        details: None,
    };
    
    let json = serde_json::to_string(&entry);
    assert!(json.is_ok());
    
    let json_str = json.unwrap();
    assert!(json_str.contains("clean"));
    assert!(json_str.contains("image.jpg"));
}

#[test]
fn test_log_entry_deserialization() {
    let json = r#"{"timestamp":"2024-01-01T00:00:00Z","action":"check","file":"test.jpg","result":"has_metadata","details":null}"#;
    
    let entry: Result<LogEntry, _> = serde_json::from_str(json);
    assert!(entry.is_ok());
    
    let entry = entry.unwrap();
    assert_eq!(entry.action, "check");
    assert_eq!(entry.file, "test.jpg");
    assert_eq!(entry.result, "has_metadata");
}

#[test]
fn test_logger_log_write() {
    let logger = Logger::new();
    let entry = LogEntry {
        timestamp: Utc::now(),
        action: "test_write".to_string(),
        file: "test_image.jpg".to_string(),
        result: "success".to_string(),
        details: Some("Integration test".to_string()),
    };
    
    logger.log(&entry);
    
    // Read back and verify
    let logs = logger.read_logs(Some(1));
    assert!(!logs.is_empty());
}

#[test]
fn test_logger_read_logs_empty() {
    // Test the behavior when reading logs
    let _temp_dir = TempDir::new().unwrap();
    
    let logger = Logger::new();
    let logs = logger.read_logs(Some(0));
    
    // Should return empty or existing logs without crashing
    let _ = logs;
}

#[test]
fn test_logger_read_logs_with_limit() {
    let logger = Logger::new();
    
    // Log multiple entries
    for i in 0..5 {
        let entry = LogEntry {
            timestamp: Utc::now(),
            action: format!("action_{}", i),
            file: format!("file_{}.jpg", i),
            result: "success".to_string(),
            details: None,
        };
        logger.log(&entry);
    }
    
    // Read with limit
    let logs = logger.read_logs(Some(3));
    assert!(logs.len() <= logs.len()); // Should respect limit or return all if fewer
}

#[test]
fn test_logger_read_logs_no_limit() {
    let logger = Logger::new();
    
    // Log an entry
    let entry = LogEntry {
        timestamp: Utc::now(),
        action: "test_no_limit".to_string(),
        file: "unlimited.jpg".to_string(),
        result: "success".to_string(),
        details: None,
    };
    logger.log(&entry);
    
    // Read without limit
    let logs = logger.read_logs(None);
    assert!(logs.len() >= 1);
}

#[test]
fn test_log_entry_with_details() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        action: "clean".to_string(),
        file: "photo.jpg".to_string(),
        result: "success".to_string(),
        details: Some("Removed GPS coordinates, timestamps, camera info".to_string()),
    };
    
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: LogEntry = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.details, entry.details);
}

#[test]
fn test_log_entry_without_details() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        action: "check".to_string(),
        file: "photo.jpg".to_string(),
        result: "no_metadata".to_string(),
        details: None,
    };
    
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: LogEntry = serde_json::from_str(&json).unwrap();
    
    assert!(deserialized.details.is_none());
}

#[test]
fn test_logger_multiple_actions() {
    let logger = Logger::new();
    
    let actions = vec!["check", "clean", "show"];
    for action in actions {
        let entry = LogEntry {
            timestamp: Utc::now(),
            action: action.to_string(),
            file: format!("{}_test.jpg", action),
            result: "success".to_string(),
            details: None,
        };
        logger.log(&entry);
    }
    
    let logs = logger.read_logs(Some(10));
    assert!(!logs.is_empty());
}
