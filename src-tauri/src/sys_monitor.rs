use core_foundation::{number::*, string::*};
use core_graphics::display::*;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::fs::File;
use std::io::{self, BufRead};
use std::ptr;

#[derive(Debug)]
#[derive(PartialEq)]
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

fn get_app_name_from_path(path: &str) -> String {
    // If path starts with /Applications/, the app name is the next segment
    // If path starts with other paths, the app name is the last segment
    let segments: Vec<&str> = path.split('/').collect();
    if segments[1] == "Applications" {
        // Delete the .app extension
        segments[2].replace(".app", "")
    } else {
        segments.last().unwrap().to_string()
    }
}

pub fn get_app_usage_from_log(file_path: &str) -> io::Result<HashMap<String, Vec<(u64, u64)>>> {
    let mut current_app_name: Option<String> = None;
    let mut app_usage: HashMap<String, Vec<(u64, u64)>> = HashMap::new();

    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();

        let event_type: &str = parts[0];
        let timestamp: u64 = match parts[1].parse() {
            Ok(t) => t,
            Err(_) => continue,
        };
        
        // When switching apps
        if event_type == "0" {
            let app_path = parts[2];
            let app_name = get_app_name_from_path(app_path);

            // Set end time for previous app
            if let Some(prev_app_name) = &current_app_name {
                app_usage
                    .entry(prev_app_name.clone())
                    .or_insert(Vec::new())
                    .last_mut()
                    .map(|(_, end_time)| *end_time = timestamp);
            }
            // Set start time for current app
            current_app_name = Some(app_name.clone());
            app_usage
                .entry(app_name.clone())
                .or_insert(Vec::new())
                .push((timestamp, timestamp));
        }
        // When shutting down or not in use
        else if event_type == "1" || event_type == "2" {
            // Set end time for current app
            if let Some(prev_app_name) = &current_app_name {
                app_usage
                    .entry(prev_app_name.clone())
                    .or_insert(Vec::new())
                    .last_mut()
                    .map(|(_, end_time)| *end_time = timestamp);
            }
            current_app_name = None;
        }
    }

    Ok(app_usage)
}

pub fn get_window_property(
    dic_ref: CFDictionaryRef,
    key: &str,
) -> Result<*const c_void, &'static str> {
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
        Ok(value)
    } else {
        Err("Property not found")
    }
}

pub fn get_frontmost_window_pid() -> Result<i32, &'static str> {
    const OPTIONS: CGWindowListOption =
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let window_list_info = unsafe { CGWindowListCopyWindowInfo(OPTIONS, kCGNullWindowID) };
    if window_list_info.is_null() {
        return Err("Failed to copy window list info");
    }
    let count = unsafe { CFArrayGetCount(window_list_info) };

    for i in 0..count {
        let dic_ref = unsafe { CFArrayGetValueAtIndex(window_list_info, i) as CFDictionaryRef };

        if let Ok(layer_value) = get_window_property(dic_ref, "kCGWindowLayer") {
            let layer_number_ref = layer_value as CFNumberRef;
            let mut layer_number: i32 = 0;
            unsafe {
                CFNumberGetValue(
                    layer_number_ref,
                    kCFNumberSInt32Type,
                    &mut layer_number as *mut _ as *mut c_void,
                )
            };

            if layer_number == 0 {
                if let Ok(pid_value) = get_window_property(dic_ref, "kCGWindowOwnerPID") {
                    let pid_number_ref = pid_value as CFNumberRef;
                    let mut pid_number: i32 = 0;
                    unsafe {
                        CFNumberGetValue(
                            pid_number_ref,
                            kCFNumberSInt32Type,
                            &mut pid_number as *mut _ as *mut c_void,
                        )
                    };
                    unsafe { CFRelease(window_list_info as *const _) };
                    return Ok(pid_number);
                }
            }
        }
    }

    unsafe { CFRelease(window_list_info as *const _) };
    Err("No frontmost window found")
}
