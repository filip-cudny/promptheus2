use super::*;
use crate::services::database::Database;

fn make_db() -> Database {
    Database::open_in_memory().unwrap()
}

fn make_svc() -> SqliteHistoryService {
    SqliteHistoryService::new(make_db(), 1000)
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
fn max_entries_enforcement() {
    let svc = SqliteHistoryService::new(make_db(), 3);
    for i in 0..5 {
        svc.add_entry(
            format!("entry-{}", i),
            HistoryEntryType::Text,
            None,
            None,
            true,
            None,
            false,
            None,
            false,
        );
    }
    assert_eq!(svc.entry_count(), 3);
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
