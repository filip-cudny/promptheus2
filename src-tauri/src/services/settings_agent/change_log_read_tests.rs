use serde_json::{json, Value};

use super::change_log_read::{read, Filters};
use crate::services::change_log::{
    ChangeLogEntry, Effect, EntryKind, LoggedOperation, SecretChange, Target,
};

const CARD_ID: &str = "20261106T100000Z-laptop-aaaaaaaa";
const SECRET_ID: &str = "20261106T100500Z-desktop-bbbbbbbb";
const SECOND_CARD_ID: &str = "20261106T101000Z-laptop-cccccccc";
const UNDO_ID: &str = "20261106T101500Z-laptop-dddddddd";
const SKILL_BODY: &str = "---\nname: s\n---\n\nSkill body.\n";

fn setting(path: &str, before: Option<Value>, after: Option<Value>) -> Effect {
    Effect { target: Target::Setting { path: path.into() }, before, after }
}

fn operation(label: &str, op: &str, effects: Vec<Effect>) -> LoggedOperation {
    LoggedOperation { label: label.into(), op: op.into(), effects, test: None }
}

fn entry(id: &str, created_at: &str, host: &str, kind: EntryKind) -> ChangeLogEntry {
    ChangeLogEntry {
        id: id.into(),
        created_at: created_at.into(),
        host: host.into(),
        kind,
        request: None,
        reply: None,
        operations: vec![],
        undoes: None,
        secret: None,
    }
}

fn fixture() -> Vec<ChangeLogEntry> {
    let card = ChangeLogEntry {
        request: Some("rename the model".into()),
        reply: Some("Renamed.".into()),
        operations: vec![
            operation(
                "Rename model",
                "set_setting",
                vec![setting("models[a].display_name", Some(json!("A")), Some(json!("A2")))],
            ),
            operation(
                "Create skill s",
                "create_skill",
                vec![Effect {
                    target: Target::File { path: "skills/s/SKILL.md".into() },
                    before: None,
                    after: Some(json!(SKILL_BODY)),
                }],
            ),
        ],
        ..entry(CARD_ID, "2026-11-06T10:00:00Z", "laptop", EntryKind::Card)
    };
    let secret = ChangeLogEntry {
        secret: Some(SecretChange {
            name: "OPENAI_KEY".into(),
            backup: Some("backup-file.env".into()),
            had_previous: true,
        }),
        ..entry(SECRET_ID, "2026-11-06T10:05:00Z", "desktop", EntryKind::Secret)
    };
    let add_var = operation(
        "Add variable X",
        "add_env_var",
        vec![setting("env_vars[X]", None, Some(json!({"name": "X"})))],
    );
    let second = ChangeLogEntry {
        request: Some("add a variable".into()),
        operations: vec![add_var],
        ..entry(SECOND_CARD_ID, "2026-11-06T10:10:00Z", "laptop", EntryKind::Card)
    };
    let undo = ChangeLogEntry {
        undoes: Some(SECOND_CARD_ID.into()),
        operations: vec![operation(
            "Add variable X",
            "add_env_var",
            vec![setting("env_vars[X]", Some(json!({"name": "X"})), None)],
        )],
        ..entry(UNDO_ID, "2026-11-06T10:15:00Z", "laptop", EntryKind::Undo)
    };
    vec![card, secret, second, undo]
}

fn list(filters: Filters) -> Value {
    read(&fixture(), &filters).unwrap()
}

fn ids(result: &Value) -> Vec<&str> {
    result["entries"].as_array().unwrap().iter().map(|e| e["id"].as_str().unwrap()).collect()
}

fn find<'a>(result: &'a Value, id: &str) -> &'a Value {
    result["entries"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap()
}

#[test]
fn the_list_is_newest_first_without_a_hint_when_everything_fits() {
    let result = list(Filters::default());
    assert_eq!(ids(&result), [UNDO_ID, SECOND_CARD_ID, SECRET_ID, CARD_ID]);
    assert_eq!(result["more"], 0);
    assert!(result.get("hint").is_none());
}

#[test]
fn key_models_a_returns_the_first_card_only() {
    let result = list(Filters { key: Some("models[a]".into()), ..Filters::default() });
    assert_eq!(ids(&result), [CARD_ID]);
}

#[test]
fn limit_returns_the_newest_with_a_count_and_a_hint_naming_its_time() {
    let result = list(Filters { limit: Some(1), ..Filters::default() });
    assert_eq!(ids(&result), [UNDO_ID]);
    assert_eq!(result["more"], 3);
    assert!(result["hint"].as_str().unwrap().contains("2026-11-06T10:15:00Z"), "{result}");
}

#[test]
fn file_effects_carry_content_only_when_read_by_name() {
    let listed = list(Filters::default());
    let file_effect = &find(&listed, CARD_ID)["operations"][1]["effects"][0];
    assert_eq!(file_effect, &json!({"target": "file", "path": "skills/s/SKILL.md"}));

    let whole = read(&fixture(), &Filters { name: Some(CARD_ID.into()), ..Filters::default() }).unwrap();
    let file_effect = &whole["operations"][1]["effects"][0];
    assert_eq!(file_effect["after"], SKILL_BODY);
    assert_eq!(file_effect["before"], Value::Null);
    assert_eq!(whole["operations"][0]["effects"][0]["after"], "A2");
}

#[test]
fn an_unknown_name_is_an_error() {
    let error = read(&fixture(), &Filters { name: Some("nope".into()), ..Filters::default() }).unwrap_err();
    assert_eq!(error, "change log entry 'nope' not found");
}

#[test]
fn the_secret_entry_has_its_name_and_no_backup() {
    let result = list(Filters::default());
    let secret = find(&result, SECRET_ID);
    assert_eq!(secret["secret"], json!({"name": "OPENAI_KEY"}));
    assert!(!result.to_string().contains("backup"));
}

#[test]
fn an_undone_card_has_state_undone() {
    let result = list(Filters::default());
    assert_eq!(find(&result, SECOND_CARD_ID)["state"], "undone");
    assert_eq!(find(&result, UNDO_ID)["state"], "applied");
    assert_eq!(find(&result, CARD_ID)["state"], "applied");
    assert_eq!(find(&result, UNDO_ID)["undoes"], SECOND_CARD_ID);
}

#[test]
fn since_keeps_entries_from_that_time() {
    let result = list(Filters { since: Some("2026-11-06T10:10:00Z".into()), ..Filters::default() });
    assert_eq!(ids(&result), [UNDO_ID, SECOND_CARD_ID]);
}

#[test]
fn until_keeps_entries_up_to_that_time() {
    let result = list(Filters { until: Some("2026-11-06T11:05:00+01:00".into()), ..Filters::default() });
    assert_eq!(ids(&result), [SECRET_ID, CARD_ID]);
}

#[test]
fn an_unparsable_time_is_an_error_naming_the_field() {
    let error = read(&fixture(), &Filters { since: Some("yesterday".into()), ..Filters::default() }).unwrap_err();
    assert!(error.starts_with("`since`"), "{error}");
    let error = read(&fixture(), &Filters { until: Some("yesterday".into()), ..Filters::default() }).unwrap_err();
    assert!(error.starts_with("`until`"), "{error}");
}

#[test]
fn op_keeps_entries_with_that_operation() {
    let result = list(Filters { op: Some("create_skill".into()), ..Filters::default() });
    assert_eq!(ids(&result), [CARD_ID]);
}

#[test]
fn text_matches_request_reply_or_label_ignoring_case() {
    let by_request = list(Filters { text: Some("ADD A VARIABLE".into()), ..Filters::default() });
    assert_eq!(ids(&by_request), [SECOND_CARD_ID]);
    let by_reply = list(Filters { text: Some("renamed.".into()), ..Filters::default() });
    assert_eq!(ids(&by_reply), [CARD_ID]);
    let by_label = list(Filters { text: Some("variable x".into()), ..Filters::default() });
    assert_eq!(ids(&by_label), [UNDO_ID, SECOND_CARD_ID]);
}

#[test]
fn host_keeps_entries_of_that_host() {
    let result = list(Filters { host: Some("desktop".into()), ..Filters::default() });
    assert_eq!(ids(&result), [SECRET_ID]);
}

#[test]
fn limit_above_the_maximum_is_clamped_to_100() {
    let mut entries = Vec::new();
    for minute in 0..120 {
        let id = format!("e{minute:03}");
        let created = format!("2026-11-06T{:02}:{:02}:00Z", minute / 60, minute % 60);
        entries.push(entry(&id, &created, "h", EntryKind::Card));
    }
    let result = read(&entries, &Filters { limit: Some(500), ..Filters::default() }).unwrap();
    assert_eq!(result["entries"].as_array().unwrap().len(), 100);
    assert_eq!(result["more"], 20);
    let default = read(&entries, &Filters::default()).unwrap();
    assert_eq!(default["entries"].as_array().unwrap().len(), 20);
}

#[test]
fn literal_api_keys_are_redacted_at_any_depth_and_references_are_kept() {
    let removed = json!({"index": 0, "value": {"id": "a", "api_key": "sk-literal-1"}});
    let whole_list = json!([
        {"id": "a", "api_key": "sk-literal-2", "parameters": {"api_key": "sk-literal-3"}},
        {"id": "b", "api_key": "${OPENAI_KEY}"},
        {"id": "c", "api_key": ""}
    ]);
    let card = ChangeLogEntry {
        operations: vec![operation(
            "Edit models",
            "update_model",
            vec![
                setting("models[a]", Some(removed), None),
                setting("models", Some(whole_list), None),
                setting("models[b].api_key", Some(json!("sk-literal-4")), Some(json!("${NEW_KEY}"))),
            ],
        )],
        ..entry("x", "2026-11-06T10:00:00Z", "h", EntryKind::Card)
    };
    let entries = [card];
    for filters in [Filters::default(), Filters { name: Some("x".into()), ..Filters::default() }] {
        let text = read(&entries, &filters).unwrap().to_string();
        assert!(!text.contains("sk-literal"), "{text}");
        assert!(text.contains("${OPENAI_KEY}") && text.contains("${NEW_KEY}"), "{text}");
    }
    let whole = read(&entries, &Filters { name: Some("x".into()), ..Filters::default() }).unwrap();
    let effects = &whole["operations"][0]["effects"];
    assert_eq!(effects[0]["before"]["value"]["api_key"], "[redacted]");
    assert_eq!(effects[1]["before"][0]["parameters"]["api_key"], "[redacted]");
    assert_eq!(effects[1]["before"][2]["api_key"], "");
    assert_eq!(effects[2]["before"], "[redacted]");
}
