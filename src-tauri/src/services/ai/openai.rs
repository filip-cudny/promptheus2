use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use serde::Deserialize;
use tokio_stream::StreamExt;

use crate::models::settings::ModelConfig;

use super::encoder::encode_completions_capabilities;
use super::http::OpenAiHttp;
use super::provider::{AiProvider, CompletionRequest, StreamChunk, ToolCallEvent, TokenUsage};
use super::sse::parse_sse_stream;
use super::AiError;

pub struct OpenAiProvider {
    http: OpenAiHttp,
}

impl OpenAiProvider {
    pub fn new(model: &ModelConfig) -> Result<Self, AiError> {
        Ok(Self {
            http: OpenAiHttp::new(model)?,
        })
    }
}

fn build_request_body(
    request: &CompletionRequest,
    stream: bool,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": request.model,
        "messages": request.messages,
        "stream": stream,
    });

    if stream {
        body["stream_options"] = serde_json::json!({"include_usage": true});
    }

    let obj = body.as_object_mut().unwrap();

    if let Some(temp) = request.parameters.temperature {
        obj.insert("temperature".into(), serde_json::json!(temp));
    }
    if let Some(max) = request.parameters.max_tokens {
        obj.insert("max_tokens".into(), serde_json::json!(max));
    }
    if let Some(top_p) = request.parameters.top_p {
        obj.insert("top_p".into(), serde_json::json!(top_p));
    }
    if let Some(fp) = request.parameters.frequency_penalty {
        obj.insert("frequency_penalty".into(), serde_json::json!(fp));
    }
    if let Some(pp) = request.parameters.presence_penalty {
        obj.insert("presence_penalty".into(), serde_json::json!(pp));
    }
    encode_completions_capabilities(&request.capabilities, &request.parameters, obj);

    for (key, value) in &request.parameters.extra {
        obj.insert(key.clone(), value.clone());
    }

    if !request.tool_payloads.is_empty() {
        obj.insert("tools".into(), serde_json::json!(request.tool_payloads));
    }

    body
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ChatCompletionMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChatCompletionChunkChoice>,
    usage: Option<ChunkUsage>,
}

#[derive(Deserialize)]
struct ChatCompletionChunkChoice {
    delta: ChatCompletionDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatCompletionDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Deserialize)]
struct ToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<FunctionCallDelta>,
}

#[derive(Deserialize)]
struct FunctionCallDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct ChunkUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

#[derive(Default)]
struct StreamState {
    accumulated: String,
    accumulated_thinking: String,
    tool_calls: Vec<AccumulatedToolCall>,
    pending_chunks: VecDeque<StreamChunk>,
}

#[derive(Default)]
struct AccumulatedToolCall {
    id: String,
    name: String,
    arguments: String,
}

fn process_chunk(state: &mut StreamState, chunk: ChatCompletionChunk) {
    let usage = chunk.usage.map(|u| TokenUsage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.completion_tokens,
    });

    let (delta, finish_reason) = match chunk.choices.into_iter().next() {
        Some(c) => (Some(c.delta), c.finish_reason),
        None => (None, None),
    };

    if let Some(tool_call_deltas) = delta.as_ref().and_then(|d| d.tool_calls.as_ref()) {
        for tc_delta in tool_call_deltas {
            while state.tool_calls.len() <= tc_delta.index {
                state.tool_calls.push(AccumulatedToolCall::default());
            }

            let fn_delta = tc_delta.function.as_ref();
            let slot = &mut state.tool_calls[tc_delta.index];
            let is_start = slot.id.is_empty() && tc_delta.id.is_some();

            if let Some(id) = tc_delta.id.as_ref() {
                slot.id = id.clone();
            }
            if let Some(name) = fn_delta.and_then(|f| f.name.as_ref()) {
                slot.name = name.clone();
            }
            if let Some(args) = fn_delta.and_then(|f| f.arguments.as_ref()) {
                slot.arguments.push_str(args);
            }

            if is_start {
                let marker = format!("{{{{tool_call:{}}}}}", slot.id);
                state.accumulated.push_str(&marker);
                state.pending_chunks.push_back(StreamChunk {
                    delta: marker,
                    ..StreamChunk::tool_call(
                        &state.accumulated,
                        ToolCallEvent::Started {
                            tool_call_id: slot.id.clone(),
                            tool_name: slot.name.clone(),
                        },
                    )
                });
            }
        }
    }

    let content = delta.as_ref().and_then(|d| d.content.as_deref()).unwrap_or("");
    let thinking = delta
        .as_ref()
        .and_then(|d| d.reasoning_content.as_deref().or(d.reasoning.as_deref()))
        .unwrap_or("");

    if !content.is_empty() || !thinking.is_empty() {
        state.accumulated.push_str(content);
        state.accumulated_thinking.push_str(thinking);
        let thinking_delta = (!thinking.is_empty()).then(|| thinking.to_string());
        state.pending_chunks.push_back(StreamChunk::content(
            content.to_string(),
            &state.accumulated,
            thinking_delta,
            &state.accumulated_thinking,
        ));
    }

    if finish_reason.as_deref() == Some("tool_calls") {
        for tc in std::mem::take(&mut state.tool_calls) {
            if tc.id.is_empty() {
                continue;
            }
            state.pending_chunks.push_back(StreamChunk::tool_call(
                &state.accumulated,
                ToolCallEvent::arguments_complete(tc.id, tc.name, &tc.arguments),
            ));
        }
    }

    if let Some(usage) = usage {
        if let Some(last) = state.pending_chunks.back_mut() {
            last.usage = Some(usage);
        } else {
            state
                .pending_chunks
                .push_back(StreamChunk::usage(&state.accumulated, usage));
        }
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String, AiError> {
        let body = build_request_body(&request, false);
        let response = self
            .http
            .post("/chat/completions", &body, Some(Duration::from_secs(120)))
            .await?;

        let parsed: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::Request(format!("failed to parse response: {e}")))?;

        parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| AiError::Request("empty response from API".into()))
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError> {
        let body = build_request_body(&request, true);
        let response = self.http.post("/chat/completions", &body, None).await?;

        let sse_stream = parse_sse_stream(response);
        let state = StreamState::default();

        let stream = futures::stream::unfold(
            (state, sse_stream),
            |(mut state, mut sse_stream)| async move {
                loop {
                    if let Some(chunk) = state.pending_chunks.pop_front() {
                        return Some((Ok(chunk), (state, sse_stream)));
                    }
                    match sse_stream.next().await {
                        Some(Ok(data)) => {
                            log::trace!("completions chunk: {data}");
                            match serde_json::from_str::<ChatCompletionChunk>(&data) {
                                Ok(chunk) => process_chunk(&mut state, chunk),
                                Err(e) => {
                                    log::warn!("completions: failed to parse chunk: {e}");
                                    log::debug!("completions: raw data: {data}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Some((Err(AiError::Stream(e.to_string())), (state, sse_stream)));
                        }
                        None => return None,
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(data: &str) -> ChatCompletionChunk {
        serde_json::from_str(data).expect("valid chunk json")
    }

    fn drain(state: &mut StreamState) -> Vec<StreamChunk> {
        state.pending_chunks.drain(..).collect()
    }

    #[test]
    fn atomic_tool_call_keeps_arguments() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","function":{"name":"web_search","arguments":"{\"query\":\"docker\"}"},"type":"function","index":0}]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#));

        let events = drain(&mut state);
        assert_eq!(events.len(), 2, "expected Started + ArgumentsComplete");

        match &events[0].tool_call_event {
            Some(ToolCallEvent::Started { tool_call_id, tool_name }) => {
                assert_eq!(tool_call_id, "call_1");
                assert_eq!(tool_name, "web_search");
            }
            other => panic!("expected Started, got {other:?}"),
        }
        match &events[1].tool_call_event {
            Some(ToolCallEvent::ArgumentsComplete { arguments, .. }) => {
                assert_eq!(arguments["query"], "docker");
            }
            other => panic!("expected ArgumentsComplete, got {other:?}"),
        }
    }

    #[test]
    fn fragmented_tool_call_accumulates_arguments() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_x","function":{"name":"web_search","arguments":""},"index":0}]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"function":{"arguments":"{\"q"},"index":0}]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"function":{"arguments":"uery\":\"k8s\"}"},"index":0}]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#));

        let events = drain(&mut state);
        let complete = events.iter().find_map(|c| match &c.tool_call_event {
            Some(ToolCallEvent::ArgumentsComplete { arguments, .. }) => Some(arguments),
            _ => None,
        }).expect("ArgumentsComplete present");
        assert_eq!(complete["query"], "k8s");
    }

    #[test]
    fn repeated_id_does_not_emit_duplicate_started() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_a","function":{"name":"t","arguments":"{"},"index":0}]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_a","function":{"arguments":"}"},"index":0}]}}]}"#));

        let events = drain(&mut state);
        let starts = events.iter().filter(|c| matches!(c.tool_call_event, Some(ToolCallEvent::Started { .. }))).count();
        assert_eq!(starts, 1, "Started should fire once even if id repeats");
    }

    #[test]
    fn multiple_tool_calls_in_single_chunk_each_emit_started() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[
            {"id":"call_1","function":{"name":"a","arguments":"{\"x\":1}"},"index":0},
            {"id":"call_2","function":{"name":"b","arguments":"{\"y\":2}"},"index":1}
        ]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#));

        let events = drain(&mut state);
        let starts: Vec<_> = events.iter().filter_map(|c| match &c.tool_call_event {
            Some(ToolCallEvent::Started { tool_call_id, .. }) => Some(tool_call_id.clone()),
            _ => None,
        }).collect();
        assert_eq!(starts, vec!["call_1", "call_2"]);

        let completes: Vec<_> = events.iter().filter_map(|c| match &c.tool_call_event {
            Some(ToolCallEvent::ArgumentsComplete { tool_call_id, arguments, .. }) => Some((tool_call_id.clone(), arguments.clone())),
            _ => None,
        }).collect();
        assert_eq!(completes.len(), 2);
        assert_eq!(completes[0].0, "call_1");
        assert_eq!(completes[0].1["x"], 1);
        assert_eq!(completes[1].0, "call_2");
        assert_eq!(completes[1].1["y"], 2);
    }

    #[test]
    fn content_and_tool_call_in_same_chunk_both_emit() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{
            "content":"hello ",
            "tool_calls":[{"id":"call_q","function":{"name":"t","arguments":"{}"},"index":0}]
        }}]}"#));

        let events = drain(&mut state);
        assert!(events.iter().any(|c| c.delta == "hello "), "content delta must be emitted");
        assert!(events.iter().any(|c| matches!(c.tool_call_event, Some(ToolCallEvent::Started { .. }))), "Started must be emitted");
    }

    #[test]
    fn usage_attaches_to_last_emitted_chunk() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"content":"hi"}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":3}}"#));

        let events = drain(&mut state);
        let with_usage = events.iter().filter(|c| c.usage.is_some()).count();
        assert_eq!(with_usage, 1);
        let last = events.last().expect("at least one chunk");
        assert_eq!(last.usage.as_ref().unwrap().prompt_tokens, 5);
        assert_eq!(last.usage.as_ref().unwrap().completion_tokens, 3);
    }

    #[test]
    fn empty_delta_chunk_does_not_emit() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"role":"assistant"}}]}"#));
        assert!(state.pending_chunks.is_empty());
    }

    #[test]
    fn unparseable_tool_call_arguments_become_empty_object() {
        let mut state = StreamState::default();
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","function":{"name":"t","arguments":"{not json"},"index":0}]}}]}"#));
        process_chunk(&mut state, parse(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#));

        let events = drain(&mut state);
        match &events[1].tool_call_event {
            Some(ToolCallEvent::ArgumentsComplete { arguments, .. }) => {
                assert_eq!(arguments, &serde_json::json!({}));
            }
            other => panic!("expected ArgumentsComplete, got {other:?}"),
        }
    }
}
