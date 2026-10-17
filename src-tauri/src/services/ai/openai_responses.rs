use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tokio_stream::StreamExt;

use crate::models::message::{ContentPart, MessageContent, ProcessedMessage};
use crate::models::settings::ModelConfig;

use super::provider::{AiProvider, CompletionRequest, StreamChunk, TokenUsage, ToolCallEvent};
use super::sse::parse_sse_stream;
use super::tools::ToolRegistry;
use super::AiError;

pub struct OpenAiResponsesProvider {
    http_client: reqwest::Client,
    base_url: String,
    store: bool,
}

impl OpenAiResponsesProvider {
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
            store: model.store,
        })
    }
}

fn to_responses_messages(msg: &ProcessedMessage) -> Vec<serde_json::Value> {
    if msg.role == "tool" {
        if let Some(ref call_id) = msg.tool_call_id {
            let output = match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Parts(_) => String::new(),
            };
            return vec![serde_json::json!({
                "type": "function_call_output",
                "call_id": call_id,
                "output": output,
            })];
        }
    }

    let mut results = Vec::new();

    if let Some(ref tool_calls) = msg.tool_calls {
        for tc in tool_calls {
            results.push(serde_json::json!({
                "type": "function_call",
                "call_id": tc.id,
                "name": tc.function.name,
                "arguments": tc.function.arguments,
            }));
        }
    }

    if !results.is_empty() {
        return results;
    }

    let content = match &msg.content {
        MessageContent::Text(text) => serde_json::json!(text),
        MessageContent::Parts(parts) => {
            let mapped: Vec<serde_json::Value> = parts
                .iter()
                .map(|part| match part {
                    ContentPart::Text { text } => serde_json::json!({
                        "type": "input_text",
                        "text": text,
                    }),
                    ContentPart::ImageUrl { image_url } => serde_json::json!({
                        "type": "input_image",
                        "image_url": image_url.url,
                    }),
                })
                .collect();
            serde_json::json!(mapped)
        }
    };
    vec![serde_json::json!({
        "role": msg.role,
        "content": content,
    })]
}

fn build_request_body(request: &CompletionRequest, stream: bool, store: bool) -> serde_json::Value {
    let mut instructions: Option<String> = None;
    let input_messages: Vec<serde_json::Value> = request
        .messages
        .iter()
        .filter(|m| {
            if m.role == "system" || m.role == "developer" {
                if let MessageContent::Text(ref text) = m.content {
                    instructions = Some(match instructions.take() {
                        Some(existing) => format!("{existing}\n{text}"),
                        None => text.clone(),
                    });
                }
                false
            } else {
                true
            }
        })
        .flat_map(|m| to_responses_messages(m))
        .collect();

    let mut body = serde_json::json!({
        "model": request.model,
        "input": input_messages,
        "stream": stream,
        "store": store,
    });

    let obj = body.as_object_mut().unwrap();
    if let Some(instructions) = instructions {
        obj.insert("instructions".into(), serde_json::json!(instructions));
    }

    if let Some(temp) = request.parameters.temperature {
        obj.insert("temperature".into(), serde_json::json!(temp));
    }
    if let Some(max) = request.parameters.max_tokens {
        obj.insert("max_output_tokens".into(), serde_json::json!(max));
    }
    if let Some(top_p) = request.parameters.top_p {
        obj.insert("top_p".into(), serde_json::json!(top_p));
    }

    request
        .encoder
        .encode_reasoning(&request.capabilities, &request.parameters, obj);

    if !request.tool_payloads.is_empty() {
        obj.insert("tools".into(), serde_json::json!(request.tool_payloads));
    }

    for (key, value) in &request.parameters.extra {
        obj.insert(key.clone(), value.clone());
    }

    log::debug!(
        "responses: model={}, reasoning={:?}",
        request.model,
        obj.get("reasoning")
    );

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
        other => AiError::ApiStatus {
            status: other,
            message: body.to_string(),
        },
    }
}

#[derive(Deserialize)]
struct NonStreamingResponse {
    output: Vec<OutputItem>,
}

#[derive(Deserialize)]
struct OutputItem {
    #[serde(rename = "type")]
    item_type: String,
    content: Option<Vec<ContentItem>>,
}

#[derive(Deserialize)]
struct ContentItem {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ResponseEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    response: Option<ResponseWrapper>,
    #[serde(default)]
    item: Option<OutputItemEvent>,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OutputItemEvent {
    #[serde(rename = "type")]
    item_type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    action: Option<serde_json::Value>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct ResponseWrapper {
    #[serde(default)]
    usage: Option<ResponseUsage>,
}

#[derive(Deserialize)]
struct ResponseUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[async_trait]
impl AiProvider for OpenAiResponsesProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<String, AiError> {
        let url = format!("{}/responses", self.base_url);
        let body = build_request_body(&request, false, self.store);

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

        let response_text = response
            .text()
            .await
            .map_err(|e| AiError::Request(format!("failed to read response: {e}")))?;

        log::debug!("responses: complete response_len={}", response_text.len());

        let parsed: NonStreamingResponse = serde_json::from_str(&response_text)
            .map_err(|e| AiError::Request(format!("failed to parse response: {e}")))?;

        parsed
            .output
            .iter()
            .find(|item| item.item_type == "message")
            .and_then(|item| item.content.as_ref())
            .and_then(|parts| parts.first())
            .and_then(|part| part.text.clone())
            .ok_or_else(|| AiError::Request("empty response from API".into()))
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError> {
        let url = format!("{}/responses", self.base_url);
        let body = build_request_body(&request, true, self.store);

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

        Ok(Box::pin(parse_responses_events(parse_sse_stream(response))))
    }
}

fn parse_responses_events<E: std::fmt::Display>(
    sse_stream: impl Stream<Item = Result<String, E>> + Unpin + Send + 'static,
) -> impl Stream<Item = Result<StreamChunk, AiError>> + Send {
    futures::stream::unfold(
    (sse_stream, String::new(), String::new(), Vec::<String>::new(), HashMap::<String, (String, String)>::new(), HashMap::<String, String>::new()),
    |(mut sse_stream, mut accumulated, mut accumulated_thinking, mut active_tool_call_ids, mut fn_call_args, mut item_id_to_call_id)| async move {
        loop {
            match sse_stream.next().await {
                Some(Ok(data)) => {
                    log::trace!("responses: SSE event len={}", data.len());
                    let event: ResponseEvent = match serde_json::from_str(&data) {
                        Ok(e) => e,
                        Err(e) => {
                            log::warn!("responses: failed to parse SSE event: {e}");
                            continue;
                        }
                    };

                    log::trace!("responses: event_type={}", event.event_type);

                    match event.event_type.as_str() {
                        "response.reasoning_summary_text.delta" => {
                            let thinking = event.delta.unwrap_or_default();
                            log::trace!("responses: reasoning delta len={}", thinking.len());
                            if thinking.is_empty() {
                                continue;
                            }
                            accumulated_thinking.push_str(&thinking);
                            return Some((
                                Ok(StreamChunk {
                                    delta: String::new(),
                                    accumulated: accumulated.clone(),
                                    thinking_delta: Some(thinking),
                                    accumulated_thinking: Some(accumulated_thinking.clone()),
                                    usage: None,
                                    tool_call_event: None,
                                }),
                                (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                            ));
                        }
                        "response.output_text.delta" => {
                            let delta = event.delta.unwrap_or_default();
                            if delta.is_empty() {
                                continue;
                            }
                            accumulated.push_str(&delta);
                            let acc_thinking = if accumulated_thinking.is_empty() {
                                None
                            } else {
                                Some(accumulated_thinking.clone())
                            };
                            return Some((
                                Ok(StreamChunk {
                                    delta,
                                    accumulated: accumulated.clone(),
                                    thinking_delta: None,
                                    accumulated_thinking: acc_thinking,
                                    usage: None,
                                    tool_call_event: None,
                                }),
                                (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                            ));
                        }
                        "response.output_item.added" => {
                            if let Some(ref item) = event.item {
                                if item.item_type.as_deref() == Some("web_search_call") {
                                    let tool_call_id = item.id.clone().unwrap_or_default();
                                    log::debug!("responses: web_search_call started id={tool_call_id}");
                                    active_tool_call_ids.push(tool_call_id.clone());
                                    let marker = format!("{{{{tool_call:{tool_call_id}}}}}");
                                    accumulated.push_str(&marker);
                                    return Some((
                                        Ok(StreamChunk {
                                            delta: String::new(),
                                            accumulated: accumulated.clone(),
                                            thinking_delta: None,
                                            accumulated_thinking: None,
                                            usage: None,
                                            tool_call_event: Some(ToolCallEvent::Started {
                                                tool_call_id,
                                                tool_name: "web_search".to_string(),
                                            }),
                                        }),
                                        (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                                    ));
                                }
                                if item.item_type.as_deref() == Some("function_call") {
                                    let tool_call_id = item.call_id.clone().unwrap_or_default();
                                    let tool_name = item.name.clone().unwrap_or_default();
                                    log::debug!("responses: function_call started call_id={tool_call_id} item_id={:?} name={tool_name}", item.id);
                                    active_tool_call_ids.push(tool_call_id.clone());
                                    fn_call_args.insert(tool_call_id.clone(), (tool_name.clone(), String::new()));
                                    if let Some(ref item_id) = item.id {
                                        item_id_to_call_id.insert(item_id.clone(), tool_call_id.clone());
                                    }
                                    let marker = format!("{{{{tool_call:{tool_call_id}}}}}");
                                    accumulated.push_str(&marker);
                                    return Some((
                                        Ok(StreamChunk {
                                            delta: String::new(),
                                            accumulated: accumulated.clone(),
                                            thinking_delta: None,
                                            accumulated_thinking: None,
                                            usage: None,
                                            tool_call_event: Some(ToolCallEvent::Started {
                                                tool_call_id,
                                                tool_name,
                                            }),
                                        }),
                                        (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                                    ));
                                }
                            }
                            continue;
                        }
                        "response.function_call_arguments.delta" => {
                            let raw_id = event.call_id.or(event.item_id.clone());
                            if let Some(raw_id) = raw_id {
                                let call_id = item_id_to_call_id.get(&raw_id).cloned().unwrap_or(raw_id);
                                if let Some(delta) = event.delta {
                                    if let Some((_, ref mut acc_args)) = fn_call_args.get_mut(&call_id) {
                                        acc_args.push_str(&delta);
                                    }
                                }
                            }
                            continue;
                        }
                        "response.function_call_arguments.done" => {
                            let raw_id = event.call_id.or(event.item_id.clone());
                            log::debug!("responses: fn_call_args.done raw_id={:?} name={:?} has_arguments={}", raw_id, event.name, event.arguments.is_some());
                            if let Some(raw_id) = raw_id {
                                let call_id = item_id_to_call_id.get(&raw_id).cloned().unwrap_or(raw_id);
                                let tool_name = event.name.unwrap_or_else(|| {
                                    fn_call_args.get(&call_id)
                                        .map(|(name, _)| name.clone())
                                        .unwrap_or_default()
                                });
                                let arguments_str = event.arguments.unwrap_or_else(|| {
                                    fn_call_args.get(&call_id)
                                        .map(|(_, args)| args.clone())
                                        .unwrap_or_default()
                                });
                                let arguments: serde_json::Value = serde_json::from_str(&arguments_str)
                                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                                log::debug!("responses: function_call args complete id={call_id} name={tool_name}");
                                fn_call_args.remove(&call_id);
                                active_tool_call_ids.retain(|id| id != &call_id);
                                return Some((
                                    Ok(StreamChunk {
                                        delta: String::new(),
                                        accumulated: accumulated.clone(),
                                        thinking_delta: None,
                                        accumulated_thinking: None,
                                        usage: None,
                                        tool_call_event: Some(ToolCallEvent::ArgumentsComplete {
                                            tool_call_id: call_id,
                                            tool_name,
                                            arguments,
                                        }),
                                    }),
                                    (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                                ));
                            }
                            continue;
                        }
                        "response.output_item.done" => {
                            if let Some(ref item) = event.item {
                                if item.item_type.as_deref() == Some("web_search_call") {
                                    let tool_call_id = item.id.clone().unwrap_or_default();
                                    log::debug!("responses: web_search_call done id={tool_call_id}");
                                    active_tool_call_ids.retain(|id| id != &tool_call_id);

                                    let result = item.action.as_ref()
                                        .and_then(ToolRegistry::format_web_search_result);

                                    return Some((
                                        Ok(StreamChunk {
                                            delta: String::new(),
                                            accumulated: accumulated.clone(),
                                            thinking_delta: None,
                                            accumulated_thinking: None,
                                            usage: None,
                                            tool_call_event: Some(ToolCallEvent::Done {
                                                tool_call_id,
                                                result,
                                                error: None,
                                            }),
                                        }),
                                        (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                                    ));
                                }
                                if item.item_type.as_deref() == Some("function_call") {
                                    let tool_call_id = item.call_id.clone().unwrap_or_default();
                                    if !active_tool_call_ids.contains(&tool_call_id) {
                                        log::debug!("responses: function_call output_item.done skipped (already handled) id={tool_call_id}");
                                        continue;
                                    }
                                    let tool_name = item.name.clone().unwrap_or_else(|| {
                                        fn_call_args.get(&tool_call_id)
                                            .map(|(n, _)| n.clone())
                                            .unwrap_or_default()
                                    });
                                    let arguments_str = item.arguments.clone().unwrap_or_else(|| {
                                        fn_call_args.get(&tool_call_id)
                                            .map(|(_, a)| a.clone())
                                            .unwrap_or_default()
                                    });
                                    let arguments: serde_json::Value = serde_json::from_str(&arguments_str)
                                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                                    log::debug!("responses: function_call done via output_item id={tool_call_id} name={tool_name}");
                                    fn_call_args.remove(&tool_call_id);
                                    active_tool_call_ids.retain(|id| id != &tool_call_id);
                                    return Some((
                                        Ok(StreamChunk {
                                            delta: String::new(),
                                            accumulated: accumulated.clone(),
                                            thinking_delta: None,
                                            accumulated_thinking: None,
                                            usage: None,
                                            tool_call_event: Some(ToolCallEvent::ArgumentsComplete {
                                                tool_call_id,
                                                tool_name,
                                                arguments,
                                            }),
                                        }),
                                        (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                                    ));
                                }
                            }
                            continue;
                        }
                        "response.web_search_call.completed"
                        | "response.web_search_call.in_progress"
                        | "response.web_search_call.searching" => {
                            continue;
                        }
                        "response.completed" => {
                            let usage = event
                                .response
                                .and_then(|r| r.usage)
                                .map(|u| TokenUsage {
                                    prompt_tokens: u.input_tokens,
                                    completion_tokens: u.output_tokens,
                                });
                            if usage.is_some() {
                                return Some((
                                    Ok(StreamChunk {
                                        delta: String::new(),
                                        accumulated: accumulated.clone(),
                                        thinking_delta: None,
                                        accumulated_thinking: None,
                                        usage,
                                        tool_call_event: None,
                                    }),
                                    (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                                ));
                            }
                            continue;
                        }
                        _ => continue,
                    }
                }
                Some(Err(e)) => {
                    return Some((
                        Err(AiError::Stream(e.to_string())),
                        (sse_stream, accumulated, accumulated_thinking, active_tool_call_ids, fn_call_args, item_id_to_call_id),
                    ));
                }
                None => return None,
            }
        }
    },
)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn run(events: &[&str]) -> Vec<StreamChunk> {
        let source = futures::stream::iter(
            events
                .iter()
                .map(|e| Ok::<String, String>(e.to_string()))
                .collect::<Vec<_>>(),
        );
        Box::pin(parse_responses_events(source))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect()
    }

    fn tool_events(chunks: &[StreamChunk]) -> Vec<String> {
        chunks
            .iter()
            .filter_map(|c| c.tool_call_event.as_ref())
            .map(|e| match e {
                ToolCallEvent::Started { tool_call_id, tool_name } => {
                    format!("started:{tool_call_id}:{tool_name}")
                }
                ToolCallEvent::ArgumentsComplete { tool_call_id, tool_name, arguments } => {
                    format!("complete:{tool_call_id}:{tool_name}:{arguments}")
                }
                ToolCallEvent::Done { tool_call_id, result, error } => {
                    format!("done:{tool_call_id}:{result:?}:{error:?}")
                }
            })
            .collect()
    }

    const ADDED_1: &str = r#"{"type":"response.output_item.added","item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"lookup"}}"#;
    const ADDED_2: &str = r#"{"type":"response.output_item.added","item":{"type":"function_call","id":"fc_2","call_id":"call_2","name":"fetch"}}"#;

    #[tokio::test]
    async fn text_deltas_accumulate_and_usage_follows() {
        let chunks = run(&[
            r#"{"type":"response.output_text.delta","delta":"Hel"}"#,
            r#"{"type":"response.output_text.delta","delta":""}"#,
            "not json",
            r#"{"type":"response.in_progress"}"#,
            r#"{"type":"response.output_text.delta","delta":"lo"}"#,
            r#"{"type":"response.completed","response":{"usage":{"input_tokens":5,"output_tokens":3}}}"#,
        ])
        .await;

        assert_eq!(chunks.len(), 3);
        assert_eq!((chunks[0].delta.as_str(), chunks[0].accumulated.as_str()), ("Hel", "Hel"));
        assert_eq!((chunks[1].delta.as_str(), chunks[1].accumulated.as_str()), ("lo", "Hello"));
        assert!(chunks[0].usage.is_none() && chunks[1].usage.is_none());
        assert_eq!((chunks[2].delta.as_str(), chunks[2].accumulated.as_str()), ("", "Hello"));
        let usage = chunks[2].usage.as_ref().unwrap();
        assert_eq!((usage.prompt_tokens, usage.completion_tokens), (5, 3));
    }

    #[tokio::test]
    async fn completed_without_usage_emits_nothing() {
        let chunks = run(&[r#"{"type":"response.completed","response":{}}"#]).await;
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn reasoning_summary_deltas_accumulate_beside_text() {
        let chunks = run(&[
            r#"{"type":"response.reasoning_summary_text.delta","delta":"think"}"#,
            r#"{"type":"response.reasoning_summary_text.delta","delta":"ing"}"#,
            r#"{"type":"response.reasoning_summary_text.delta","delta":""}"#,
            r#"{"type":"response.output_text.delta","delta":"ans"}"#,
        ])
        .await;

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].delta, "");
        assert_eq!(chunks[0].thinking_delta.as_deref(), Some("think"));
        assert_eq!(chunks[0].accumulated_thinking.as_deref(), Some("think"));
        assert_eq!(chunks[1].thinking_delta.as_deref(), Some("ing"));
        assert_eq!(chunks[1].accumulated_thinking.as_deref(), Some("thinking"));
        assert_eq!(chunks[2].delta, "ans");
        assert_eq!(chunks[2].accumulated, "ans");
        assert_eq!(chunks[2].thinking_delta, None);
        assert_eq!(chunks[2].accumulated_thinking.as_deref(), Some("thinking"));
    }

    #[tokio::test]
    async fn one_tool_call_streams_started_then_arguments_complete() {
        let chunks = run(&[
            ADDED_1,
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_1","delta":"{\"q\":"}"#,
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_1","delta":"\"x\"}"}"#,
            r#"{"type":"response.function_call_arguments.done","item_id":"fc_1"}"#,
        ])
        .await;

        assert_eq!(
            tool_events(&chunks),
            vec!["started:call_1:lookup", r#"complete:call_1:lookup:{"q":"x"}"#]
        );
        assert_eq!(chunks[0].delta, "");
        assert_eq!(chunks[0].accumulated, "{{tool_call:call_1}}");
        assert_eq!(chunks[1].accumulated, "{{tool_call:call_1}}");
    }

    #[tokio::test]
    async fn arguments_done_payload_wins_over_accumulated_deltas() {
        let chunks = run(&[
            ADDED_1,
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_1","delta":"{\"q\":\"old\"}"}"#,
            r#"{"type":"response.function_call_arguments.done","item_id":"fc_1","name":"renamed","arguments":"{\"q\":\"new\"}"}"#,
        ])
        .await;

        assert_eq!(
            tool_events(&chunks),
            vec!["started:call_1:lookup", r#"complete:call_1:renamed:{"q":"new"}"#]
        );
    }

    #[tokio::test]
    async fn two_interleaved_tool_calls_complete_independently() {
        let chunks = run(&[
            ADDED_1,
            ADDED_2,
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_1","delta":"{\"a\":"}"#,
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_2","delta":"{\"b\":2}"}"#,
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_1","delta":"1}"}"#,
            r#"{"type":"response.function_call_arguments.done","item_id":"fc_2"}"#,
            r#"{"type":"response.function_call_arguments.done","item_id":"fc_1"}"#,
        ])
        .await;

        assert_eq!(
            tool_events(&chunks),
            vec![
                "started:call_1:lookup",
                "started:call_2:fetch",
                r#"complete:call_2:fetch:{"b":2}"#,
                r#"complete:call_1:lookup:{"a":1}"#,
            ]
        );
        assert_eq!(chunks[1].accumulated, "{{tool_call:call_1}}{{tool_call:call_2}}");
    }

    #[tokio::test]
    async fn output_item_done_after_arguments_done_is_not_emitted_again() {
        let chunks = run(&[
            ADDED_1,
            r#"{"type":"response.function_call_arguments.done","item_id":"fc_1","arguments":"{\"q\":\"x\"}"}"#,
            r#"{"type":"response.output_item.done","item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"lookup","arguments":"{\"q\":\"x\"}"}}"#,
        ])
        .await;

        assert_eq!(
            tool_events(&chunks),
            vec!["started:call_1:lookup", r#"complete:call_1:lookup:{"q":"x"}"#]
        );
    }

    #[tokio::test]
    async fn output_item_done_completes_a_call_still_active() {
        let chunks = run(&[
            ADDED_1,
            r#"{"type":"response.output_item.done","item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"lookup","arguments":"{\"q\":\"x\"}"}}"#,
            r#"{"type":"response.output_item.done","item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"lookup","arguments":"{\"q\":\"x\"}"}}"#,
        ])
        .await;

        assert_eq!(
            tool_events(&chunks),
            vec!["started:call_1:lookup", r#"complete:call_1:lookup:{"q":"x"}"#]
        );
    }

    #[tokio::test]
    async fn unparseable_arguments_become_empty_object() {
        let via_arguments_done = run(&[
            ADDED_1,
            r#"{"type":"response.function_call_arguments.done","item_id":"fc_1","arguments":"{not json"}"#,
        ])
        .await;
        let via_output_item = run(&[
            ADDED_1,
            r#"{"type":"response.output_item.done","item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"lookup","arguments":"{not json"}}"#,
        ])
        .await;

        let expected = vec!["started:call_1:lookup", "complete:call_1:lookup:{}"];
        assert_eq!(tool_events(&via_arguments_done), expected);
        assert_eq!(tool_events(&via_output_item), expected);
    }

    #[tokio::test]
    async fn web_search_call_emits_started_then_done_with_queries() {
        let chunks = run(&[
            r#"{"type":"response.output_item.added","item":{"type":"web_search_call","id":"ws_1"}}"#,
            r#"{"type":"response.web_search_call.searching","item_id":"ws_1"}"#,
            r#"{"type":"response.output_item.done","item":{"type":"web_search_call","id":"ws_1","action":{"queries":["a","b"]}}}"#,
        ])
        .await;

        assert_eq!(
            tool_events(&chunks),
            vec!["started:ws_1:web_search", r#"done:ws_1:Some("a\nb"):None"#]
        );
        assert_eq!(chunks[0].accumulated, "{{tool_call:ws_1}}");
    }

    #[tokio::test]
    async fn source_error_becomes_stream_error() {
        let source = futures::stream::iter(vec![Err::<String, String>("boom".into())]);
        let results = Box::pin(parse_responses_events(source)).collect::<Vec<_>>().await;

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], Err(AiError::Stream(m)) if m == "boom"));
    }
}
