use std::collections::HashMap;

const REFRESH_THRESHOLD_SECS: i64 = 20 * 60;

struct CachedContext {
    resolved: String,
    resolved_at: chrono::DateTime<chrono::Local>,
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
            },
        );
    }

    pub fn time_update_if_stale(&self, tab_id: &str) -> Option<String> {
        let entry = self.entries.get(tab_id)?;
        let elapsed = chrono::Local::now()
            .signed_duration_since(entry.resolved_at)
            .num_seconds();

        if elapsed < REFRESH_THRESHOLD_SECS {
            return None;
        }

        let now = chrono::Local::now();
        Some(format!(
            "[Current time: {} {} ({})]",
            now.format("%Y-%m-%d"),
            now.format("%H:%M"),
            now.format("%Z"),
        ))
    }

    pub fn remove(&mut self, tab_id: &str) {
        self.entries.remove(tab_id);
    }
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
    fn time_update_returns_none_when_fresh() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "value".to_string());
        assert!(cache.time_update_if_stale("tab-1").is_none());
    }

    #[test]
    fn time_update_returns_none_for_unknown_tab() {
        let cache = ConversationContextCache::new();
        assert!(cache.time_update_if_stale("unknown").is_none());
    }

    #[test]
    fn remove_clears_entry() {
        let mut cache = ConversationContextCache::new();
        cache.insert("tab-1".to_string(), "value".to_string());
        cache.remove("tab-1");
        assert!(!cache.has("tab-1"));
        assert_eq!(cache.get("tab-1"), None);
    }
}
