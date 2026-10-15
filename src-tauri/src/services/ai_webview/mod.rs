mod external_links;
mod lifecycle;
mod palette;
mod provider_send;
mod provider_swap;
mod scripts;
mod surface;
mod tabs;
mod unload;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex as StdMutex;
use std::time::Instant;

use crate::models::settings::WebviewProvider;

use lifecycle::CONVERSATION_DIALOG_LABEL;
use tabs::{TabKind, TabList};

pub use lifecycle::cleanup_host_state;
pub use palette::{swap_from_palette, swap_to_palette, PALETTE_BACKDROP_TITLE, PALETTE_TITLE};
pub use provider_send::send_to_provider_chat;
pub use provider_swap::{
    activate_adjacent_tab, activate_tab, activate_tab_at, close_active_tab, close_tab, get_tabs,
    new_tab, new_tab_like_active, reload_active_in_host, report_native_tabs, show_native_view,
};
pub use surface::{hide_host, surface, SurfaceTarget};
pub use tabs::{NativeTabReport, TabsSnapshot};
pub use unload::{run_unload_pass, UNLOAD_POLL_INTERVAL};

pub(super) struct PendingMessage {
    pub run: u64,
    pub transcript: String,
    pub script: String,
    pub host: String,
    pub delivered: bool,
}

#[derive(Default)]
pub struct AiWebviewState {
    tabs: StdMutex<TabList>,
    reserved_labels: StdMutex<HashMap<String, String>>,
    last_page_urls: StdMutex<HashMap<String, String>>,
    palette_open: StdMutex<HashMap<String, bool>>,
    pending_messages: StdMutex<HashMap<String, PendingMessage>>,
}

impl AiWebviewState {
    pub(super) fn with_tabs<R>(&self, f: impl FnOnce(&mut TabList) -> R) -> R {
        f(&mut self.tabs.lock().unwrap())
    }

    pub(super) fn provider_for_label(&self, label: &str) -> Option<WebviewProvider> {
        self.with_tabs(|tabs| match &tabs.by_child_label(label)?.kind {
            TabKind::Provider { provider } => Some(provider.clone()),
            TabKind::Native { .. } => None,
        })
    }

    pub(super) fn active_view_label(&self) -> String {
        self.with_tabs(|tabs| {
            tabs.active_id()
                .and_then(|id| tabs.get(id))
                .and_then(|tab| tab.child_label.clone())
        })
        .unwrap_or_else(|| CONVERSATION_DIALOG_LABEL.to_string())
    }

    pub(super) fn armed_tab_ids(&self) -> HashSet<String> {
        let tabs = self.tabs.lock().unwrap();
        let pending = self.pending_messages.lock().unwrap();
        tabs.tabs()
            .iter()
            .filter(|tab| {
                tab.child_label
                    .as_ref()
                    .and_then(|label| pending.get(label))
                    .is_some_and(|message| !message.delivered)
            })
            .map(|tab| tab.id.clone())
            .collect()
    }

    pub(super) fn arm_in_new_tab(
        &self,
        provider: &WebviewProvider,
        message: PendingMessage,
    ) -> (String, String) {
        let (tab_id, label) = self.with_tabs(|tabs| {
            let tab_id = tabs.push_provider(provider, Instant::now());
            let label = tabs.next_child_label(&provider.id);
            (tab_id, label)
        });
        self.reserved_labels
            .lock()
            .unwrap()
            .insert(tab_id.clone(), label.clone());
        self.arm_message(&label, message);
        (tab_id, label)
    }

    pub(super) fn child_label_for_new_child(&self, tab_id: &str, provider_id: &str) -> String {
        let reserved = self.reserved_labels.lock().unwrap().remove(tab_id);
        reserved.unwrap_or_else(|| self.with_tabs(|tabs| tabs.next_child_label(provider_id)))
    }

    pub(super) fn set_last_page_url(&self, label: &str, url: String) {
        self.last_page_urls
            .lock()
            .unwrap()
            .insert(label.to_string(), url);
    }

    pub(super) fn mark_unloaded(&self, tab_id: &str, url: Option<String>) -> Option<PendingMessage> {
        let mut tabs = self.tabs.lock().unwrap();
        let label = tabs.get(tab_id)?.child_label.clone()?;
        let page_url = self.last_page_urls.lock().unwrap().remove(&label);
        if let Some(url) = url.or(page_url) {
            tabs.set_last_url(tab_id, Some(url));
        }
        tabs.set_live(tab_id, None);
        self.pending_messages.lock().unwrap().remove(&label)
    }

    pub(super) fn drop_host(&self, host_label: &str) {
        self.palette_open.lock().unwrap().remove(host_label);
        if host_label != CONVERSATION_DIALOG_LABEL {
            return;
        }
        *self.tabs.lock().unwrap() = TabList::default();
        self.reserved_labels.lock().unwrap().clear();
        let prefix = format!("{host_label}::");
        self.last_page_urls
            .lock()
            .unwrap()
            .retain(|k, _| !k.starts_with(&prefix));
        self.pending_messages
            .lock()
            .unwrap()
            .retain(|k, _| !k.starts_with(&prefix));
    }

    pub(super) fn arm_message(&self, label: &str, msg: PendingMessage) -> Option<PendingMessage> {
        self.pending_messages
            .lock()
            .unwrap()
            .insert(label.to_string(), msg)
    }

    pub(super) fn take_delivery(&self, label: &str, page_host: &str) -> Option<(u64, String)> {
        let mut map = self.pending_messages.lock().unwrap();
        let msg = map.get_mut(label)?;
        if msg.delivered || msg.host != page_host {
            return None;
        }
        msg.delivered = true;
        Some((msg.run, msg.script.clone()))
    }

    pub(super) fn finish_message(&self, label: &str, run: u64) -> Option<PendingMessage> {
        let mut map = self.pending_messages.lock().unwrap();
        if map.get(label)?.run != run {
            return None;
        }
        map.remove(label)
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
    fn palette_open_consume_clears_only_when_open() {
        let state = AiWebviewState::default();
        assert!(!state.try_consume_palette_open("host"));

        state.set_palette_open("host", true);
        assert!(state.try_consume_palette_open("host"));
        assert!(!state.is_palette_open("host"));
        assert!(!state.try_consume_palette_open("host"));
    }

    #[test]
    fn drop_host_clears_tabs_and_pending_messages_of_its_children() {
        let state = AiWebviewState::default();
        state.set_palette_open(CONVERSATION_DIALOG_LABEL, true);
        let (_, label) = state.arm_in_new_tab(&provider("claude"), pending(1, "claude.ai"));
        state.arm_message("other::ai-webview-claude-1", pending(2, "claude.ai"));

        state.drop_host(CONVERSATION_DIALOG_LABEL);

        assert!(state.with_tabs(|tabs| tabs.tabs().is_empty()));
        assert!(!state.is_palette_open(CONVERSATION_DIALOG_LABEL));
        assert!(state.finish_message(&label, 1).is_none());
        assert_eq!(
            state.finish_message("other::ai-webview-claude-1", 2).map(|m| m.run),
            Some(2)
        );
    }

    #[test]
    fn pill_run_arms_on_a_new_label_before_any_child_exists() {
        let state = AiWebviewState::default();
        let (tab_id, label) = state.arm_in_new_tab(&provider("claude"), pending(1, "claude.ai"));

        assert_eq!(label, "conversation-dialog::ai-webview-claude-1");
        assert!(state.with_tabs(|tabs| tabs.by_child_label(&label).is_none()));
        assert_eq!(state.child_label_for_new_child(&tab_id, "claude"), label);
        assert_eq!(state.take_delivery(&label, "claude.ai"), Some((1, "script1".to_string())));
    }

    #[test]
    fn second_pill_run_gets_another_tab_and_label() {
        let state = AiWebviewState::default();
        let (first_tab, first_label) =
            state.arm_in_new_tab(&provider("claude"), pending(1, "claude.ai"));
        let (second_tab, second_label) =
            state.arm_in_new_tab(&provider("claude"), pending(2, "claude.ai"));

        assert_ne!(first_tab, second_tab);
        assert_ne!(first_label, second_label);
        assert_eq!(state.finish_message(&first_label, 1).map(|m| m.run), Some(1));
        assert_eq!(state.finish_message(&second_label, 2).map(|m| m.run), Some(2));
    }

    #[test]
    fn unloading_a_tab_returns_its_armed_message_and_leaves_none() {
        let state = AiWebviewState::default();
        let (tab_id, label) = state.arm_in_new_tab(&provider("claude"), pending(1, "claude.ai"));
        state.with_tabs(|tabs| tabs.set_live(&tab_id, Some(label.clone())));

        let message = state.mark_unloaded(&tab_id, Some("https://claude.ai/c/1".into()));

        assert_eq!(message.map(|m| m.run), Some(1));
        assert!(state.take_delivery(&label, "claude.ai").is_none());
        assert!(state.with_tabs(|tabs| tabs.get(&tab_id).unwrap().child_label.is_none()));
        assert_eq!(
            state.with_tabs(|tabs| tabs.get(&tab_id).unwrap().last_url.clone()).as_deref(),
            Some("https://claude.ai/c/1")
        );
    }

    #[test]
    fn unload_falls_back_to_the_last_page_load_url() {
        let state = AiWebviewState::default();
        let (tab_id, label) = state.arm_in_new_tab(&provider("claude"), pending(1, "claude.ai"));
        state.with_tabs(|tabs| tabs.set_live(&tab_id, Some(label.clone())));
        state.set_last_page_url(&label, "https://claude.ai/c/2".into());

        state.mark_unloaded(&tab_id, None);

        assert_eq!(
            state.with_tabs(|tabs| tabs.get(&tab_id).unwrap().last_url.clone()).as_deref(),
            Some("https://claude.ai/c/2")
        );
    }

    #[test]
    fn armed_set_skips_tabs_whose_message_was_delivered() {
        let state = AiWebviewState::default();
        let (waiting_tab, waiting_label) =
            state.arm_in_new_tab(&provider("claude"), pending(1, "claude.ai"));
        let (sent_tab, sent_label) =
            state.arm_in_new_tab(&provider("claude"), pending(2, "claude.ai"));
        state.with_tabs(|tabs| {
            tabs.set_live(&waiting_tab, Some(waiting_label.clone()));
            tabs.set_live(&sent_tab, Some(sent_label.clone()));
        });
        assert!(state.take_delivery(&sent_label, "claude.ai").is_some());

        let armed = state.armed_tab_ids();

        assert!(armed.contains(&waiting_tab));
        assert!(!armed.contains(&sent_tab));
    }

    fn pending(run: u64, host: &str) -> PendingMessage {
        PendingMessage {
            run,
            transcript: "t".into(),
            script: format!("script{run}"),
            host: host.into(),
            delivered: false,
        }
    }

    #[test]
    fn pending_message_is_delivered_once() {
        let state = AiWebviewState::default();
        state.arm_message("w", pending(1, "claude.ai"));
        assert_eq!(state.take_delivery("w", "claude.ai"), Some((1, "script1".to_string())));
        assert!(state.take_delivery("w", "claude.ai").is_none());
    }

    #[test]
    fn pending_message_waits_for_provider_host() {
        let state = AiWebviewState::default();
        state.arm_message("w", pending(1, "claude.ai"));
        assert!(state.take_delivery("w", "login.example").is_none());
        assert_eq!(state.take_delivery("w", "claude.ai"), Some((1, "script1".to_string())));
    }

    #[test]
    fn finish_message_requires_matching_run() {
        let state = AiWebviewState::default();
        state.arm_message("w", pending(1, "claude.ai"));
        assert!(state.finish_message("w", 2).is_none());
        assert_eq!(state.finish_message("w", 1).map(|m| m.run), Some(1));
        assert!(state.finish_message("w", 1).is_none());
    }

    #[test]
    fn arm_message_returns_replaced() {
        let state = AiWebviewState::default();
        assert!(state.arm_message("w", pending(1, "claude.ai")).is_none());
        assert_eq!(state.arm_message("w", pending(2, "claude.ai")).map(|m| m.run), Some(1));
    }
}
