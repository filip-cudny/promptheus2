pub fn detect() -> String {
    let script =
        r#"tell application "System Events" to return name of first application process whose frontmost is true"#;
    std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}
