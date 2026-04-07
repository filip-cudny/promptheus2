use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tokio_stream::StreamExt;

use crate::models::settings::ModelConfig;

use crate::models::settings::{ApiMode, Provider};

use super::provider::{AiProvider, CompletionRequest, StreamChunk, ToolCallEvent, TokenUsage};
use super::sse::parse_sse_stream;
use super::tools::ToolRegistry;
use super::AiError;

pub struct OpenAiProvider {
    http_client: reqwest::Client,
    base_url: String,
}

impl OpenAiProvider {
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
    if let Some(ref re) = request.parameters.reasoning_effort {
        obj.insert("reasoning_effort".into(), serde_json::json!(re));
    }

    for (key, value) in &request.parameters.extra {
        obj.insert(key.clone(), value.clone());
    }

    if !request.tools.is_empty() {
        let tools_json: Vec<serde_json::Value> = request
            .tools
            .iter()
            .map(|t| ToolRegistry::to_request_payload(t, &Provider::Openai, &ApiMode::Completions))
            .collect();
        obj.insert("tools".into(), serde_json::json!(tools_json));
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

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn supported_params(&self) -> &'static [&'static str] {
        &["temperature", "max_tokens", "top_p", "frequency_penalty", "presence_penalty", "reasoning_effort"]
    }

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

        struct UnfoldState {
            sse_stream: Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>>,
            accumulated: String,
            accumulated_thinking: String,
            tool_calls: Vec<AccumulatedToolCall>,
            pending_tool_done_events: Vec<ToolCallEvent>,
        }

        struct AccumulatedToolCall {
            id: String,
            name: String,
            arguments: String,
        }

        let state = UnfoldState {
            sse_stream,
            accumulated: String::new(),
            accumulated_thinking: String::new(),
            tool_calls: Vec::new(),
            pending_tool_done_events: Vec::new(),
        };

        let stream = futures::stream::unfold(state, |mut state| async move {
            if let Some(event) = state.pending_tool_done_events.pop() {
                return Some((
                    Ok(StreamChunk {
                        delta: String::new(),
                        accumulated: state.accumulated.clone(),
                        thinking_delta: None,
                        accumulated_thinking: None,
                        usage: None,
                        tool_call_event: Some(event),
                    }),
                    state,
                ));
            }

            loop {
                match state.sse_stream.next().await {
                    Some(Ok(data)) => {
                        let chunk: ChatCompletionChunk = match serde_json::from_str(&data) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                        let usage = chunk.usage.map(|u| TokenUsage {
                            prompt_tokens: u.prompt_tokens,
                            completion_tokens: u.completion_tokens,
                        });

                        let choice = chunk.choices.first();
                        let choice_delta = choice.map(|c| &c.delta);
                        let finish_reason = choice.and_then(|c| c.finish_reason.as_deref());

                        if let Some(tool_call_deltas) = choice_delta.and_then(|d| d.tool_calls.as_ref()) {
                            for tc_delta in tool_call_deltas {
                                if let Some(ref id) = tc_delta.id {
                                    let name = tc_delta
                                        .function
                                        .as_ref()
                                        .and_then(|f| f.name.clone())
                                        .unwrap_or_default();

                                    while state.tool_calls.len() <= tc_delta.index {
                                        state.tool_calls.push(AccumulatedToolCall {
                                            id: String::new(),
                                            name: String::new(),
                                            arguments: String::new(),
                                        });
                                    }
                                    state.tool_calls[tc_delta.index] = AccumulatedToolCall {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments: String::new(),
                                    };

                                    let marker = format!("{{{{tool_call:{id}}}}}");
                                    state.accumulated.push_str(&marker);

                                    return Some((
                                        Ok(StreamChunk {
                                            delta: marker,
                                            accumulated: state.accumulated.clone(),
                                            thinking_delta: None,
                                            accumulated_thinking: None,
                                            usage: None,
                                            tool_call_event: Some(ToolCallEvent::Started {
                                                tool_call_id: id.clone(),
                                                tool_name: name,
                                            }),
                                        }),
                                        state,
                                    ));
                                }

                                if let Some(ref args) = tc_delta.function.as_ref().and_then(|f| f.arguments.clone()) {
                                    if tc_delta.index < state.tool_calls.len() {
                                        state.tool_calls[tc_delta.index].arguments.push_str(args);
                                    }
                                }
                            }
                        }

                        if finish_reason == Some("tool_calls") {
                            for tc in &state.tool_calls {
                                if !tc.id.is_empty() {
                                    state.pending_tool_done_events.push(ToolCallEvent::Done {
                                        tool_call_id: tc.id.clone(),
                                        result: Some(tc.arguments.clone()),
                                        error: None,
                                    });
                                }
                            }
                            state.tool_calls.clear();

                            if let Some(event) = state.pending_tool_done_events.pop() {
                                return Some((
                                    Ok(StreamChunk {
                                        delta: String::new(),
                                        accumulated: state.accumulated.clone(),
                                        thinking_delta: None,
                                        accumulated_thinking: None,
                                        usage,
                                        tool_call_event: Some(event),
                                    }),
                                    state,
                                ));
                            }
                        }

                        let delta = choice_delta
                            .and_then(|d| d.content.as_deref())
                            .unwrap_or("");

                        let thinking = choice_delta
                            .and_then(|d| d.reasoning_content.as_deref().or(d.reasoning.as_deref()))
                            .unwrap_or("");

                        if delta.is_empty() && thinking.is_empty() && usage.is_none() {
                            continue;
                        }

                        state.accumulated.push_str(delta);
                        state.accumulated_thinking.push_str(thinking);

                        let thinking_delta = if thinking.is_empty() { None } else { Some(thinking.to_string()) };
                        let acc_thinking = if state.accumulated_thinking.is_empty() { None } else { Some(state.accumulated_thinking.clone()) };

                        return Some((
                            Ok(StreamChunk {
                                delta: delta.to_string(),
                                accumulated: state.accumulated.clone(),
                                thinking_delta,
                                accumulated_thinking: acc_thinking,
                                usage,
                                tool_call_event: None,
                            }),
                            state,
                        ));
                    }
                    Some(Err(e)) => {
                        return Some((Err(AiError::Stream(e.to_string())), state));
                    }
                    None => return None,
                }
            }
        });

        Ok(Box::pin(stream))
    }
}
