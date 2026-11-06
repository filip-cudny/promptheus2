use tauri_plugin_log::{Builder, Target, TargetKind, TimezoneStrategy};

fn format_line(
    at: chrono::NaiveDateTime,
    level: log::Level,
    target: &str,
    message: &std::fmt::Arguments,
) -> String {
    format!("[{}][{}][{level}][{target}] {message}", at.format("%Y-%m-%d"), at.format("%H:%M:%S%.3f"))
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut log_builder = Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
            Target::new(TargetKind::Webview),
        ])
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .format(|out, message, record| {
            let now = chrono::Local::now().naive_local();
            out.finish(format_args!("{}", format_line(now, record.level(), record.target(), message)))
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_milliseconds_level_target_and_message() {
        let at = chrono::NaiveDate::from_ymd_opt(2026, 11, 6)
            .unwrap()
            .and_hms_milli_opt(13, 34, 0, 839)
            .unwrap();
        let line = format_line(at, log::Level::Info, "t", &format_args!("m"));
        assert_eq!(line, "[2026-11-06][13:34:00.839][INFO][t] m");
    }
}
