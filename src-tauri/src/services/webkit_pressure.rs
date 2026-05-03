pub fn install() {
    use webkit2gtk::{MemoryPressureSettings, WebsiteDataManager};

    let mut settings = MemoryPressureSettings::new();
    settings.set_memory_limit(2048);
    settings.set_kill_threshold(0.95);
    settings.set_strict_threshold(0.75);
    settings.set_conservative_threshold(0.50);
    settings.set_poll_interval(30.0);
    WebsiteDataManager::set_memory_pressure_settings(&mut settings);
    log::info!(
        target: "app_lib",
        "WebKit memory pressure: limit=2048MB conservative=0.50 strict=0.75 kill=0.95 poll=30s",
    );
}
