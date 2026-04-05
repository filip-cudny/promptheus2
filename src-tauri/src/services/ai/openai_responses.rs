use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tokio_stream::StreamExt;

use crate::models::message::{ContentPart, MessageContent, ProcessedMessage};
use crate::models::settings::ModelConfig;

use super::provider::{AiProvider, CompletionRequest, StreamChunk, TokenUsage};
use super::sse::parse_sse_stream;
use super::AiError;

pub struct OpenAiResponsesProvider {
    http_client: reqwest::Client,
    base_url: String,
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
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AiError::Request(e.to_string()))?;

        Ok(Self {
            http_client,
            base_url,
        })
    }
}

fn to_responses_message(msg: &ProcessedMessage) -> serde_json::Value {
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
    serde_json::json!({
        "role": msg.role,
        "content": content,
    })
}

fn build_request_body(request: &CompletionRequest, stream: bool) -> serde_json::Value {
    let mut instructions: Option<String> = None;
    let input_messages: Vec<serde_json::Value> = request
        .messages
        .iter()
        .filter(|m| {
            if m.role == "system" {
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
        .map(|m| to_responses_message(m))
        .collect();

    let mut body = serde_json::json!({
        "model": request.model,
        "input": input_messages,
        "stream": stream,
    });

    let obj = body.as_object_mut().unwrap();
    if let Some(instructions) = instructions {
        obj.insert("instructions".into(), serde_json::json!(instructions));
    }

    let obj = body.as_object_mut().unwrap();

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
        let body = build_request_body(&request, false);

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

        let stream = futures::stream::unfold(
            (sse_stream, String::new(), String::new()),
            |(mut sse_stream, mut accumulated, mut accumulated_thinking)| async move {
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

                            log::trace!("responses: event_type={}, has_delta={}", event.event_type, event.delta.is_some());

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
                                        }),
                                        (sse_stream, accumulated, accumulated_thinking),
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
                                        }),
                                        (sse_stream, accumulated, accumulated_thinking),
                                    ));
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
                                            }),
                                            (sse_stream, accumulated, accumulated_thinking),
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
                                (sse_stream, accumulated, accumulated_thinking),
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
