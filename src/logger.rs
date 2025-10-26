use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub file: String,
    pub result: String,
    pub details: Option<String>,
}

pub struct Logger {
    log_path: PathBuf,
}

impl Logger {
    pub fn new() -> Self {
        let mut log_path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        log_path.push("medars");
        log_path.push("medars.log");
        Logger { log_path }
    }

    pub fn log(&self, entry: &LogEntry) {
        if let Some(parent) = self.log_path.parent() {
            let _ = create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            if let Ok(json) = serde_json::to_string(entry) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }

    pub fn read_logs(&self, max: Option<usize>) -> Vec<LogEntry> {
        let mut entries = Vec::new();
        if let Ok(file) = File::open(&self.log_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
                    entries.push(entry);
                }
            }
        }
        if let Some(max) = max {
            let len = entries.len();
            if len > max {
                entries.drain(0..len - max);
            }
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation_with_all_fields() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            action: "test_action".to_string(),
            file: "test_file.jpg".to_string(),
            result: "test_result".to_string(),
            details: Some("test_details".to_string()),
        };
        assert_eq!(entry.action, "test_action");
        assert!(entry.details.is_some());
    }

    #[test]
    fn test_log_entry_serialization_deserialization() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            action: "serialize_test".to_string(),
            file: "file.jpg".to_string(),
            result: "success".to_string(),
            details: None,
        };
        
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();
        
        assert_eq!(entry.action, deserialized.action);
        assert_eq!(entry.file, deserialized.file);
    }

    #[test]
    fn test_logger_path_creation() {
        let logger = Logger::new();
        assert!(logger.log_path.to_str().is_some());
    }
}
