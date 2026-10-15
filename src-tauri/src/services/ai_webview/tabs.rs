use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::models::settings::WebviewProvider;

pub const MAX_LIVE_PROVIDER_TABS: usize = 3;
pub const UNLOAD_IDLE: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone)]
pub enum TabKind {
    Native { tab_id: String },
    Provider { provider: WebviewProvider },
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub kind: TabKind,
    pub child_label: Option<String>,
    pub last_url: Option<String>,
    pub title: String,
    pub streaming: bool,
    pub last_used: Instant,
}

impl Tab {
    fn is_native(&self) -> bool {
        matches!(self.kind, TabKind::Native { .. })
    }

    fn is_live_provider(&self) -> bool {
        !self.is_native() && self.child_label.is_some()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TabDto {
    pub id: String,
    pub kind: &'static str,
    pub provider_id: Option<String>,
    pub native_tab_id: Option<String>,
    pub title: String,
    pub live: bool,
    pub streaming: bool,
    pub recency: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TabsSnapshot {
    pub tabs: Vec<TabDto>,
    pub active_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NativeTabReport {
    pub tab_id: String,
    pub title: String,
    pub streaming: bool,
    pub listed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseOutcome {
    pub successor: Option<String>,
    pub list_empty: bool,
}

#[derive(Debug, Default)]
pub struct TabList {
    tabs: Vec<Tab>,
    active: Option<String>,
    provider_counter: u32,
    label_counters: HashMap<String, u32>,
}

fn native_id(tab_id: &str) -> String {
    format!("native:{tab_id}")
}

impl TabList {
    pub fn active_id(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn get(&self, tab_id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == tab_id)
    }

    fn get_mut(&mut self, tab_id: &str) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == tab_id)
    }

    pub fn by_child_label(&self, label: &str) -> Option<&Tab> {
        self.tabs
            .iter()
            .find(|t| t.child_label.as_deref() == Some(label))
    }

    pub fn push_provider(&mut self, provider: &WebviewProvider, now: Instant) -> String {
        self.provider_counter += 1;
        let id = format!("provider:{}", self.provider_counter);
        self.tabs.push(Tab {
            id: id.clone(),
            title: provider.name.clone(),
            kind: TabKind::Provider {
                provider: provider.clone(),
            },
            child_label: None,
            last_url: None,
            streaming: false,
            last_used: now,
        });
        id
    }

    pub fn next_child_label(&mut self, provider_id: &str) -> String {
        let counter = self
            .label_counters
            .entry(provider_id.to_string())
            .or_insert(0);
        *counter += 1;
        format!("conversation-dialog::ai-webview-{provider_id}-{counter}")
    }

    pub fn set_live(&mut self, tab_id: &str, label: Option<String>) {
        if let Some(tab) = self.get_mut(tab_id) {
            tab.child_label = label;
        }
    }

    pub fn set_title(&mut self, tab_id: &str, title: String) {
        if let Some(tab) = self.get_mut(tab_id) {
            tab.title = title;
        }
    }

    pub fn set_last_url(&mut self, tab_id: &str, url: Option<String>) {
        if let Some(tab) = self.get_mut(tab_id) {
            tab.last_url = url;
        }
    }

    fn stamp_active(&mut self, now: Instant) {
        if let Some(id) = self.active.clone() {
            if let Some(tab) = self.get_mut(&id) {
                tab.last_used = now;
            }
        }
    }

    pub fn activate(&mut self, tab_id: &str, now: Instant) {
        if self.get(tab_id).is_none() {
            return;
        }
        self.stamp_active(now);
        if let Some(tab) = self.get_mut(tab_id) {
            tab.last_used = now;
        }
        self.active = Some(tab_id.to_string());
    }

    pub fn set_native_view(&mut self, now: Instant) {
        self.stamp_active(now);
        self.active = None;
    }

    pub fn close(&mut self, tab_id: &str, now: Instant) -> CloseOutcome {
        let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) else {
            return CloseOutcome {
                successor: None,
                list_empty: self.tabs.is_empty(),
            };
        };
        self.tabs.remove(index);
        let mut successor = None;
        if self.active.as_deref() == Some(tab_id) {
            successor = self
                .tabs
                .get(index)
                .or_else(|| index.checked_sub(1).and_then(|i| self.tabs.get(i)))
                .map(|t| t.id.clone());
            self.active = None;
            if let Some(id) = &successor {
                self.activate(id, now);
            }
        }
        CloseOutcome {
            successor,
            list_empty: self.tabs.is_empty(),
        }
    }

    #[allow(dead_code)]
    pub fn adjacent(&self, step: i32) -> Option<String> {
        let len = self.tabs.len() as i32;
        if len == 0 {
            return None;
        }
        let current = self
            .active
            .as_deref()
            .and_then(|id| self.tabs.iter().position(|t| t.id == id));
        let index = match current {
            Some(i) => (i as i32 + step).rem_euclid(len),
            None if step >= 0 => 0,
            None => len - 1,
        };
        Some(self.tabs[index as usize].id.clone())
    }

    #[allow(dead_code)]
    pub fn at_position(&self, position: u32) -> Option<String> {
        match position {
            1..=8 => self.tabs.get(position as usize - 1),
            9 => self.tabs.last(),
            _ => None,
        }
        .map(|t| t.id.clone())
    }

    pub fn apply_native_snapshot(
        &mut self,
        reports: &[NativeTabReport],
        active_native_id: Option<&str>,
        now: Instant,
    ) {
        let listed: HashSet<&str> = reports
            .iter()
            .filter(|r| r.listed)
            .map(|r| r.tab_id.as_str())
            .collect();
        self.tabs.retain(|t| match &t.kind {
            TabKind::Native { tab_id } => listed.contains(tab_id.as_str()),
            TabKind::Provider { .. } => true,
        });
        if self
            .active
            .as_deref()
            .is_some_and(|id| self.get(id).is_none())
        {
            self.active = None;
        }
        for report in reports.iter().filter(|r| r.listed) {
            let id = native_id(&report.tab_id);
            match self.get_mut(&id) {
                Some(tab) => {
                    tab.title = report.title.clone();
                    tab.streaming = report.streaming;
                }
                None => self.tabs.push(Tab {
                    id,
                    kind: TabKind::Native {
                        tab_id: report.tab_id.clone(),
                    },
                    child_label: None,
                    last_url: None,
                    title: report.title.clone(),
                    streaming: report.streaming,
                    last_used: now,
                }),
            }
        }
        let active_is_native_or_none = self
            .active
            .as_deref()
            .and_then(|id| self.get(id))
            .is_none_or(Tab::is_native);
        if let Some(id) =
            active_native_id.filter(|id| active_is_native_or_none && listed.contains(id))
        {
            self.activate(&native_id(id), now);
        }
    }

    pub fn limit_unload_victim(
        &self,
        target_tab_id: &str,
        armed: &HashSet<String>,
    ) -> Option<String> {
        let live_others = self
            .tabs
            .iter()
            .filter(|t| t.is_live_provider() && t.id != target_tab_id);
        if live_others.clone().count() < MAX_LIVE_PROVIDER_TABS {
            return None;
        }
        live_others
            .filter(|t| self.active.as_deref() != Some(&t.id) && !armed.contains(&t.id))
            .min_by_key(|t| t.last_used)
            .map(|t| t.id.clone())
    }

    pub fn idle_unload_candidates(&self, now: Instant, armed: &HashSet<String>) -> Vec<String> {
        self.tabs
            .iter()
            .filter(|t| {
                t.is_live_provider()
                    && self.active.as_deref() != Some(&t.id)
                    && !armed.contains(&t.id)
                    && now.saturating_duration_since(t.last_used) >= UNLOAD_IDLE
            })
            .map(|t| t.id.clone())
            .collect()
    }

    pub fn live_provider_tabs(&self) -> Vec<String> {
        self.tabs
            .iter()
            .filter(|t| t.is_live_provider())
            .map(|t| t.id.clone())
            .collect()
    }

    pub fn snapshot(&self) -> TabsSnapshot {
        let mut order: Vec<usize> = (0..self.tabs.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.tabs[i].last_used));
        let mut recency = vec![0u32; self.tabs.len()];
        for (rank, &i) in order.iter().enumerate() {
            recency[i] = rank as u32;
        }
        let tabs = self
            .tabs
            .iter()
            .zip(recency)
            .map(|(t, recency)| {
                let (kind, provider_id, native_tab_id) = match &t.kind {
                    TabKind::Native { tab_id } => ("native", None, Some(tab_id.clone())),
                    TabKind::Provider { provider } => ("provider", Some(provider.id.clone()), None),
                };
                TabDto {
                    id: t.id.clone(),
                    kind,
                    provider_id,
                    native_tab_id,
                    title: t.title.clone(),
                    live: t.is_native() || t.child_label.is_some(),
                    streaming: t.is_native() && t.streaming,
                    recency,
                }
            })
            .collect();
        TabsSnapshot {
            tabs,
            active_id: self.active.clone(),
        }
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

    fn secs(base: Instant, s: u64) -> Instant {
        base + Duration::from_secs(s)
    }

    fn report(id: &str, title: &str, listed: bool) -> NativeTabReport {
        NativeTabReport {
            tab_id: id.into(),
            title: title.into(),
            streaming: false,
            listed,
        }
    }

    fn list_of(n: usize, base: Instant) -> (TabList, Vec<String>) {
        let mut list = TabList::default();
        let ids = (0..n)
            .map(|_| list.push_provider(&provider("claude"), base))
            .collect();
        (list, ids)
    }

    fn order(list: &TabList) -> Vec<String> {
        list.tabs().iter().map(|t| t.id.clone()).collect()
    }

    fn live_list(n: usize, base: Instant) -> (TabList, Vec<String>) {
        let (mut list, ids) = list_of(n, base);
        for id in &ids {
            let label = list.next_child_label("claude");
            list.set_live(id, Some(label));
        }
        (list, ids)
    }

    #[test]
    fn push_appends_in_order_not_live_not_active() {
        let base = Instant::now();
        let (list, ids) = list_of(3, base);
        assert_eq!(ids, vec!["provider:1", "provider:2", "provider:3"]);
        assert_eq!(order(&list), ids);
        assert!(list.active_id().is_none());
        assert!(list.tabs().iter().all(|t| t.child_label.is_none()));
    }

    #[test]
    fn activate_stamps_previous_and_new() {
        let base = Instant::now();
        let (mut list, ids) = list_of(2, base);
        list.activate(&ids[0], secs(base, 10));
        list.activate(&ids[1], secs(base, 20));
        assert_eq!(list.get(&ids[0]).unwrap().last_used, secs(base, 20));
        assert_eq!(list.get(&ids[1]).unwrap().last_used, secs(base, 20));
        assert_eq!(list.active_id(), Some(ids[1].as_str()));
    }

    #[test]
    fn set_native_view_deactivates_and_stamps() {
        let base = Instant::now();
        let (mut list, ids) = list_of(1, base);
        list.activate(&ids[0], secs(base, 10));
        list.set_native_view(secs(base, 30));
        assert!(list.active_id().is_none());
        assert_eq!(list.get(&ids[0]).unwrap().last_used, secs(base, 30));
    }

    #[test]
    fn close_active_middle_activates_right_neighbour() {
        let base = Instant::now();
        let (mut list, ids) = list_of(3, base);
        list.activate(&ids[1], base);
        let out = list.close(&ids[1], secs(base, 5));
        assert_eq!(out.successor.as_deref(), Some(ids[2].as_str()));
        assert!(!out.list_empty);
        assert_eq!(list.active_id(), Some(ids[2].as_str()));
        assert_eq!(list.get(&ids[2]).unwrap().last_used, secs(base, 5));
    }

    #[test]
    fn close_active_last_activates_left_neighbour() {
        let base = Instant::now();
        let (mut list, ids) = list_of(3, base);
        list.activate(&ids[2], base);
        let out = list.close(&ids[2], base);
        assert_eq!(out.successor.as_deref(), Some(ids[1].as_str()));
        assert_eq!(list.active_id(), Some(ids[1].as_str()));
    }

    #[test]
    fn close_inactive_keeps_active() {
        let base = Instant::now();
        let (mut list, ids) = list_of(3, base);
        list.activate(&ids[0], base);
        let out = list.close(&ids[2], base);
        assert_eq!(
            out,
            CloseOutcome {
                successor: None,
                list_empty: false
            }
        );
        assert_eq!(list.active_id(), Some(ids[0].as_str()));
    }

    #[test]
    fn close_last_remaining_reports_empty() {
        let base = Instant::now();
        let (mut list, ids) = list_of(1, base);
        list.activate(&ids[0], base);
        let out = list.close(&ids[0], base);
        assert_eq!(
            out,
            CloseOutcome {
                successor: None,
                list_empty: true
            }
        );
        assert!(list.active_id().is_none());
    }

    #[test]
    fn adjacent_wraps_both_ways() {
        let base = Instant::now();
        let (mut list, ids) = list_of(3, base);
        list.activate(&ids[2], base);
        assert_eq!(list.adjacent(1), Some(ids[0].clone()));
        list.activate(&ids[0], base);
        assert_eq!(list.adjacent(-1), Some(ids[2].clone()));
        list.set_native_view(base);
        assert_eq!(list.adjacent(1), Some(ids[0].clone()));
        assert_eq!(list.adjacent(-1), Some(ids[2].clone()));
    }

    #[test]
    fn at_position_rules() {
        let base = Instant::now();
        let (nine, n) = list_of(9, base);
        assert_eq!(nine.at_position(1), Some(n[0].clone()));
        assert_eq!(nine.at_position(8), Some(n[7].clone()));
        assert_eq!(nine.at_position(0), None);
        assert_eq!(nine.at_position(10), None);
        let (three, t) = list_of(3, base);
        assert_eq!(three.at_position(9), Some(t[2].clone()));
        assert_eq!(three.at_position(5), None);
        let (ten, x) = list_of(10, base);
        assert_eq!(ten.at_position(9), Some(x[9].clone()));
    }

    #[test]
    fn native_snapshot_updates_drops_orders_and_appends() {
        let base = Instant::now();
        let mut list = TabList::default();
        list.apply_native_snapshot(
            &[
                report("a", "A", true),
                report("b", "B", true),
                report("c", "C", true),
            ],
            None,
            base,
        );
        let p = list.push_provider(&provider("claude"), base);
        list.apply_native_snapshot(
            &[
                report("a", "A2", true),
                report("b", "B", false),
                report("d", "D", true),
            ],
            None,
            base,
        );
        assert_eq!(order(&list), vec!["native:a", &p, "native:d"]);
        assert_eq!(list.get("native:a").unwrap().title, "A2");
    }

    #[test]
    fn native_snapshot_active_rules() {
        let base = Instant::now();
        let mut list = TabList::default();
        let reports = [report("a", "A", true), report("b", "B", true)];
        list.apply_native_snapshot(&reports, Some("a"), base);
        assert_eq!(list.active_id(), Some("native:a"));
        list.apply_native_snapshot(&reports, Some("b"), base);
        assert_eq!(list.active_id(), Some("native:b"));
        let p = list.push_provider(&provider("claude"), base);
        list.activate(&p, base);
        list.apply_native_snapshot(&reports, Some("a"), base);
        assert_eq!(list.active_id(), Some(p.as_str()));
    }

    #[test]
    fn native_snapshot_removing_active_native_clears_active() {
        let base = Instant::now();
        let mut list = TabList::default();
        list.apply_native_snapshot(&[report("a", "A", true)], Some("a"), base);
        list.apply_native_snapshot(&[report("a", "A", false)], None, base);
        assert!(list.active_id().is_none());
        assert!(list.tabs().is_empty());
    }

    #[test]
    fn live_limit_picks_lru_non_active_non_armed() {
        let base = Instant::now();
        let (mut list, ids) = live_list(3, base);
        list.activate(&ids[0], secs(base, 10));
        list.activate(&ids[1], secs(base, 20));
        list.activate(&ids[2], secs(base, 30));
        let fourth = list.push_provider(&provider("chatgpt"), base);
        let none = HashSet::new();
        assert_eq!(
            list.limit_unload_victim(&fourth, &none),
            Some(ids[0].clone())
        );
        let armed: HashSet<String> = [ids[0].clone()].into();
        assert_eq!(
            list.limit_unload_victim(&fourth, &armed),
            Some(ids[1].clone())
        );
        let armed: HashSet<String> = [ids[0].clone(), ids[1].clone()].into();
        assert_eq!(list.limit_unload_victim(&fourth, &armed), None);
    }

    #[test]
    fn live_limit_never_picks_active_and_ignores_natives() {
        let base = Instant::now();
        let (mut list, ids) = live_list(3, base);
        list.apply_native_snapshot(
            &[report("a", "A", true), report("b", "B", true)],
            None,
            base,
        );
        let fourth = list.push_provider(&provider("chatgpt"), base);
        list.activate(&ids[0], secs(base, 5));
        let armed: HashSet<String> = [ids[1].clone(), ids[2].clone()].into();
        assert_eq!(list.limit_unload_victim(&fourth, &armed), None);

        let (mut small, _) = live_list(2, base);
        small.apply_native_snapshot(
            &[report("a", "A", true), report("b", "B", true)],
            None,
            base,
        );
        let third = small.push_provider(&provider("chatgpt"), base);
        assert_eq!(small.limit_unload_victim(&third, &HashSet::new()), None);
    }

    #[test]
    fn idle_unload_rules() {
        let base = Instant::now();
        let (mut list, ids) = live_list(4, base);
        let now = secs(base, 40 * 60);
        list.activate(&ids[1], base);
        list.activate(&ids[2], secs(base, 39 * 60));
        list.activate(&ids[3], now);
        let armed: HashSet<String> = HashSet::new();
        assert_eq!(
            list.idle_unload_candidates(secs(base, 31 * 60), &armed),
            vec![ids[0].clone()]
        );
        assert_eq!(
            list.idle_unload_candidates(now, &armed),
            vec![ids[0].clone()]
        );
        let armed: HashSet<String> = [ids[0].clone()].into();
        assert!(list.idle_unload_candidates(now, &armed).is_empty());
        assert_eq!(list.live_provider_tabs(), ids);
    }

    #[test]
    fn child_labels_are_unique_and_map_back_to_tab() {
        let base = Instant::now();
        let mut list = TabList::default();
        let first = list.next_child_label("claude");
        let second = list.next_child_label("claude");
        let other = list.next_child_label("chatgpt");
        assert_ne!(first, second);
        assert_eq!(first, "conversation-dialog::ai-webview-claude-1");
        assert_eq!(other, "conversation-dialog::ai-webview-chatgpt-1");
        let id = list.push_provider(&provider("my-llm"), base);
        let label = list.next_child_label("my-llm");
        list.set_live(&id, Some(label.clone()));
        let tab = list.by_child_label(&label).unwrap();
        assert_eq!(tab.id, id);
        match &tab.kind {
            TabKind::Provider { provider } => assert_eq!(provider.id, "my-llm"),
            TabKind::Native { .. } => panic!("expected provider tab"),
        }
    }

    #[test]
    fn snapshot_recency_and_flags() {
        let base = Instant::now();
        let (mut list, ids) = list_of(2, base);
        list.apply_native_snapshot(
            &[NativeTabReport {
                streaming: true,
                ..report("a", "A", true)
            }],
            None,
            base,
        );
        list.activate(&ids[0], secs(base, 10));
        let snap = list.snapshot();
        assert_eq!(snap.active_id.as_deref(), Some(ids[0].as_str()));
        let by_id = |id: &str| snap.tabs.iter().find(|t| t.id == id).unwrap();
        assert_eq!(by_id(&ids[0]).recency, 0);
        assert!(by_id("native:a").live && by_id("native:a").streaming);
        assert!(!by_id(&ids[1]).live && !by_id(&ids[1]).streaming);
        assert_eq!(by_id("native:a").native_tab_id.as_deref(), Some("a"));
        assert_eq!(by_id(&ids[1]).provider_id.as_deref(), Some("claude"));
    }
}
