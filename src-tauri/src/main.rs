#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Timelike;
use cocoa::base::nil;
use cocoa::foundation::NSString;
use libproc::libproc::proc_pid;
use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, WindowEvent
};
use tauri::api::notification::Notification;
use tracing::{debug, error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Registry};

mod app_management;
mod sys_monitor;
mod utils;

use app_management::{add_app_to_login_items, is_app_in_login_items, terminate_previous_instance};
use sys_monitor::{get_app_usages_from_log, get_frontmost_window_pid, AppUsage, EventType};
use utils::{get_current_timestamp, write_to_file};

static LAST_CHECK_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

#[tauri::command]
fn get_app_usages_handler(start_timestamp: u64, end_timestamp: u64) -> Vec<AppUsage> {
    match get_app_usages_from_log(start_timestamp, end_timestamp) {
        Ok(usage) => usage,
        Err(e) => {
            error!("Failed to get app usage: {}", e);
            Vec::new()
        }
    }
}

#[tauri::command]
fn show_window_handler(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_window("main") {
        window.emit("refresh_data", "").unwrap();
        window.show().unwrap();
        window.set_focus().unwrap();
    } else {
        println!("Failed to get main window");
    }
}

/// Check if the frontmost window has changed. If so, log the event.
fn check(app: &AppHandle, shared_previous_path: Arc<Mutex<String>>) {
    let current_timestamp = get_current_timestamp();

    // Update the timestamp and check the time difference
    let last_timestamp = LAST_CHECK_TIMESTAMP.load(Ordering::SeqCst);
    if current_timestamp > last_timestamp + 10_000 {
        info!("More than 10 seconds passed since last check.");

        write_to_file(EventType::ShutDown, current_timestamp, "");
        let mut previous_path = shared_previous_path.lock().unwrap();
        *previous_path = String::new();
    }

    match get_frontmost_window_pid() {
        Ok(pid) => match proc_pid::pidpath(pid) {
            Ok(current_path) => {
                let mut previous_path = shared_previous_path.lock().unwrap();
                if *previous_path != current_path {
                    info!("New program: {}", current_path);
                    write_to_file(EventType::CameToFront, current_timestamp, &current_path);
                    *previous_path = current_path;
                }
            }
            _ => error!("Failed to retrieve process path for PID {}", pid),
        },
        Err(e) => error!("{}", e),
    }

    // Update LAST_CHECK_TIMESTAMP with the current timestamp
    LAST_CHECK_TIMESTAMP.store(current_timestamp, Ordering::SeqCst);

    // Notify the user when 23:30
    let now = chrono::Local::now();
    let title = "Flows";
    let body = "Check your screen time!";

    if now.hour() == 23 && now.minute() == 30 && now.second() == 0 {
        let _ = Notification::new(&app.config().tauri.bundle.identifier)
            .title(title)
            .body(body)
            .show();
        debug!("Notified user: {}: {}", title, body);
    }
}

fn show_alert(title: &str, message: &str) {
    unsafe {
        let alert_class = Class::get("NSAlert").unwrap();
        let alert: *mut Object = msg_send![alert_class, alloc];
        let alert: *mut Object = msg_send![alert, init];

        let _: () = msg_send![alert, setMessageText: NSString::alloc(nil).init_str(title)];
        let _: () = msg_send![alert, setInformativeText: NSString::alloc(nil).init_str(message)];

        let _: () = msg_send![alert, runModal];
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
    let shared_previous_path = Arc::new(Mutex::new(String::new()));
    let current_timestamp = get_current_timestamp();
    LAST_CHECK_TIMESTAMP.store(current_timestamp, Ordering::SeqCst);

    // Create the system tray
    let shared_path_clone = Arc::clone(&shared_previous_path);
    let dashboard = CustomMenuItem::new("dashboard".to_string(), "Dashboard");
    let about = CustomMenuItem::new("about".to_string(), "About");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(dashboard)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(about)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    let tray = SystemTray::new().with_menu(tray_menu);

    // Start the app
    tauri::Builder::default()
        .setup(move |app| {
            let shared_path_clone = Arc::clone(&shared_previous_path);
            let app_handle = app.handle();
            let _ = thread::spawn(move || {
                loop {
                    check(&app_handle, Arc::clone(&shared_path_clone));
                    thread::sleep(Duration::from_secs(1));
                }
            });
            Ok(app.set_activation_policy(tauri::ActivationPolicy::Accessory))
        })
        .invoke_handler(tauri::generate_handler![
            get_app_usages_handler,
            show_window_handler
        ])
        .system_tray(tray)
        .on_system_tray_event(move |app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                let _item_handle = app.tray_handle().get_item(&id);
                match id.as_str() {
                    "quit" => {
                        write_to_file(EventType::StopMonitoring, get_current_timestamp(), "");
                        std::process::exit(0);
                    }
                    "dashboard" => {
                        check(app, Arc::clone(&shared_path_clone));
                        let window = app.get_window("main").unwrap();
                        window.emit("refresh_data", "").unwrap();
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                    "about" => {
                        let version = "0.1.0";
                        let build_type = if cfg!(debug_assertions) {
                            "Debug"
                        } else {
                            "Release"
                        };
                        show_alert(
                            "About Flows",
                            &format!("Version: {} ({})", version, build_type),
                        );
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
