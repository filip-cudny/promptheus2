use serde::Serialize;

use crate::models::message::NodeUpdate;

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum StreamEvent {
    Chunk { delta: String, accumulated: String },
    Done { full_text: String },
    Error { message: String },
    NodeUpdates { node_id: String, updates: Vec<NodeUpdate> },
}
