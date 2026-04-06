use std::collections::HashMap;
use tokio::sync::oneshot;

pub struct ToolConfirmationService {
    pending: HashMap<String, oneshot::Sender<bool>>,
}

impl ToolConfirmationService {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool_call_id: String) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        self.pending.insert(tool_call_id, tx);
        rx
    }

    pub fn respond(&mut self, tool_call_id: &str, approved: bool) -> Result<(), String> {
        let sender = self
            .pending
            .remove(tool_call_id)
            .ok_or_else(|| format!("No pending confirmation for tool call: {tool_call_id}"))?;
        sender
            .send(approved)
            .map_err(|_| format!("Confirmation receiver dropped for tool call: {tool_call_id}"))
    }

    pub fn cancel_all(&mut self) {
        self.pending.clear();
    }
}
