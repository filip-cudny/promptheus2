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
            .api_key
            .as_deref()
            .filter(|k| !k.is_empty())
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

    if let Some(ref effort) = request.parameters.reasoning_effort {
        let mut reasoning = serde_json::json!({ "effort": effort });
        if effort != "none" {
            reasoning["summary"] = serde_json::json!("auto");
        }
        obj.insert("reasoning".into(), reasoning);
    }

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
    status: Option<String>,
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
    fn supported_params(&self) -> &'static [&'static str] {
        &["temperature", "max_tokens", "top_p", "reasoning_effort"]
    }

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

        let sse_stream = parse_sse_stream(response);

        let stream = futures::stream::unfold(
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

                            log::debug!("responses: event_type={}", event.event_type);

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
                                                        tool_name: "builtin_web_search".to_string(),
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
        );

        Ok(Box::pin(stream))
    }
}
