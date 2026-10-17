use std::collections::VecDeque;
use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use super::config::ConfigService;
use super::frontmost_app;

#[derive(Default)]
pub struct RecentAppsState {
    apps: Mutex<VecDeque<String>>,
}

impl RecentAppsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn push(&self, app: String, max: usize) {
        if app.is_empty() || app.to_lowercase().contains("promptheus") {
            return;
        }
        let mut apps = self.apps.lock().await;
        apps.retain(|a| a != &app);
        apps.push_front(app);
        apps.truncate(max);
    }

    pub async fn record_frontmost(app: &tauri::AppHandle) {
        let detected = frontmost_app::detect();
        let max = app
            .state::<Arc<Mutex<ConfigService>>>()
            .lock()
            .await
            .settings()
            .recent_apps_count;
        app.state::<Arc<RecentAppsState>>().push(detected, max).await;
    }

    pub async fn active(&self) -> String {
        self.apps
            .lock()
            .await
            .front()
            .cloned()
            .unwrap_or_default()
    }

    pub async fn display(&self) -> String {
        self.apps
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }
}
