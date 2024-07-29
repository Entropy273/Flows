use chrono::Local;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::sys_monitor::EventType;

pub fn get_log_file_path() -> PathBuf {
    let home_dir = env::var("HOME").unwrap();
    let current_date = Local::now();
    let log_file_name = format!("{}.log", current_date.format("%Y%m%d"));
    let log_file_path_str = format!("{}/Documents/Flows/{}", home_dir, log_file_name);
    let mut path = PathBuf::from(&log_file_path_str);
    path.pop();
    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }
    PathBuf::from(&log_file_path_str)
}

pub fn get_log_file_dir_str() -> String {
    let home_dir = env::var("HOME").unwrap();
    format!("{}/Documents/Flows", home_dir)
}

pub fn write_to_file(event_type: EventType, timestamp: u64, path: &str) {
    let log_file_path = get_log_file_path();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path)
        .unwrap();
    if event_type == EventType::StopMonitoring {
        writeln!(file, "{},{}", event_type.to_int(), timestamp).unwrap();
    } else if event_type == EventType::CameToFront {
        writeln!(file, "{},{},{}", event_type.to_int(), timestamp, path).unwrap();
    }
}

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
