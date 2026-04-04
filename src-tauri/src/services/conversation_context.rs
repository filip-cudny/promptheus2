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

    pub fn context_update_if_changed(
        &mut self,
        tab_id: &str,
        context_text: &str,
        image_count: usize,
        image_data_len: usize,
    ) -> Option<NodeUpdate> {
        let new_hash = compute_context_hash(context_text, image_count, image_data_len);
        let entry = self.entries.get_mut(tab_id)?;

        match entry.last_context_hash {
            None => {
                entry.last_context_hash = Some(new_hash);
                if context_text.is_empty() && image_count == 0 {
                    return None;
                }
                Some(NodeUpdate::Context {
                    content: context_text.to_string(),
                    reason: "initial".to_string(),
                    image_refs: vec![],
                })
            }
            Some(prev_hash) if prev_hash == new_hash => None,
            Some(_) => {
                entry.last_context_hash = Some(new_hash);
                if context_text.is_empty() && image_count == 0 {
                    Some(NodeUpdate::Context {
                        content: String::new(),
                        reason: "cleared".to_string(),
                        image_refs: vec![],
                    })
                } else {
                    Some(NodeUpdate::Context {
                        content: context_text.to_string(),
                        reason: "replaced".to_string(),
                        image_refs: vec![],
                    })
                }
            }
        }
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
        let update = cache.context_update_if_changed("tab-1", "some context", 0, 0);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "initial"
        ));
    }

    #[test]
    fn context_update_none_for_empty_initial() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        let update = cache.context_update_if_changed("tab-1", "", 0, 0);
        assert!(update.is_none());
    }

    #[test]
    fn context_update_none_when_unchanged() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        cache.context_update_if_changed("tab-1", "ctx", 0, 0);
        let update = cache.context_update_if_changed("tab-1", "ctx", 0, 0);
        assert!(update.is_none());
    }

    #[test]
    fn context_update_replaced_when_changed() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        cache.context_update_if_changed("tab-1", "ctx-a", 0, 0);
        let update = cache.context_update_if_changed("tab-1", "ctx-b", 0, 0);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "replaced"
        ));
    }

    #[test]
    fn context_update_cleared_when_emptied() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        cache.context_update_if_changed("tab-1", "ctx", 0, 0);
        let update = cache.context_update_if_changed("tab-1", "", 0, 0);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "cleared"
        ));
    }

    #[test]
    fn context_update_detects_image_change() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "env".to_string());
        cache.context_update_if_changed("tab-1", "ctx", 1, 100);
        let update = cache.context_update_if_changed("tab-1", "ctx", 2, 200);
        assert!(matches!(
            update,
            Some(NodeUpdate::Context { reason, .. }) if reason == "replaced"
        ));
    }
}
