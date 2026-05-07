use std::collections::VecDeque;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tokio_stream::StreamExt;

use crate::models::settings::ModelConfig;

use super::provider::{AiProvider, CompletionRequest, StreamChunk, ToolCallEvent, TokenUsage};
use super::sse::parse_sse_stream;
use super::AiError;

pub struct OpenAiProvider {
    http_client: reqwest::Client,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(model: &ModelConfig) -> Result<Self, AiError> {
        let api_key = model
            .resolved_api_key()
            .ok_or_else(|| {
                AiError::Authentication(format!(
                    "no API key configured for model '{}'",
                    model.display_name
                ))
            })?;

        let base_url = model
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1")
            .trim_end_matches('/')
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| AiError::Request(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| AiError::Request(e.to_string()))?;

        Ok(Self {
            http_client,
            base_url,
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
    request
        .encoder
        .encode_reasoning(&request.capabilities, &request.parameters, obj);

    for (key, value) in &request.parameters.extra {
        obj.insert(key.clone(), value.clone());
    }

    if !request.tool_payloads.is_empty() {
        obj.insert("tools".into(), serde_json::json!(request.tool_payloads));
    }

    body
}

fn map_http_error(status: reqwest::StatusCode, body: &str) -> AiError {
    match status.as_u16() {
        401 => AiError::Authentication("API key is invalid or expired".into()),
        429 => AiError::RateLimit,
        status @ 500..=599 => AiError::ApiStatus {
            status,
            message: body.to_string(),
        },
        status => AiError::ApiStatus {
            status,
            message: body.to_string(),
        },
    }
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
                    accumulated: state.accumulated.clone(),
                    thinking_delta: None,
                    accumulated_thinking: None,
                    usage: None,
                    tool_call_event: Some(ToolCallEvent::Started {
                        tool_call_id: slot.id.clone(),
                        tool_name: slot.name.clone(),
                    }),
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
        let acc_thinking = (!state.accumulated_thinking.is_empty())
            .then(|| state.accumulated_thinking.clone());
        state.pending_chunks.push_back(StreamChunk {
            delta: content.to_string(),
            accumulated: state.accumulated.clone(),
            thinking_delta,
            accumulated_thinking: acc_thinking,
            usage: None,
            tool_call_event: None,
        });
    }

    if finish_reason.as_deref() == Some("tool_calls") {
        for tc in std::mem::take(&mut state.tool_calls) {
            if tc.id.is_empty() {
                continue;
            }
            state.pending_chunks.push_back(StreamChunk {
                delta: String::new(),
                accumulated: state.accumulated.clone(),
                thinking_delta: None,
                accumulated_thinking: None,
                usage: None,
                tool_call_event: Some(ToolCallEvent::ArgumentsComplete {
                    tool_call_id: tc.id,
                    tool_name: tc.name,
                    arguments: serde_json::from_str(&tc.arguments).unwrap_or(serde_json::Value::Null),
                }),
            });
        }
    }

    if usage.is_some() {
        if let Some(last) = state.pending_chunks.back_mut() {
            last.usage = usage;
        } else {
            state.pending_chunks.push_back(StreamChunk {
                delta: String::new(),
                accumulated: state.accumulated.clone(),
                thinking_delta: None,
                accumulated_thinking: None,
                usage,
                tool_call_event: None,
            });
        }
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String, AiError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = build_request_body(&request, false);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AiError::Connection("connection failed — check your internet".into())
                } else {
                    AiError::Request(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(map_http_error(status, &body_text));
        }

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
        let url = format!("{}/chat/completions", self.base_url);
        let body = build_request_body(&request, true);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AiError::Connection("connection failed — check your internet".into())
                } else {
                    AiError::Request(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(map_http_error(status, &body_text));
        }

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
}
