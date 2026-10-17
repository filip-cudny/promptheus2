pub fn detect() -> String {
    detect_wayland()
}

fn detect_wayland() -> String {
    let result = crate::services::gnome_shell::blocking_proxy()
        .and_then(|proxy| proxy.get_focused_wm_class());
    match result {
        Ok(wm_class) => wm_class,
        Err(e) => {
            log::debug!("failed to read focused wm_class from the shell extension: {e}");
            String::new()
        }
    }
}
