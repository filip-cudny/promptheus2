use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::watch;
use tokio::sync::Mutex as TokioMutex;
use tokio_stream::{Stream, StreamExt};

use super::types::{PendingToolCall, StreamResult, ToolExecutionResult};
use crate::models::ai::{StreamEvent, ToolCall, ToolCallStatus, ToolCallType};
use crate::models::message::{
    MessageContent, ProcessedMessage, ToolCallFunction, ToolCallPayload,
};
use crate::services::ai::provider::{StreamChunk, ToolCallEvent};
use crate::services::ai::AiError;
use crate::services::execution::lifecycle::LiveExecution;
use crate::services::mcp::McpRegistry;
use crate::Error;

fn tool_display_name(tool_name: &str) -> &str {
    match tool_name {
        "web_search" => "Web Search",
        other => other,
    }
}

fn tool_type_from_name(tool_name: &str) -> ToolCallType {
    match tool_name {
        "web_search" => ToolCallType::WebSearch,
        _ => ToolCallType::Custom,
    }
}

pub async fn run_stream_loop(
    mut stream: Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>,
    live: Arc<TokioMutex<LiveExecution>>,
    cancel_rx: Option<watch::Receiver<bool>>,
    text_prefix: &str,
) -> crate::Result<StreamResult> {
    let mut full_text = String::new();
    let mut full_thinking = String::new();
    let mut prompt_tokens: Option<usize> = None;
    let mut completion_tokens: Option<usize> = None;
    let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
    let mut cancel_rx = cancel_rx;

    loop {
        let chunk_result: Option<Result<StreamChunk, AiError>> =
            if let Some(ref mut rx) = cancel_rx {
                tokio::select! {
                    biased;
                    result = rx.changed() => {
                        if result.is_ok() && *rx.borrow() {
                            let mut live = live.lock().await;
                            live.snapshot.finished = true;
                            live.snapshot.error = Some("Cancelled".to_string());
                            if let Some(ref ch) = live.channel {
                                let _ = ch.send(StreamEvent::Error { message: "Cancelled".to_string() });
                            }
                            return Err(Error::Other("Cancelled".to_string()));
                        }
                        continue;
                    }
                    next = stream.next() => next,
                }
            } else {
                stream.next().await
            };

        let Some(result) = chunk_result else { break };

        match result {
            Ok(chunk) => {
                if let Some(usage) = chunk.usage {
                    prompt_tokens = Some(usage.prompt_tokens);
                    completion_tokens = Some(usage.completion_tokens);
                }

                if let Some(tool_event) = chunk.tool_call_event {
                    let prefixed_accumulated = format!("{text_prefix}{}", chunk.accumulated);
                    full_text.clone_from(&prefixed_accumulated);
                    let mut live = live.lock().await;
                    live.snapshot.accumulated_text.clone_from(&prefixed_accumulated);

                    match tool_event {
                        ToolCallEvent::Started {
                            tool_call_id,
                            tool_name,
                        } => {
                            let display_name = tool_display_name(&tool_name).to_string();
                            let tool_type = tool_type_from_name(&tool_name);
                            let tc = ToolCall {
                                tool_call_id,
                                tool_name,
                                tool_display_name: display_name,
                                tool_type,
                                arguments: serde_json::json!({}),
                                result: None,
                                error: None,
                                status: ToolCallStatus::InProgress,
                                requires_confirmation: false,
                                started_at: Some(chrono::Utc::now().to_rfc3339()),
                                completed_at: None,
                            };
                            live.snapshot.tool_calls.push(tc.clone());
                            if let Some(ref ch) = live.channel {
                                if ch.send(StreamEvent::ToolCallStart { tool_call: tc }).is_err() {
                                    live.channel = None;
                                }
                            }
                        }
                        ToolCallEvent::ArgumentsComplete {
                            tool_call_id,
                            tool_name,
                            arguments,
                        } => {
                            if let Some(tc) = live
                                .snapshot
                                .tool_calls
                                .iter_mut()
                                .find(|t| t.tool_call_id == tool_call_id)
                            {
                                tc.arguments = arguments.clone();
                            }
                            pending_tool_calls.push(PendingToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                            });
                        }
                        ToolCallEvent::Done {
                            tool_call_id,
                            result,
                            error,
                        } => {
                            if let Some(tc) = live
                                .snapshot
                                .tool_calls
                                .iter_mut()
                                .find(|t| t.tool_call_id == tool_call_id)
                            {
                                tc.status = if error.is_some() {
                                    ToolCallStatus::Failed
                                } else {
                                    ToolCallStatus::Completed
                                };
                                tc.result.clone_from(&result);
                                tc.error.clone_from(&error);
                                tc.completed_at = Some(chrono::Utc::now().to_rfc3339());
                            }
                            if let Some(ref ch) = live.channel {
                                if ch
                                    .send(StreamEvent::ToolCallDone {
                                        tool_call_id,
                                        result,
                                        error,
                                    })
                                    .is_err()
                                {
                                    live.channel = None;
                                }
                            }
                        }
                    }
                    continue;
                }

                let has_content = !chunk.delta.is_empty() || chunk.thinking_delta.is_some();
                if has_content {
                    let prefixed_accumulated = format!("{text_prefix}{}", chunk.accumulated);
                    full_text.clone_from(&prefixed_accumulated);
                    if let Some(ref acc) = chunk.accumulated_thinking {
                        full_thinking.clone_from(acc);
                    }

                    let mut live = live.lock().await;
                    live.snapshot.accumulated_text.clone_from(&prefixed_accumulated);
                    live.snapshot.accumulated_thinking = chunk.accumulated_thinking.clone();
                    live.snapshot.is_thinking =
                        chunk.accumulated_thinking.is_some() && chunk.accumulated.is_empty();

                    if let Some(ref ch) = live.channel {
                        if ch
                            .send(StreamEvent::Chunk {
                                delta: chunk.delta,
                                accumulated: prefixed_accumulated,
                                thinking_delta: chunk.thinking_delta,
                                accumulated_thinking: chunk.accumulated_thinking,
                            })
                            .is_err()
                        {
                            live.channel = None;
                        }
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let mut live = live.lock().await;
                live.snapshot.finished = true;
                live.snapshot.error = Some(msg.clone());
                if let Some(ref ch) = live.channel {
                    let _ = ch.send(StreamEvent::Error { message: msg });
                }
                return Err(Error::from(e));
            }
        }
    }

    let thinking = if full_thinking.is_empty() {
        None
    } else {
        Some(full_thinking)
    };

    if pending_tool_calls.is_empty() {
        let mut live = live.lock().await;
        live.snapshot.finished = true;
        live.snapshot.prompt_tokens = prompt_tokens;
        live.snapshot.completion_tokens = completion_tokens;
        if let Some(ref ch) = live.channel {
            let _ = ch.send(StreamEvent::Done {
                full_text: full_text.clone(),
                full_thinking: thinking.clone(),
                prompt_tokens,
                completion_tokens,
            });
        }
    }

    Ok(StreamResult {
        full_text,
        full_thinking: thinking,
        prompt_tokens,
        completion_tokens,
        pending_tool_calls,
    })
}

fn extract_mcp_result_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn execute_tool_calls(
    mcp: &Arc<McpRegistry>,
    pending: &[PendingToolCall],
    live: &Arc<TokioMutex<LiveExecution>>,
) -> Vec<ToolExecutionResult> {
    let futures: Vec<_> = pending
        .iter()
        .map(|tc| {
            let tool_call_id = tc.tool_call_id.clone();
            let tool_name = tc.tool_name.clone();
            let arguments = tc.arguments.clone();
            let mcp = Arc::clone(mcp);
            let live = Arc::clone(live);
            async move {
                let result = mcp.call_tool(&tool_name, arguments).await;

                let execution_result = match result {
                    Ok(result) => {
                        let is_error = result.is_error.unwrap_or(false);
                        let text = extract_mcp_result_text(&result);
                        ToolExecutionResult {
                            tool_call_id,
                            result_text: text,
                            is_error,
                        }
                    }
                    Err(e) => ToolExecutionResult {
                        tool_call_id,
                        result_text: format!("Error executing tool '{}': {}", tool_name, e),
                        is_error: true,
                    },
                };

                let mut live = live.lock().await;
                if let Some(tc) = live
                    .snapshot
                    .tool_calls
                    .iter_mut()
                    .find(|t| t.tool_call_id == execution_result.tool_call_id)
                {
                    tc.status = if execution_result.is_error {
                        ToolCallStatus::Failed
                    } else {
                        ToolCallStatus::Completed
                    };
                    if execution_result.is_error {
                        tc.error = Some(execution_result.result_text.clone());
                    } else {
                        tc.result = Some(execution_result.result_text.clone());
                    }
                    tc.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
                if let Some(ref ch) = live.channel {
                    let _ = ch.send(StreamEvent::ToolCallDone {
                        tool_call_id: execution_result.tool_call_id.clone(),
                        result: if execution_result.is_error {
                            None
                        } else {
                            Some(execution_result.result_text.clone())
                        },
                        error: if execution_result.is_error {
                            Some(execution_result.result_text.clone())
                        } else {
                            None
                        },
                    });
                }

                execution_result
            }
        })
        .collect();

    futures::future::join_all(futures).await
}

pub fn build_tool_loop_messages(
    pending: &[PendingToolCall],
    accumulated_text: &str,
    results: &[ToolExecutionResult],
) -> Vec<ProcessedMessage> {
    let mut messages = Vec::with_capacity(1 + results.len());

    let tool_calls: Vec<ToolCallPayload> = pending
        .iter()
        .map(|tc| ToolCallPayload {
            id: tc.tool_call_id.clone(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: tc.tool_name.clone(),
                arguments: serde_json::to_string(&tc.arguments).unwrap_or_default(),
            },
        })
        .collect();

    messages.push(ProcessedMessage {
        role: "assistant".to_string(),
        content: MessageContent::Text(accumulated_text.to_string()),
        tool_calls: Some(tool_calls),
        tool_call_id: None,
    });

    for r in results {
        messages.push(ProcessedMessage {
            role: "tool".to_string(),
            content: MessageContent::Text(r.result_text.clone()),
            tool_calls: None,
            tool_call_id: Some(r.tool_call_id.clone()),
        });
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ai::capabilities::ModelCapabilities;
    use crate::services::ai::provider::{AiProvider, CompletionRequest};
    use crate::services::execution::lifecycle::{ExecutionSnapshot, LiveExecution};
    use async_trait::async_trait;
    use futures::stream;

    struct FakeProvider {
        chunks: std::sync::Mutex<Option<Vec<StreamChunk>>>,
    }

    impl FakeProvider {
        fn new(chunks: Vec<StreamChunk>) -> Self {
            Self {
                chunks: std::sync::Mutex::new(Some(chunks)),
            }
        }
    }

    #[async_trait]
    impl AiProvider for FakeProvider {
        fn capabilities(&self, _model: &str) -> ModelCapabilities {
            ModelCapabilities::minimal()
        }

        async fn complete(&self, _request: CompletionRequest) -> Result<String, AiError> {
            Ok(String::new())
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError>
        {
            let chunks = self.chunks.lock().unwrap().take().unwrap_or_default();
            Ok(Box::pin(stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    fn empty_live() -> Arc<TokioMutex<LiveExecution>> {
        Arc::new(TokioMutex::new(LiveExecution {
            snapshot: ExecutionSnapshot {
                execution_id: "test".to_string(),
                user_message: String::new(),
                accumulated_text: String::new(),
                accumulated_thinking: None,
                tool_calls: Vec::new(),
                is_thinking: false,
                finished: false,
                error: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
            channel: None,
        }))
    }

    fn text_chunk(delta: &str, accumulated: &str) -> StreamChunk {
        StreamChunk {
            delta: delta.to_string(),
            accumulated: accumulated.to_string(),
            thinking_delta: None,
            accumulated_thinking: None,
            usage: None,
            tool_call_event: None,
        }
    }

    #[tokio::test]
    async fn stream_loop_aggregates_text_chunks() {
        let provider = FakeProvider::new(vec![
            text_chunk("Hello", "Hello"),
            text_chunk(" world", "Hello world"),
        ]);

        let request = CompletionRequest {
            model: "test".to_string(),
            messages: vec![],
            parameters: Default::default(),
            tool_payloads: vec![],
        };
        let stream = provider.complete_stream(request).await.unwrap();

        let result = run_stream_loop(stream, empty_live(), None, "")
            .await
            .unwrap();
        assert_eq!(result.full_text, "Hello world");
        assert!(result.pending_tool_calls.is_empty());
    }
}
