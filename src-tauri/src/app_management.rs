use std::process::{id, Command};

pub fn terminate_previous_instance(app_name: &str) {
    let output = Command::new("pgrep")
        .arg(app_name)
        .output()
        .expect("Failed to execute pgrep");

    let current_pid = id();

    if !output.stdout.is_empty() {
        let pids_str = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<&str> = pids_str.split_whitespace().collect();

        for pid in pids {
            if let Ok(pid) = pid.parse::<u32>() {
                if pid != current_pid {
                    Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .output()
                        .expect("Failed to terminate previous instance");
                    println!("Terminated instance with PID: {}", pid);
                }
            }
        }
    }
}

pub fn is_app_in_login_items(app_path: &str) -> bool {
    let script = format!(
        r#"tell application "System Events" to get the path of every login item whose path is "{}""#,
        app_path
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .expect("Failed to execute osascript");

    let stdout = String::from_utf8_lossy(&output.stdout);
    !stdout.trim().is_empty()
}

pub fn add_app_to_login_items(app_path: &str) {
    let script = format!(
        r#"tell application "System Events" to make login item at end with properties {{path:"{}", hidden:false}}"#,
        app_path
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .expect("Failed to execute osascript");
}
