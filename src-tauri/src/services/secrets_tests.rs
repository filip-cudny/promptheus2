use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use super::env_resolve::resolve_env_refs;
use super::secrets::*;
use super::ai::AiService;
use super::change_log::{change_log_dir, load_all, secret_backup_dir, ChangeLogEntry, EntryKind};
use crate::models::settings::{ModelConfig, Settings};

const HOST: &str = "laptop";

fn config_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

fn model(id: &str, api_key: &str) -> ModelConfig {
    serde_json::from_value(serde_json::json!({
        "id": id, "model": "gpt", "display_name": "M", "api_key": api_key
    }))
    .unwrap()
}

fn read_entry(dir: &TempDir, id: &str) -> ChangeLogEntry {
    load_all(&change_log_dir(dir.path()))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap()
}

fn env_file_text(dir: &TempDir) -> String {
    std::fs::read_to_string(dir.path().join(".env")).unwrap()
}

#[test]
fn save_writes_quoted_line_and_keeps_other_lines() {
    let dir = config_dir();
    std::fs::write(dir.path().join(".env"), "# note\nOTHER=1\nexport SEC_QUOTE=old\nLAST=2\n").unwrap();
    let value = "a\"b\\c\nd$e";

    save_secret(dir.path(), HOST, "SEC_QUOTE", value).unwrap();

    let text = env_file_text(&dir);
    assert!(text.contains("# note\nOTHER=1\nSEC_QUOTE=\""));
    assert!(text.ends_with("LAST=2\n") || text.contains("\nLAST=2\n"));
    assert_eq!(text.matches("SEC_QUOTE").count(), 1);
    let read: Vec<_> = dotenvy::from_path_iter(dir.path().join(".env"))
        .unwrap()
        .flatten()
        .collect();
    assert!(read.contains(&("SEC_QUOTE".to_string(), value.to_string())));
    assert!(read.contains(&("OTHER".to_string(), "1".to_string())));
    assert!(read.contains(&("LAST".to_string(), "2".to_string())));
}

#[test]
fn save_sets_override_without_touching_process_env() {
    let dir = config_dir();
    save_secret(dir.path(), HOST, "SEC_OVERRIDE", "v1").unwrap();
    assert_eq!(resolve_env_refs("${SEC_OVERRIDE}"), "v1");
    assert!(std::env::var("SEC_OVERRIDE").is_err());
}

#[test]
fn saved_secret_makes_model_available() {
    let dir = config_dir();
    let models = [model("m-sec", "${SEC_MODEL_KEY}")];
    assert!(!AiService::new(&models).has_model("m-sec"));
    save_secret(dir.path(), HOST, "SEC_MODEL_KEY", "sk-test").unwrap();
    assert!(AiService::new(&models).has_model("m-sec"));
}

#[test]
fn new_env_file_is_private_and_invalid_names_are_rejected() {
    let dir = config_dir();
    save_secret(dir.path(), HOST, "SEC_MODE", "v").unwrap();
    let mode = std::fs::metadata(dir.path().join(".env")).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert!(matches!(
        save_secret(dir.path(), HOST, "1BAD", "v"),
        Err(SecretError::InvalidName(_))
    ));
    assert!(validate_name("_ok1") && !validate_name("a-b") && !validate_name(""));
}

#[test]
fn backup_holds_previous_value_and_entry_holds_neither() {
    let dir = config_dir();
    save_secret(dir.path(), HOST, "SEC_BACKUP", "first-value").unwrap();
    let id = save_secret(dir.path(), HOST, "SEC_BACKUP", "second-value").unwrap();

    let entry = read_entry(&dir, &id);
    let secret = entry.secret.clone().unwrap();
    assert_eq!(entry.kind, EntryKind::Secret);
    assert!(secret.had_previous);
    let backup = secret_backup_dir(dir.path()).join(secret.backup.unwrap());
    let backup_text = std::fs::read_to_string(&backup).unwrap();
    assert_eq!(backup_text, "SEC_BACKUP=\"first-value\"\n");
    assert_eq!(std::fs::metadata(&backup).unwrap().permissions().mode() & 0o777, 0o600);
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("SEC_BACKUP"));
    assert!(!json.contains("first-value") && !json.contains("second-value"));
}

#[test]
fn undo_restores_previous_value() {
    let dir = config_dir();
    save_secret(dir.path(), HOST, "SEC_UNDO", "first").unwrap();
    let id = save_secret(dir.path(), HOST, "SEC_UNDO", "second").unwrap();

    let outcome = undo_secret(dir.path(), HOST, &read_entry(&dir, &id)).unwrap();

    let UndoOutcome::Undone { entry_id } = outcome else { panic!("expected Undone") };
    assert_eq!(resolve_env_refs("${SEC_UNDO}"), "first");
    assert_eq!(current_value(dir.path(), "SEC_UNDO").as_deref(), Some("first"));
    let undo = read_entry(&dir, &entry_id);
    assert_eq!(undo.kind, EntryKind::Undo);
    assert_eq!(undo.undoes.as_deref(), Some(id.as_str()));
    assert!(undo.secret.unwrap().backup.is_some());
}

#[test]
fn undo_of_first_secret_removes_it() {
    let dir = config_dir();
    std::fs::write(dir.path().join(".env"), "KEEP=1\n").unwrap();
    let id = save_secret(dir.path(), HOST, "SEC_FIRST", "only").unwrap();

    undo_secret(dir.path(), HOST, &read_entry(&dir, &id)).unwrap();

    assert_eq!(env_file_text(&dir), "KEEP=1\n");
    assert_eq!(current_value(dir.path(), "SEC_FIRST"), None);
    assert_eq!(resolve_env_refs("${SEC_FIRST}"), "");
}

#[test]
fn undo_without_backup_is_skipped_and_changes_nothing() {
    let dir = config_dir();
    save_secret(dir.path(), HOST, "SEC_SKIP", "first").unwrap();
    let id = save_secret(dir.path(), "desktop", "SEC_SKIP", "second").unwrap();
    let entry = read_entry(&dir, &id);
    std::fs::remove_dir_all(secret_backup_dir(dir.path())).unwrap();
    let before = env_file_text(&dir);
    let entries_before = load_all(&change_log_dir(dir.path())).len();

    let outcome = undo_secret(dir.path(), HOST, &entry).unwrap();

    assert_eq!(outcome, UndoOutcome::Skipped { held_by: "desktop".into() });
    assert_eq!(env_file_text(&dir), before);
    assert_eq!(current_value(dir.path(), "SEC_SKIP").as_deref(), Some("second"));
    assert_eq!(load_all(&change_log_dir(dir.path())).len(), entries_before);
}

#[test]
fn redactor_removes_every_source_and_keeps_names() {
    let dir = config_dir();
    std::fs::write(dir.path().join(".env"), "SEC_RED_FILE=\"file-secret-1\"\n").unwrap();
    save_secret(dir.path(), HOST, "SEC_RED_REF", "ref-secret-2").unwrap();
    let mut settings = Settings::default();
    settings.models.push(model("a", "${SEC_RED_REF}"));
    settings.models.push(model("b", "literal-secret-3"));
    settings.prompt_base.about_you = "mine is file-secret-1".into();
    let redactor = Redactor::new(dir.path(), &settings);

    let text = redactor.redact("file-secret-1 ref-secret-2 literal-secret-3 ${SEC_RED_REF}");
    assert!(!text.contains("secret-1") && !text.contains("secret-2") && !text.contains("secret-3"));
    assert!(text.contains("${SEC_RED_REF}"));

    let json = redactor.redact_settings(&settings).to_string();
    assert!(!json.contains("file-secret-1"));
    assert!(!json.contains("ref-secret-2"));
    assert!(!json.contains("literal-secret-3"));
    assert!(json.contains("${SEC_RED_REF}"));
}

fn env_var(name: &str, renamed_from: &[&str]) -> crate::models::settings::EnvVar {
    crate::models::settings::EnvVar {
        name: name.to_string(),
        description: String::new(),
        secret: true,
        renamed_from: renamed_from.iter().map(|s| s.to_string()).collect(),
    }
}

fn settings_with(vars: Vec<crate::models::settings::EnvVar>) -> Settings {
    let mut settings = Settings::default();
    settings.env_vars = vars;
    settings
}

#[test]
fn copy_fills_missing_name_from_renamed_from_and_keeps_old_key() {
    let dir = config_dir();
    std::fs::write(dir.path().join(".env"), "CPY_OLD=\"v1\"\n").unwrap();
    let settings = settings_with(vec![env_var("CPY_NEW", &["CPY_OLD"])]);

    assert_eq!(copy_renamed_values(dir.path(), &settings), vec!["CPY_NEW".to_string()]);

    assert_eq!(current_value(dir.path(), "CPY_NEW").as_deref(), Some("v1"));
    assert_eq!(super::env_resolve::env_value("CPY_NEW").as_deref(), Some("v1"));
    assert!(env_file_text(&dir).contains("CPY_OLD=\"v1\"\n"));
}

#[test]
fn copy_leaves_an_already_set_name_untouched() {
    let dir = config_dir();
    let original = "CPY_SET_OLD=\"v1\"\nCPY_SET_NEW=\"v2\"\n";
    std::fs::write(dir.path().join(".env"), original).unwrap();
    let settings = settings_with(vec![env_var("CPY_SET_NEW", &["CPY_SET_OLD"])]);

    assert!(copy_renamed_values(dir.path(), &settings).is_empty());

    assert_eq!(current_value(dir.path(), "CPY_SET_NEW").as_deref(), Some("v2"));
    assert_eq!(env_file_text(&dir), original);
}

#[test]
fn copy_takes_the_first_listed_old_name_that_has_a_value() {
    let dir = config_dir();
    std::fs::write(dir.path().join(".env"), "CPY_ORD_A=\"cpy-ord-a-value\"\n").unwrap();
    let settings = settings_with(vec![env_var("CPY_ORD_NEW", &["CPY_ORD_B", "CPY_ORD_A"])]);
    copy_renamed_values(dir.path(), &settings);
    assert_eq!(current_value(dir.path(), "CPY_ORD_NEW").as_deref(), Some("cpy-ord-a-value"));

    let both = config_dir();
    std::fs::write(both.path().join(".env"), "CPY_ORD2_A=\"cpy-ord2-a-value\"\nCPY_ORD2_B=\"cpy-ord2-b-value\"\n").unwrap();
    let settings = settings_with(vec![env_var("CPY_ORD2_NEW", &["CPY_ORD2_B", "CPY_ORD2_A"])]);
    copy_renamed_values(both.path(), &settings);
    assert_eq!(current_value(both.path(), "CPY_ORD2_NEW").as_deref(), Some("cpy-ord2-b-value"));
}

#[test]
fn second_copy_returns_empty() {
    let dir = config_dir();
    std::fs::write(dir.path().join(".env"), "CPY_TWICE_OLD=\"v1\"\n").unwrap();
    let settings = settings_with(vec![env_var("CPY_TWICE_NEW", &["CPY_TWICE_OLD"])]);

    assert_eq!(copy_renamed_values(dir.path(), &settings).len(), 1);
    assert!(copy_renamed_values(dir.path(), &settings).is_empty());
}
