use core_foundation::{dictionary::*, number::*, string::*};
use core_graphics::display::*;
use serde::Serialize;
use std::ffi::{c_void, CString};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::ptr;
use tracing::debug;

use crate::utils::{get_day_start_timestamp, get_log_file_dir_str};

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum EventType {
    CameToFront,
    ShutDown,
    StopMonitoring,
}

impl EventType {
    pub fn to_int(&self) -> i32 {
        match self {
            EventType::CameToFront => 0,
            EventType::ShutDown => 1,
            EventType::StopMonitoring => 2,
        }
    }
}

#[derive(Serialize)]
pub struct AppUsage {
    pub name: String,
    pub path: String,
    pub total_secs: u64,
    pub durations: Vec<(u64, u64)>,
}

/// Get app name from path by querying a map or parsing the path
fn get_app_name_from_path(path: &str) -> Option<String> {
    // TODO: query the app name from a map

    let path = Path::new(path);
    let components: Vec<&str> = path.iter().filter_map(|os_str| os_str.to_str()).collect();

    #[cfg(target_os = "macos")]
    {
        // macOS
        // Attention: components[0] is "/".
        if components.len() > 1 && components[1] == "Applications" {
            if components.len() > 2 {
                return Some(components[2].replace(".app", ""));
            } else {
                return None;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows
        if components.len() > 1
            && (components[1] == "Program Files" || components[1] == "Program Files (x86)")
        {
            if components.len() > 2 {
                return Some(String::from(components[2]));
            } else {
                return None;
            }
        }
    }

    // If not in common paths, return the file name
    path.file_name()
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.replace(".exe", ""))
}

/// Get all app usages from local log file
///
/// Each app usage contains the app name, path, total time in seconds, and durations
pub fn get_app_usages_from_log(
    start_timestamp: u64,
    end_timestamp: u64,
) -> io::Result<Vec<AppUsage>> {
    let mut current_app_name: Option<String> = None;
    let mut app_usages: Vec<AppUsage> = Vec::new();

    // Find all log files between start_time and end_time
    let mut temp_timestamp = start_timestamp;
    let mut log_files = Vec::new();
    while temp_timestamp <= get_day_start_timestamp(end_timestamp) + 86399000 {
        match chrono::DateTime::from_timestamp_millis(temp_timestamp as i64) {
            Some(date) => {
                let log_file_name =
                    format!("{}/{}.log", get_log_file_dir_str(), date.format("%Y%m%d"));
                log_files.push(log_file_name);
            }
            _ => continue,
        }
        temp_timestamp += 86400000;
    }

    debug!(
        "StartTime: {}, EndTime: {}\nLogFiles: {:?}",
        chrono::DateTime::from_timestamp_millis(start_timestamp as i64)
            .map(|date| date.format("%Y%m%d").to_string())
            .unwrap_or(String::from("None")),
        chrono::DateTime::from_timestamp_millis(start_timestamp as i64)
            .map(|date| date.format("%Y%m%d").to_string())
            .unwrap_or(String::from("None")),
        log_files
    );

    for log_file_name in log_files {
        let file = File::open(log_file_name)?;
        let reader = io::BufReader::new(file);

        // Read each line of the log file, and get all durations for each app
        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split(',').collect();

            let event_type: &str = parts[0];
            let timestamp: u64 = match parts[1].parse() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Skip events before start_time or after end_time
            if timestamp < start_timestamp || timestamp > end_timestamp {
                continue;
            }

            // When switching apps
            if event_type == "0" {
                let app_path = parts[2];
                let app_name = get_app_name_from_path(app_path).unwrap_or(String::from("Unknown"));

                // Set end time for previous app
                if let Some(prev_app_name) = &current_app_name {
                    app_usages
                        .iter_mut()
                        .find(|app| &app.name == prev_app_name)
                        .map(|app| {
                            app.durations
                                .last_mut()
                                .map(|(_, _end_time)| *_end_time = timestamp)
                        });
                }
                // Set start time for current app
                current_app_name = Some(app_name.clone());
                if let Some(app) = app_usages.iter_mut().find(|app| &app.name == &app_name) {
                    app.durations.push((timestamp, timestamp));
                } else {
                    app_usages.push(AppUsage {
                        name: app_name,
                        path: app_path.to_string(),
                        total_secs: 0,
                        durations: vec![(timestamp, timestamp)],
                    });
                }
            }
            // When shutting down or not in use
            else if event_type == "1" || event_type == "2" {
                // Set end time for current app
                if let Some(prev_app_name) = &current_app_name {
                    app_usages
                        .iter_mut()
                        .find(|app| &app.name == prev_app_name)
                        .map(|app| {
                            app.durations
                                .last_mut()
                                .map(|(_, _end_time)| *_end_time = timestamp)
                        });
                }
                current_app_name = None;
            }
        }
    }

    // Calculate total time for each app
    for app in app_usages.iter_mut() {
        let mut total_microsecs: u64 = 0;
        for (_start_time, _end_time) in app.durations.iter() {
            total_microsecs += _end_time - _start_time;
        }
        app.total_secs = total_microsecs / 1000;
    }

    // Sort by total time
    app_usages.sort_by(|a, b| b.total_secs.cmp(&a.total_secs));

    Ok(app_usages)
}

pub fn get_window_property<T: FromCFType>(
    dic_ref: CFDictionaryRef,
    key: &str,
) -> Result<T, &'static str> {
    let c_key = CString::new(key).map_err(|_| "Failed to create CString")?;
    let cf_key =
        unsafe { CFStringCreateWithCString(ptr::null(), c_key.as_ptr(), kCFStringEncodingUTF8) };
    if cf_key.is_null() {
        return Err("Failed to create CFString");
    }

    let mut value: *const c_void = ptr::null();
    let found =
        unsafe { CFDictionaryGetValueIfPresent(dic_ref, cf_key as *const _, &mut value) != 0 };
    unsafe { CFRelease(cf_key as *const _) };

    if found {
        T::from_cf_type(value).ok_or("Failed to convert CFType")
    } else {
        Err("Property not found")
    }
}

pub trait FromCFType: Sized {
    fn from_cf_type(cf_type: *const c_void) -> Option<Self>;
}

impl FromCFType for i32 {
    fn from_cf_type(cf_type: *const c_void) -> Option<Self> {
        let number_ref = cf_type as CFNumberRef;
        let mut value: i32 = 0;
        let success = unsafe {
            CFNumberGetValue(
                number_ref,
                kCFNumberSInt32Type,
                &mut value as *mut _ as *mut c_void,
            )
        };
        if success {
            Some(value)
        } else {
            None
        }
    }
}

/// Get the PID of the frontmost window
///
/// This function is only available on macOS
pub fn get_frontmost_window_pid() -> Result<i32, &'static str> {
    #[cfg(target_os = "macos")]
    {
        const OPTIONS: CGWindowListOption =
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
        let window_list_info = unsafe { CGWindowListCopyWindowInfo(OPTIONS, kCGNullWindowID) };
        if window_list_info.is_null() {
            return Err("Failed to copy window list info");
        }

        let count = unsafe { CFArrayGetCount(window_list_info) };
        if count == 0 {
            unsafe { CFRelease(window_list_info as *const _) };
            return Err("No windows found");
        }

        let mut front_window_pid: Option<i32> = None;
        let mut last_layer: i32 = 0;

        for i in 0..count {
            let dic_ref = unsafe { CFArrayGetValueAtIndex(window_list_info, i) as CFDictionaryRef };
            if dic_ref.is_null() {
                continue;
            }

            if let Ok(layer) = get_window_property(dic_ref, "kCGWindowLayer") {
                if layer == 0 && last_layer != 0 {
                    if let Ok(pid) = get_window_property(dic_ref, "kCGWindowOwnerPID") {
                        front_window_pid = Some(pid);
                        break;
                    }
                }
                last_layer = layer;
            }
        }

        unsafe { CFRelease(window_list_info as *const _) };

        front_window_pid.ok_or("Failed to get frontmost window PID")
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("This function is only available on macOS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_app_name_from_path() {
        assert_eq!(
            get_app_name_from_path("/Applications/MyApp.app"),
            Some("MyApp".to_string())
        );
        assert_eq!(get_app_name_from_path("/Applications/"), None);
        assert_eq!(
            get_app_name_from_path("/Users/username/Downloads/MyApp"),
            Some("MyApp".to_string())
        );
    }
}
