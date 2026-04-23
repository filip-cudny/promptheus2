use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::models::message::ProcessedMessage;
use crate::models::settings::ModelParameters;

use super::capabilities::ModelCapabilities;
use super::AiError;

pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ProcessedMessage>,
    pub parameters: ModelParameters,
    pub tool_payloads: Vec<serde_json::Value>,
}

pub struct StreamChunk {
    pub delta: String,
    pub accumulated: String,
    pub thinking_delta: Option<String>,
    pub accumulated_thinking: Option<String>,
    pub usage: Option<TokenUsage>,
    pub tool_call_event: Option<ToolCallEvent>,
}

#[derive(Debug, Clone)]
pub enum ToolCallEvent {
    Started {
        tool_call_id: String,
        tool_name: String,
    },
    ArgumentsComplete {
        tool_call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    Done {
        tool_call_id: String,
        result: Option<String>,
        error: Option<String>,
    },
}

pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn capabilities(&self, model: &str) -> ModelCapabilities;

    async fn complete(&self, request: CompletionRequest) -> Result<String, AiError>;

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError>;
}
