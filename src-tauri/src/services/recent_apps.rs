use std::collections::VecDeque;

use tokio::sync::Mutex;

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
