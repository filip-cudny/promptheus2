use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::{stream, Stream};

use super::provider::{AiProvider, CompletionRequest, StreamChunk, ToolCallEvent};
use super::AiError;

pub enum MockReply {
    Text(String),
    ToolCalls(Vec<(String, String, serde_json::Value)>),
    Error(AiError),
}

pub struct MockProvider {
    script: Mutex<VecDeque<MockReply>>,
    requests: Arc<Mutex<Vec<String>>>,
}

impl MockProvider {
    pub fn new(script: Vec<MockReply>) -> (Self, Arc<Mutex<Vec<String>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let provider = Self {
            script: Mutex::new(script.into()),
            requests: requests.clone(),
        };
        (provider, requests)
    }

    fn next_reply(&self, request: &CompletionRequest) -> MockReply {
        let recorded = serde_json::json!({
            "messages": request.messages,
            "tool_payloads": request.tool_payloads,
        });
        self.requests.lock().unwrap().push(recorded.to_string());
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| MockReply::Error(AiError::Request("mock script exhausted".into())))
    }
}

#[async_trait]
impl AiProvider for MockProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String, AiError> {
        match self.next_reply(&request) {
            MockReply::Text(text) => Ok(text),
            MockReply::ToolCalls(_) => Ok(String::new()),
            MockReply::Error(error) => Err(error),
        }
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError> {
        let chunks = match self.next_reply(&request) {
            MockReply::Text(text) => vec![StreamChunk::content(text.clone(), &text, None, "")],
            MockReply::ToolCalls(calls) => calls
                .into_iter()
                .flat_map(|(id, name, arguments)| {
                    [
                        ToolCallEvent::Started {
                            tool_call_id: id.clone(),
                            tool_name: name.clone(),
                        },
                        ToolCallEvent::ArgumentsComplete {
                            tool_call_id: id,
                            tool_name: name,
                            arguments,
                        },
                    ]
                })
                .map(|event| StreamChunk::tool_call("", event))
                .collect(),
            MockReply::Error(error) => return Err(error),
        };
        Ok(Box::pin(stream::iter(chunks.into_iter().map(Ok))))
    }
}
