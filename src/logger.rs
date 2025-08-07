use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use std::fs::{OpenOptions, create_dir_all, File};
use std::io::{BufReader, BufRead, Write};
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
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.log_path) {
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
                entries.drain(0..len-max);
            }
        }
        entries
    }
}
