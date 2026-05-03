pub fn detect() -> String {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return detect_wayland();
    }
    detect_x11()
}

fn detect_x11() -> String {
    let window_id = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if window_id.is_empty() {
        return String::new();
    }

    let pid = std::process::Command::new("xdotool")
        .args(["getwindowpid", &window_id])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if !pid.is_empty() {
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
            let name = comm.trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    std::process::Command::new("xdotool")
        .args(["getwindowname", &window_id])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn detect_wayland() -> String {
    let output = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/gnome/Shell",
            "--method",
            "org.gnome.Shell.Eval",
            "global.display.focus_window ? global.display.focus_window.get_wm_class() : ''",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    output
        .split('\'')
        .nth(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}
