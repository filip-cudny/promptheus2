use tauri_plugin_log::{Builder, Target, TargetKind, TimezoneStrategy};

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut log_builder = Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
            Target::new(TargetKind::Webview),
        ])
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .level(log::LevelFilter::Info)
        .level_for("app_lib", log::LevelFilter::Debug);

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        for directive in rust_log.split(',') {
            let directive = directive.trim();
            if let Some((module, level_str)) = directive.split_once('=') {
                if let Ok(level) = level_str.parse::<log::LevelFilter>() {
                    log_builder = log_builder.level_for(module.to_string(), level);
                }
            } else if let Ok(level) = directive.parse::<log::LevelFilter>() {
                log_builder = log_builder.level(level);
            }
        }
    }

    log_builder.build()
}
