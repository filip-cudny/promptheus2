use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::models::message::ProcessedMessage;
use crate::models::settings::ModelParameters;

use super::AiError;

pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ProcessedMessage>,
    pub parameters: ModelParameters,
}

pub struct StreamChunk {
    pub delta: String,
    pub accumulated: String,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<String, AiError>;

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError>;
}
