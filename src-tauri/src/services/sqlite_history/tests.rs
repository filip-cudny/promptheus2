use super::*;
use crate::services::database::Database;

fn make_db() -> Database {
    Database::open_in_memory().unwrap()
}

fn make_svc() -> SqliteHistoryService {
    SqliteHistoryService::new(make_db(), 0)
}

fn backdate(svc: &SqliteHistoryService, entry_id: &str, days: u32) {
    svc.conn()
        .execute(
            "UPDATE conversations
             SET created_at = datetime('now', 'localtime', ?2), updated_at = NULL
             WHERE id = ?1",
            rusqlite::params![entry_id, format!("-{} days", days)],
        )
        .unwrap();
}

fn add_plain(svc: &SqliteHistoryService, text: &str) -> String {
    svc.add_entry(
        text.into(),
        HistoryEntryType::Text,
        None,
        None,
        true,
        None,
        false,
        None,
        false,
    )
    .unwrap()
}

fn make_nodes(user_text: &str, assistant_text: &str) -> Vec<SerializedConversationNode> {
    vec![
        SerializedConversationNode {
            node_id: "u1".into(),
            parent_id: None,
            role: "user".into(),
            content: user_text.into(),
            timestamp: "2026-01-01".into(),
            children: vec!["a1".into()],
            updates: vec![],
            prompt_tokens: None,
            completion_tokens: None,
            thinking: None,
            thinking_duration: None,
            query_duration: None,
            error: None,
            cancelled: false,
            tool_calls: vec![],
            text_attachments: vec![],
            applied_skills: vec![],
        },
        SerializedConversationNode {
            node_id: "a1".into(),
            parent_id: Some("u1".into()),
            role: "assistant".into(),
            content: assistant_text.into(),
            timestamp: "2026-01-01".into(),
            children: vec![],
            updates: vec![],
            prompt_tokens: None,
            completion_tokens: None,
            thinking: None,
            thinking_duration: None,
            query_duration: None,
            error: None,
            cancelled: false,
            tool_calls: vec![],
            text_attachments: vec![],
            applied_skills: vec![],
        },
    ]
}

#[test]
fn add_and_get_history() {
    let svc = make_svc();
    svc.add_entry(
        "hello".into(),
        HistoryEntryType::Text,
        Some("world".into()),
        None,
        true,
        None,
        false,
        None,
        false,
    );
    let history = svc.get_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].input_content, "hello");
}

#[test]
fn add_conversation_and_restore() {
    let svc = make_svc();
    let nodes = make_nodes("hi", "hello");
    let id = svc.add_conversation_entry(
        "context".into(),
        None,
        None,
        true,
        None,
        nodes,
        Some("u1".into()),
        vec!["u1".into(), "a1".into()],
        false,
        None,
        vec![],
        None,
        None,
    );

    let entry = svc.get_entry_by_id(&id).unwrap();
    assert!(entry.is_multi_turn);
    let conv = entry.conversation_data.unwrap();
    assert_eq!(conv.nodes.len(), 2);
    assert_eq!(conv.root_node_id, Some("u1".into()));
    assert_eq!(conv.current_path, vec!["u1", "a1"]);
    assert_eq!(conv.context_text, "context");
}

#[test]
fn update_conversation_entry() {
    let svc = make_svc();
    let nodes = make_nodes("hi", "hello");
    let id = svc.add_conversation_entry(
        "ctx".into(),
        None,
        None,
        true,
        None,
        nodes,
        Some("u1".into()),
        vec!["u1".into(), "a1".into()],
        false,
        None,
        vec![],
        None,
        None,
    );

    let new_nodes = make_nodes("hi updated", "hello updated");
    svc.update_conversation_entry(
        &id,
        "new ctx".into(),
        new_nodes,
        Some("u1".into()),
        vec!["u1".into(), "a1".into()],
        vec![],
        None,
        None,
    )
    .unwrap();

    let entry = svc.get_entry_by_id(&id).unwrap();
    assert!(entry.updated_at.is_some());
    let conv = entry.conversation_data.unwrap();
    assert_eq!(conv.context_text, "new ctx");
    assert_eq!(conv.nodes[0].content, "hi updated");
}

#[test]
fn update_nonexistent_entry_fails() {
    let svc = make_svc();
    let result = svc.update_conversation_entry(
        "nonexistent",
        "ctx".into(),
        vec![],
        None,
        vec![],
        vec![],
        None,
        None,
    );
    assert!(result.is_err());
}

#[test]
fn retention_prunes_only_entries_past_cutoff() {
    let svc = SqliteHistoryService::new(make_db(), 30);
    let stale = add_plain(&svc, "stale");
    let borderline = add_plain(&svc, "borderline");
    add_plain(&svc, "fresh");
    backdate(&svc, &stale, 60);
    backdate(&svc, &borderline, 29);

    assert_eq!(svc.enforce_retention(), 1);
    assert_eq!(svc.entry_count(), 2);
    assert!(svc.get_entry_by_id(&stale).is_none());
    assert!(svc.get_entry_by_id(&borderline).is_some());
}

#[test]
fn retention_disabled_keeps_everything() {
    let svc = SqliteHistoryService::new(make_db(), 0);
    let ancient = add_plain(&svc, "ancient");
    backdate(&svc, &ancient, 3000);

    assert_eq!(svc.enforce_retention(), 0);
    assert_eq!(svc.entry_count(), 1);
}

#[test]
fn retention_respects_updated_at_over_created_at() {
    let svc = SqliteHistoryService::new(make_db(), 30);
    let touched = add_plain(&svc, "touched");
    svc.conn()
        .execute(
            "UPDATE conversations
             SET created_at = datetime('now', 'localtime', '-90 days'),
                 updated_at = datetime('now', 'localtime', '-1 days')
             WHERE id = ?1",
            rusqlite::params![touched],
        )
        .unwrap();

    assert_eq!(svc.enforce_retention(), 0);
    assert!(svc.get_entry_by_id(&touched).is_some());
}

#[test]
fn adding_an_entry_does_not_trigger_a_sweep() {
    let mut svc = SqliteHistoryService::new(make_db(), 30);
    let stale = add_plain(&svc, "stale");
    backdate(&svc, &stale, 60);

    add_plain(&svc, "fresh");
    assert_eq!(svc.entry_count(), 2);

    svc.set_retention_days(30);
    assert_eq!(svc.enforce_retention(), 1);
    assert_eq!(svc.entry_count(), 1);
}

#[test]
fn get_entry_by_id_found_and_not_found() {
    let svc = make_svc();
    svc.add_entry(
        "test".into(),
        HistoryEntryType::Text,
        None,
        None,
        true,
        None,
        false,
        None,
        false,
    );
    let entry = svc.get_history().into_iter().next().unwrap();
    assert!(svc.get_entry_by_id(&entry.id).is_some());
    assert!(svc.get_entry_by_id("nonexistent").is_none());
}

#[test]
fn get_last_item_by_type() {
    let svc = make_svc();
    svc.add_entry("t1".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
    svc.add_entry("s1".into(), HistoryEntryType::Speech, None, None, true, None, false, None, false);
    svc.add_entry("t2".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);

    let last_text = svc.get_last_item_by_type(HistoryEntryType::Text).unwrap();
    assert_eq!(last_text.input_content, "t2");

    let last_speech = svc.get_last_item_by_type(HistoryEntryType::Speech).unwrap();
    assert_eq!(last_speech.input_content, "s1");
}

#[test]
fn update_title() {
    let svc = make_svc();
    svc.add_entry("test".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
    let entry = svc.get_history().into_iter().next().unwrap();

    svc.update_entry_title(&entry.id, "My Title".into()).unwrap();
    let updated = svc.get_entry_by_id(&entry.id).unwrap();
    assert_eq!(updated.title, Some("My Title".into()));
}

#[test]
fn clear_removes_all() {
    let svc = make_svc();
    svc.add_entry("test".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
    assert_eq!(svc.entry_count(), 1);
    svc.clear();
    assert_eq!(svc.entry_count(), 0);
}

#[test]
fn image_round_trip() {
    let svc = make_svc();
    let nodes = make_nodes("hi", "hello");
    let image_data = BASE64.encode(b"fake png data");

    let id = svc.add_conversation_entry(
        "ctx".into(),
        None,
        None,
        true,
        None,
        nodes,
        Some("u1".into()),
        vec!["u1".into(), "a1".into()],
        false,
        None,
        vec![
            ImagePayload {
                node_id: Some("u1".into()),
                image_index: 0,
                data: image_data.clone(),
                media_type: "image/png".into(),
            },
            ImagePayload {
                node_id: None,
                image_index: 0,
                data: image_data.clone(),
                media_type: "image/jpeg".into(),
            },
        ],
        None,
        None,
    );

    let entry = svc.get_entry_by_id(&id).unwrap();
    let conv = entry.conversation_data.unwrap();

    let node_imgs = conv.node_images.get("u1").unwrap();
    assert_eq!(node_imgs.len(), 1);
    assert_eq!(node_imgs[0].data, image_data);
    assert_eq!(node_imgs[0].media_type, "image/png");

    assert_eq!(conv.context_images.len(), 1);
    assert_eq!(conv.context_images[0].data, image_data);
    assert_eq!(conv.context_images[0].media_type, "image/jpeg");
}

#[test]
fn image_cascade_delete() {
    let svc = make_svc();
    let nodes = make_nodes("hi", "hello");
    let image_data = BASE64.encode(b"data");

    svc.add_conversation_entry(
        "ctx".into(),
        None,
        None,
        true,
        None,
        nodes,
        Some("u1".into()),
        vec!["u1".into(), "a1".into()],
        false,
        None,
        vec![ImagePayload {
            node_id: Some("u1".into()),
            image_index: 0,
            data: image_data,
            media_type: "image/png".into(),
        }],
        None,
        None,
    );

    svc.clear();

    let count: i64 = svc
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM conversation_images", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn quick_action_query() {
    let svc = make_svc();
    svc.add_entry("normal".into(), HistoryEntryType::Text, None, None, true, None, false, None, false);
    svc.add_entry("quick".into(), HistoryEntryType::Text, None, None, true, None, false, None, true);

    let last_quick = svc.get_last_quick_action(HistoryEntryType::Text).unwrap();
    assert_eq!(last_quick.input_content, "quick");
}

#[test]
fn vacuum_reclaims_free_pages_after_bulk_delete() {
    let svc = SqliteHistoryService::new(make_db(), 0);
    for i in 0..200 {
        add_plain(&svc, &format!("{}-{}", i, "x".repeat(4096)));
    }
    let filled = svc.storage_stats();
    svc.clear();

    let before = svc.storage_stats();
    assert!(before.reclaimable_bytes > 0);
    assert_eq!(svc.entry_count(), 0);

    let after = svc.vacuum().unwrap();
    assert_eq!(after.reclaimable_bytes, 0);
    assert!(after.database_bytes < filled.database_bytes);
}

#[test]
fn compact_after_bulk_delete_skips_small_databases() {
    let svc = SqliteHistoryService::new(make_db(), 0);
    let stale = add_plain(&svc, "stale");
    backdate(&svc, &stale, 60);
    svc.clear();

    assert!(!svc.compact_after_bulk_delete());
}
