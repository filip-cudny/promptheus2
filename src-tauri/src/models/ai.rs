use serde::Serialize;

use crate::models::message::NodeUpdate;

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum StreamEvent {
    Chunk {
        delta: String,
        accumulated: String,
        thinking_delta: Option<String>,
        accumulated_thinking: Option<String>,
    },
    Done {
        full_text: String,
        full_thinking: Option<String>,
        prompt_tokens: Option<usize>,
        completion_tokens: Option<usize>,
    },
    Error { message: String },
    NodeUpdates { node_id: String, updates: Vec<NodeUpdate> },
}
