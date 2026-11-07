use std::os::unix::fs::symlink;

use chrono::{Duration, Utc};
use tempfile::TempDir;

use super::*;

fn entry(id: &str, created_at: &str, host: &str, kind: EntryKind) -> ChangeLogEntry {
    ChangeLogEntry {
        id: id.to_string(),
        created_at: created_at.to_string(),
        host: host.to_string(),
        kind,
        request: None,
        reply: None,
        operations: Vec::new(),
        undoes: None,
        secret: None,
    }
}

fn undo(id: &str, created_at: &str, undoes: &str) -> ChangeLogEntry {
    ChangeLogEntry {
        undoes: Some(undoes.to_string()),
        ..entry(id, created_at, "h", EntryKind::Undo)
    }
}

fn aged(days: i64, id: &str) -> ChangeLogEntry {
    let created = (Utc::now() - Duration::days(days)).to_rfc3339();
    entry(id, &created, "h", EntryKind::Card)
}

#[test]
fn file_name_matches_format() {
    let dir = TempDir::new().unwrap();
    let id = new_id(Utc::now(), "my host.local");
    let path = write_entry(dir.path(), &entry(&id, "2026-01-01T00:00:00Z", "my-host-local", EntryKind::Card)).unwrap();
    let name = path.file_name().unwrap().to_str().unwrap();
    let re = regex::Regex::new(r"^\d{8}T\d{6}Z-[A-Za-z0-9-]+-[0-9a-f]{8}\.json$").unwrap();
    assert!(re.is_match(name), "{name}");
    assert!(name.contains("-my-host-local-"));
}

#[test]
fn writing_an_existing_entry_fails_and_keeps_the_file() {
    let dir = TempDir::new().unwrap();
    let first = entry("e1", "2026-01-01T00:00:00Z", "a", EntryKind::Card);
    let path = write_entry(dir.path(), &first).unwrap();
    let before = std::fs::read(&path).unwrap();

    let second = ChangeLogEntry {
        request: Some("other".to_string()),
        ..first
    };
    assert!(write_entry(dir.path(), &second).is_err());

    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn entries_of_different_hosts_load_sorted_by_created_at() {
    let dir = TempDir::new().unwrap();
    write_entry(dir.path(), &entry("late", "2026-01-02T00:00:00Z", "b", EntryKind::Card)).unwrap();
    write_entry(dir.path(), &entry("early", "2026-01-01T00:00:00Z", "a", EntryKind::Secret)).unwrap();
    std::fs::write(dir.path().join("broken.json"), "{").unwrap();

    let ids: Vec<String> = load_all(dir.path()).into_iter().map(|e| e.id).collect();

    assert_eq!(ids, ["early", "late"]);
}

#[test]
fn entry_state_follows_undo_chain() {
    let card = entry("c", "2026-01-01T00:00:00Z", "h", EntryKind::Card);
    let mut entries = vec![card];
    assert_eq!(entry_state(&entries, "c"), EntryState::Applied);

    entries.push(undo("u1", "2026-01-02T00:00:00Z", "c"));
    assert_eq!(entry_state(&entries, "c"), EntryState::Undone);

    entries.push(undo("u2", "2026-01-03T00:00:00Z", "u1"));
    assert_eq!(entry_state(&entries, "c"), EntryState::Applied);
}

#[test]
fn prune_deletes_old_entry_with_its_backup_and_keeps_recent() {
    let dir = TempDir::new().unwrap();
    let backups = TempDir::new().unwrap();
    std::fs::write(backups.path().join("old.bak"), "secret").unwrap();
    let old = ChangeLogEntry {
        secret: Some(SecretChange {
            name: "KEY".to_string(),
            backup: Some("old.bak".to_string()),
            had_previous: true,
        }),
        ..aged(10, "old")
    };
    write_entry(dir.path(), &old).unwrap();
    write_entry(dir.path(), &aged(1, "recent")).unwrap();

    let pruned = prune(dir.path(), backups.path(), 7, Utc::now());

    assert_eq!(pruned, 1);
    assert!(!dir.path().join("old.json").exists());
    assert!(!backups.path().join("old.bak").exists());
    assert!(dir.path().join("recent.json").exists());
}

#[test]
fn prune_deletes_the_card_backup_of_a_pruned_entry_and_keeps_the_others() {
    let dir = TempDir::new().unwrap();
    let backups = TempDir::new().unwrap();
    write_entry(dir.path(), &aged(10, "old-card")).unwrap();
    write_entry(dir.path(), &aged(1, "recent-card")).unwrap();
    std::fs::write(backups.path().join("old-card.env"), "before__KEY=\"v\"\n").unwrap();
    std::fs::write(backups.path().join("recent-card.env"), "").unwrap();

    assert_eq!(prune(dir.path(), backups.path(), 7, Utc::now()), 1);

    assert!(!backups.path().join("old-card.env").exists());
    assert!(backups.path().join("recent-card.env").exists());
}

#[test]
fn a_secret_target_serializes_with_its_name() {
    let target = Target::Secret { name: "KEY".to_string() };

    assert_eq!(serde_json::to_value(&target).unwrap(), serde_json::json!({"type": "secret", "name": "KEY"}));
}

#[test]
fn prune_with_zero_retention_deletes_nothing() {
    let dir = TempDir::new().unwrap();
    write_entry(dir.path(), &aged(400, "old")).unwrap();

    assert_eq!(prune(dir.path(), dir.path(), 0, Utc::now()), 0);
    assert!(dir.path().join("old.json").exists());
}

#[test]
fn dir_follows_settings_symlink_target() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().canonicalize().unwrap().join("repo/tauri-app");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("settings.json"), "{}").unwrap();
    let config = tmp.path().join("config");
    std::fs::create_dir_all(&config).unwrap();
    symlink(repo.join("settings.json"), config.join("settings.json")).unwrap();

    assert_eq!(change_log_dir(&config), repo.join("change-log"));
}

#[test]
fn dir_without_symlink_is_inside_config() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("settings.json"), "{}").unwrap();

    assert_eq!(change_log_dir(tmp.path()), tmp.path().join("change-log"));
    assert_eq!(secret_backup_dir(tmp.path()), tmp.path().join("secret-backups"));
}
