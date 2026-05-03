use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::models::history::HistoryChangedEvent;

#[derive(Default, Clone)]
pub struct HistoryVersion(Arc<AtomicU64>);

impl HistoryVersion {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed) + 1
    }
}

pub fn emit_history_changed(
    app: &AppHandle,
    added_id: Option<String>,
    removed_id: Option<String>,
) -> crate::Result<()> {
    let version = app.state::<HistoryVersion>().next();
    app.emit(
        "history-changed",
        HistoryChangedEvent {
            added_id,
            removed_id,
            version,
        },
    )?;
    Ok(())
}
