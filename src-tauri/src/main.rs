#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use libproc::libproc::proc_pid;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use tauri::{AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, WindowEvent};
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Registry};

mod app_management;
mod sys_monitor;
mod utils;

use app_management::{add_app_to_login_items, is_app_in_login_items, terminate_previous_instance};
use sys_monitor::{get_app_usage_from_log, get_frontmost_window_pid, EventType};
use utils::{get_current_timestamp, get_log_file_path, write_to_file};

#[tauri::command]
fn get_app_usages() -> HashMap<String, Vec<(u64, u64)>> {
    match get_app_usage_from_log(match get_log_file_path().to_str() {
        Some(path) => path,
        _ => panic!("Failed to get log file path"),
    }) {
        Ok(usage) => usage,
        Err(e) => {
            error!("Failed to get app usage: {}", e);
            std::collections::HashMap::new()
        }
    }
}

#[tauri::command]
fn show_window(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_window("main") {
        window.emit("refresh_data", "").unwrap();
        window.show().unwrap();
        window.set_focus().unwrap();
    } else {
        println!("Failed to get main window");
    }
}

fn init_tracing(data_path: &str) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(data_path, "app.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(std::io::stdout);

    let file_layer = fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_writer(file_writer);

    let subscriber = Registry::default().with(console_layer).with(file_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    guard
}

fn main() {
    let app_name = "Flows";
    let process_name = "flows";
    let app_path = format!("/Applications/{}.app", app_name);
    let data_path = format!("{}/Documents/{}", std::env::var("HOME").unwrap(), app_name);

    // Check if there has been a previous instance of the app running. If so, terminate it.
    terminate_previous_instance(process_name);

    // Initialize tracing
    let _guard = init_tracing(&(data_path + "/logs"));

    // Check if the app is in login items. If not, add it.
    if !is_app_in_login_items(&app_path) {
        info!("App is not in login items, adding...");
        add_app_to_login_items(&app_path);
    }

    // Start the system monitor thread
    let _ = thread::spawn(move || {
        let mut previous_path = String::new();
        loop {
            match get_frontmost_window_pid() {
                Ok(pid) => match proc_pid::pidpath(pid) {
                    Ok(current_path) => {
                        if current_path != previous_path {
                            info!("New program: {}", current_path);
                            let timestamp: u128 = get_current_timestamp();
                            write_to_file(EventType::CameToFront, timestamp, &current_path);
                            previous_path = current_path;
                        }
                    }
                    _ => error!("Failed to retrieve process path for PID {}", pid),
                },
                Err(e) => error!("{}", e),
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    // Create the system tray
    let dashboard = CustomMenuItem::new("dashboard".to_string(), "Dashboard");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(dashboard)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    let tray = SystemTray::new().with_menu(tray_menu);

    // Start the app
    tauri::Builder::default()
        .setup(|app| Ok(app.set_activation_policy(tauri::ActivationPolicy::Accessory)))
        .invoke_handler(tauri::generate_handler![get_app_usages, show_window])
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                let _item_handle = app.tray_handle().get_item(&id);
                match id.as_str() {
                    "quit" => {
                        write_to_file(EventType::StopMonitoring, get_current_timestamp(), "");
                        std::process::exit(0);
                    }
                    "dashboard" => {
                        let window = app.get_window("main").unwrap();
                        window.emit("refresh_data", "").unwrap();
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                event.window().hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
