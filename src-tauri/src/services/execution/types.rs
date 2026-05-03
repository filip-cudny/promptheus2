pub struct PendingToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

pub struct StreamResult {
    pub full_text: String,
    pub full_thinking: Option<String>,
    pub prompt_tokens: Option<usize>,
    pub completion_tokens: Option<usize>,
    pub pending_tool_calls: Vec<PendingToolCall>,
}

pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub result_text: String,
    pub is_error: bool,
}

pub const MAX_TOOL_LOOP_ITERATIONS: usize = 10;
