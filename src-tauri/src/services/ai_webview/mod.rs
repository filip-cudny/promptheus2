mod cold_suspend;
mod lifecycle;
mod palette;
mod provider_swap;
mod scripts;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use crate::models::settings::WebviewProvider;

pub use cold_suspend::{run_cold_suspend_pass, COLD_SUSPEND_POLL_INTERVAL};
pub use lifecycle::{
    cleanup_host_state, close, focus_conversation_dialog, navigate, open_new_instance,
    open_or_focus,
};
pub use palette::{swap_from_palette, swap_to_palette};
pub use provider_swap::{
    active_provider_for, emit_active_changed_for, mark_active_webview, new_chat_in_host,
    reload_active_in_host, swap_to_conversation_dialog, swap_to_provider,
};

#[derive(Default)]
pub struct AiWebviewState {
    hosted: StdMutex<HashMap<String, HashSet<String>>>,
    current_provider: StdMutex<HashMap<String, String>>,
    provider_snapshots: StdMutex<HashMap<String, WebviewProvider>>,
    active_webview: StdMutex<HashMap<String, String>>,
    palette_open: StdMutex<HashMap<String, bool>>,
    pending_provider: StdMutex<HashMap<String, String>>,
    last_active: StdMutex<HashMap<String, Instant>>,
    suspended: StdMutex<HashMap<String, String>>,
}

impl AiWebviewState {
    pub(super) fn set_provider(&self, label: &str, provider: &WebviewProvider) {
        self.current_provider
            .lock()
            .unwrap()
            .insert(label.to_string(), provider.id.clone());
        self.provider_snapshots
            .lock()
            .unwrap()
            .insert(label.to_string(), provider.clone());
    }

    pub(super) fn remove_provider(&self, label: &str) {
        self.current_provider.lock().unwrap().remove(label);
        self.provider_snapshots.lock().unwrap().remove(label);
    }

    pub(super) fn snapshot(&self, label: &str) -> Option<WebviewProvider> {
        self.provider_snapshots.lock().unwrap().get(label).cloned()
    }

    pub(super) fn current_provider_for(&self, label: &str) -> Option<String> {
        self.current_provider.lock().unwrap().get(label).cloned()
    }

    pub(super) fn has_hosted_child(&self, host_label: &str, provider_id: &str) -> bool {
        self.hosted
            .lock()
            .unwrap()
            .get(host_label)
            .map(|set| set.contains(provider_id))
            .unwrap_or(false)
    }

    pub(super) fn mark_hosted_child(&self, host_label: &str, provider_id: &str) {
        self.hosted
            .lock()
            .unwrap()
            .entry(host_label.to_string())
            .or_default()
            .insert(provider_id.to_string());
    }

    pub(super) fn unmark_hosted_child(&self, host_label: &str, provider_id: &str) {
        if let Some(set) = self.hosted.lock().unwrap().get_mut(host_label) {
            set.remove(provider_id);
        }
    }

    pub(super) fn drop_host(&self, host_label: &str) {
        self.hosted.lock().unwrap().remove(host_label);
        self.active_webview.lock().unwrap().remove(host_label);
        self.palette_open.lock().unwrap().remove(host_label);
        let prefix = format!("{host_label}::");
        self.last_active
            .lock()
            .unwrap()
            .retain(|k, _| !k.starts_with(&prefix) && k != host_label);
        self.suspended
            .lock()
            .unwrap()
            .retain(|k, _| !k.starts_with(&prefix) && k != host_label);
    }

    pub fn set_pending_provider(&self, host_label: &str, provider_id: &str) {
        self.pending_provider
            .lock()
            .unwrap()
            .insert(host_label.to_string(), provider_id.to_string());
    }

    pub fn take_pending_provider(&self, host_label: &str) -> Option<String> {
        self.pending_provider.lock().unwrap().remove(host_label)
    }

    pub(super) fn set_active(&self, host_label: &str, webview_label: &str) {
        self.active_webview
            .lock()
            .unwrap()
            .insert(host_label.to_string(), webview_label.to_string());
        self.last_active
            .lock()
            .unwrap()
            .insert(webview_label.to_string(), Instant::now());
    }

    pub(super) fn touch_last_active(&self, webview_label: &str) {
        self.last_active
            .lock()
            .unwrap()
            .insert(webview_label.to_string(), Instant::now());
    }

    pub fn is_suspended(&self, webview_label: &str) -> bool {
        self.suspended.lock().unwrap().contains_key(webview_label)
    }

    pub(super) fn mark_suspended(&self, webview_label: &str, restore_url: String) {
        self.suspended
            .lock()
            .unwrap()
            .insert(webview_label.to_string(), restore_url);
    }

    pub(super) fn take_suspended_url(&self, webview_label: &str) -> Option<String> {
        self.suspended.lock().unwrap().remove(webview_label)
    }

    pub(super) fn unmark_suspended(&self, webview_label: &str) {
        self.suspended.lock().unwrap().remove(webview_label);
    }

    pub(super) fn cold_suspend_candidates(&self, idle_threshold: Duration) -> Vec<String> {
        let now = Instant::now();
        let active: HashSet<String> = self
            .active_webview
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        let suspended = self.suspended.lock().unwrap();
        self.last_active
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(label, last)| {
                if !cold_suspend::is_suspendable_webview_label(label) {
                    return None;
                }
                if active.contains(label) {
                    return None;
                }
                if suspended.contains_key(label) {
                    return None;
                }
                if now.duration_since(*last) < idle_threshold {
                    return None;
                }
                Some(label.clone())
            })
            .collect()
    }

    pub(super) fn get_active(&self, host_label: &str) -> Option<String> {
        self.active_webview
            .lock()
            .unwrap()
            .get(host_label)
            .cloned()
    }

    pub(super) fn set_palette_open(&self, host_label: &str, open: bool) {
        self.palette_open
            .lock()
            .unwrap()
            .insert(host_label.to_string(), open);
    }

    pub(super) fn is_palette_open(&self, host_label: &str) -> bool {
        self.palette_open
            .lock()
            .unwrap()
            .get(host_label)
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn try_consume_palette_open(&self, host_label: &str) -> bool {
        let mut map = self.palette_open.lock().unwrap();
        let was_open = map.get(host_label).copied().unwrap_or(false);
        if was_open {
            map.insert(host_label.to_string(), false);
        }
        was_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str) -> WebviewProvider {
        WebviewProvider {
            id: id.into(),
            name: id.into(),
            url: format!("https://{id}.example/"),
        }
    }

    #[test]
    fn provider_set_remove_round_trip() {
        let state = AiWebviewState::default();
        let p = provider("claude");
        state.set_provider("ai-webview-claude", &p);
        assert_eq!(
            state.snapshot("ai-webview-claude").map(|p| p.id).as_deref(),
            Some("claude")
        );
        assert_eq!(
            state.current_provider_for("ai-webview-claude").as_deref(),
            Some("claude")
        );
        state.remove_provider("ai-webview-claude");
        assert!(state.snapshot("ai-webview-claude").is_none());
        assert!(state.current_provider_for("ai-webview-claude").is_none());
    }

    #[test]
    fn hosted_child_tracking() {
        let state = AiWebviewState::default();
        assert!(!state.has_hosted_child("host", "claude"));
        state.mark_hosted_child("host", "claude");
        assert!(state.has_hosted_child("host", "claude"));
        state.unmark_hosted_child("host", "claude");
        assert!(!state.has_hosted_child("host", "claude"));
    }

    #[test]
    fn drop_host_clears_state() {
        let state = AiWebviewState::default();
        state.mark_hosted_child("host", "claude");
        state.set_palette_open("host", true);
        state.set_active("host", "host::ai-webview-claude");
        state.mark_suspended("host::ai-webview-claude", "https://x".into());

        state.drop_host("host");

        assert!(!state.has_hosted_child("host", "claude"));
        assert!(!state.is_palette_open("host"));
        assert!(state.get_active("host").is_none());
        assert!(!state.is_suspended("host::ai-webview-claude"));
    }

    #[test]
    fn palette_open_consume_clears_only_when_open() {
        let state = AiWebviewState::default();
        assert!(!state.try_consume_palette_open("host"));

        state.set_palette_open("host", true);
        assert!(state.try_consume_palette_open("host"));
        assert!(!state.is_palette_open("host"));
        assert!(!state.try_consume_palette_open("host"));
    }

    #[test]
    fn pending_provider_take_is_destructive() {
        let state = AiWebviewState::default();
        state.set_pending_provider("host", "claude");
        assert_eq!(state.take_pending_provider("host").as_deref(), Some("claude"));
        assert!(state.take_pending_provider("host").is_none());
    }

    #[test]
    fn suspend_round_trip() {
        let state = AiWebviewState::default();
        state.mark_suspended("ai-webview-claude", "https://claude.ai/c/123".into());
        assert!(state.is_suspended("ai-webview-claude"));
        assert_eq!(
            state.take_suspended_url("ai-webview-claude").as_deref(),
            Some("https://claude.ai/c/123")
        );
        assert!(!state.is_suspended("ai-webview-claude"));
    }

    #[test]
    fn cold_suspend_candidates_skip_active() {
        let state = AiWebviewState::default();
        let p = provider("claude");
        state.set_provider("ai-webview-claude", &p);
        state.set_active("host", "ai-webview-claude");

        let zero = Duration::from_secs(0);
        let candidates = state.cold_suspend_candidates(zero);
        assert!(
            !candidates.contains(&"ai-webview-claude".to_string()),
            "active webview should not be a candidate"
        );
    }

    #[test]
    fn cold_suspend_candidates_skip_already_suspended() {
        let state = AiWebviewState::default();
        let p = provider("claude");
        state.set_provider("ai-webview-claude", &p);
        state.set_active("host", "ai-webview-other");
        state.mark_suspended("ai-webview-claude", "https://x".into());
        state.touch_last_active("ai-webview-claude");

        let zero = Duration::from_secs(0);
        let candidates = state.cold_suspend_candidates(zero);
        assert!(!candidates.contains(&"ai-webview-claude".to_string()));
    }
}
