use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::models::message::NodeUpdate;

const REFRESH_THRESHOLD_SECS: i64 = 20 * 60;

struct CachedContext {
    resolved: String,
    resolved_at: chrono::DateTime<chrono::Local>,
    last_context_hash: Option<u64>,
}

#[derive(Default)]
pub struct ConversationContextCache {
    entries: HashMap<String, CachedContext>,
}

impl ConversationContextCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has(&self, tab_id: &str) -> bool {
        self.entries.contains_key(tab_id)
    }

    pub fn get(&self, tab_id: &str) -> Option<&str> {
        self.entries.get(tab_id).map(|e| e.resolved.as_str())
    }

    pub fn insert(&mut self, tab_id: String, resolved: String) {
        self.entries.insert(
            tab_id,
            CachedContext {
                resolved,
                resolved_at: chrono::Local::now(),
                last_context_hash: None,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn insert_aged(&mut self, tab_id: String, resolved: String, age: chrono::Duration) {
        self.insert(tab_id.clone(), resolved);
        if let Some(entry) = self.entries.get_mut(&tab_id) {
            entry.resolved_at = chrono::Local::now() - age;
        }
    }

    pub fn environment_update_if_stale(
        &self,
        tab_id: &str,
        active_app: &str,
        recent_apps: &str,
    ) -> Option<NodeUpdate> {
        let entry = self.entries.get(tab_id)?;
        let elapsed = chrono::Local::now()
            .signed_duration_since(entry.resolved_at)
            .num_seconds();

        if elapsed < REFRESH_THRESHOLD_SECS {
            return None;
        }

        let now = chrono::Local::now();
        Some(NodeUpdate::Environment {
            value: format!(
                "[Current time: {} {} ({}) | Active app: {} | Recent apps: {}]",
                now.format("%Y-%m-%d"),
                now.format("%H:%M"),
                now.format("%Z"),
                active_app,
                recent_apps,
            ),
        })
    }

    pub fn pending_updates(
        &self,
        tab_id: &str,
        active_app: &str,
        recent_apps: &str,
        context_text: &str,
        image_count: usize,
        image_data_len: usize,
    ) -> Vec<NodeUpdate> {
        self.environment_update_if_stale(tab_id, active_app, recent_apps)
            .into_iter()
            .chain(self.context_update_if_changed(
                tab_id,
                context_text,
                image_count,
                image_data_len,
            ))
            .collect()
    }

    pub fn mark_context_sent(
        &mut self,
        tab_id: &str,
        context_text: &str,
        image_count: usize,
        image_data_len: usize,
    ) {
        if let Some(entry) = self.entries.get_mut(tab_id) {
            entry.last_context_hash = Some(compute_context_hash(
                context_text,
                image_count,
                image_data_len,
            ));
        }
    }

    pub fn context_update_if_changed(
        &self,
        tab_id: &str,
        context_text: &str,
        image_count: usize,
        image_data_len: usize,
    ) -> Option<NodeUpdate> {
        let new_hash = compute_context_hash(context_text, image_count, image_data_len);
        let previous = self.entries.get(tab_id).and_then(|e| e.last_context_hash);
        let cleared = context_text.is_empty() && image_count == 0;

        let (content, reason) = match previous {
            None if cleared => return None,
            None => (context_text, "initial"),
            Some(prev_hash) if prev_hash == new_hash => return None,
            Some(_) if cleared => ("", "cleared"),
            Some(_) => (context_text, "replaced"),
        };
        Some(NodeUpdate::Context {
            content: content.to_string(),
            reason: reason.to_string(),
            image_refs: vec![],
        })
    }

    pub fn remove(&mut self, tab_id: &str) {
        self.entries.remove(tab_id);
    }
}

fn compute_context_hash(text: &str, image_count: usize, image_data_len: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    image_count.hash(&mut hasher);
    image_data_len.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context_update(
        cache: &mut ConversationContextCache,
        tab_id: &str,
        text: &str,
        image_count: usize,
        image_data_len: usize,
    ) -> Option<NodeUpdate> {
        let update = cache.context_update_if_changed(tab_id, text, image_count, image_data_len);
        cache.mark_context_sent(tab_id, text, image_count, image_data_len);
        update
    }

    #[test]
    fn insert_and_get() {
        let mut cache = ConversationContextCache::new();
        assert!(!cache.has("tab-1"));

        cache.insert("tab-1".to_string(), "resolved-value".to_string());
        assert!(cache.has("tab-1"));
        assert_eq!(cache.get("tab-1"), Some("resolved-value"));
    }

    #[test]
    fn second_insert_does_not_overwrite() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "first".to_string());
        assert_eq!(cache.get("tab-1"), Some("first"));

        assert!(cache.has("tab-1"));
    }

    #[test]
    fn environment_update_returns_none_when_fresh() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "value".to_string());
        assert!(cache
            .environment_update_if_stale("tab-1", "Safari", "Safari, Code")
            .is_none());
    }

    #[test]
    fn environment_update_returns_none_for_unknown_tab() {
        let cache = ConversationContextCache::new();
        assert!(cache
            .environment_update_if_stale("unknown", "Safari", "Safari, Code")
            .is_none());
    }

    #[test]
    fn remove_clears_entry() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "value".to_string());
        cache.remove("tab-1");
        assert!(!cache.has("tab-1"));
        assert_eq!(cache.get("tab-1"), None);
    }

    #[test]
    fn context_update_initial_with_content() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        let update = context_update(&mut cache, "tab-1", "some context", 0, 0);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "initial"
        ));
    }

    #[test]
    fn context_update_none_for_empty_initial() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        let update = context_update(&mut cache, "tab-1", "", 0, 0);
        assert!(update.is_none());
    }

    #[test]
    fn context_update_none_when_unchanged() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        context_update(&mut cache, "tab-1", "ctx", 0, 0);
        let update = context_update(&mut cache, "tab-1", "ctx", 0, 0);
        assert!(update.is_none());
    }

    #[test]
    fn context_update_replaced_when_changed() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        context_update(&mut cache, "tab-1", "ctx-a", 0, 0);
        let update = context_update(&mut cache, "tab-1", "ctx-b", 0, 0);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "replaced"
        ));
    }

    #[test]
    fn context_update_cleared_when_emptied() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        context_update(&mut cache, "tab-1", "ctx", 0, 0);
        let update = context_update(&mut cache, "tab-1", "", 0, 0);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "cleared"
        ));
    }

    #[test]
    fn context_update_detects_image_change() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        context_update(&mut cache, "tab-1", "ctx", 1, 100);
        let update = context_update(&mut cache, "tab-1", "ctx", 2, 200);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "replaced"
        ));
    }

    #[test]
    fn pending_updates_does_not_mark_context_sent() {
        let mut cache = ConversationContextCache::new();
        cache.insert_aged("tab-1".to_string(), "env".to_string(), chrono::Duration::hours(1));

        let first = cache.pending_updates("tab-1", "Safari", "Safari", "ctx", 0, 0);
        let second = cache.pending_updates("tab-1", "Safari", "Safari", "ctx", 0, 0);

        assert_eq!(first.len(), 2);
        assert!(matches!(first[0], NodeUpdate::Environment { .. }));
        assert!(matches!(first[1], NodeUpdate::Context { .. }));
        assert_eq!(first, second);
    }

    #[test]
    fn pending_updates_treats_unknown_tab_as_new() {
        let cache = ConversationContextCache::new();
        let updates = cache.pending_updates("unknown", "Safari", "Safari", "ctx", 0, 0);
        assert!(matches!(
            updates.as_slice(),
            [NodeUpdate::Context { reason, .. }] if reason == "initial"
        ));
    }

    #[test]
    fn mark_context_sent_silences_pending_context_update() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        cache.mark_context_sent("tab-1", "ctx", 0, 0);
        assert!(cache
            .pending_updates("tab-1", "Safari", "Safari", "ctx", 0, 0)
            .is_empty());
    }
}
