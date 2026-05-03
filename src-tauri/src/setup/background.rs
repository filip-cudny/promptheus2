use crate::services;

pub fn spawn_heartbeat(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut counter: u64 = 0;
        loop {
            interval.tick().await;
            counter += 1;
            let n = counter;
            let dispatched_at = std::time::Instant::now();
            log::info!(
                target: "app_lib::heartbeat",
                "heartbeat dispatch #{n}",
            );
            if let Err(e) = app_handle.run_on_main_thread(move || {
                let elapsed = dispatched_at.elapsed();
                log::info!(
                    target: "app_lib::heartbeat",
                    "heartbeat tick #{n} delay={elapsed:?}",
                );
            }) {
                log::warn!(
                    target: "app_lib::heartbeat",
                    "heartbeat dispatch #{n} failed: {e}",
                );
            }
        }
    });
}

pub fn spawn_ai_webview_cold_suspend(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval =
            tokio::time::interval(services::ai_webview::COLD_SUSPEND_POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            interval.tick().await;
            let app = app_handle.clone();
            if let Err(e) = app_handle.run_on_main_thread(move || {
                services::ai_webview::run_cold_suspend_pass(app);
            }) {
                log::warn!(
                    target: "app_lib::ai_webview",
                    "cold-suspend dispatch failed: {e}",
                );
            }
        }
    });
}
