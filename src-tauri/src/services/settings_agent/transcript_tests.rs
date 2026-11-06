use chrono::{Duration, Utc};
use tempfile::TempDir;

use super::transcript::{prune, write, Transcript};

fn written(dir: &TempDir, age_days: i64, now: chrono::DateTime<Utc>) -> std::path::PathBuf {
    let transcript = Transcript::start("request", now - Duration::days(age_days));
    write(dir.path(), &transcript).unwrap()
}

#[test]
fn prune_deletes_an_older_transcript_and_keeps_a_newer_one() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    let old = written(&dir, 40, now);
    let new = written(&dir, 1, now);

    assert_eq!(prune(dir.path(), 30, now), 1);

    assert!(!old.exists());
    assert!(new.exists());
}

#[test]
fn prune_with_zero_retention_keeps_everything() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    let old = written(&dir, 400, now);

    assert_eq!(prune(dir.path(), 0, now), 0);

    assert!(old.exists());
}

#[test]
fn prune_skips_an_unreadable_file_and_a_missing_directory() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    let broken = dir.path().join("broken.json");
    std::fs::write(&broken, "not json").unwrap();

    assert_eq!(prune(dir.path(), 30, now), 0);
    assert!(broken.exists());
    assert_eq!(prune(&dir.path().join("missing"), 30, now), 0);
}
