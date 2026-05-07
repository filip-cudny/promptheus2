pub mod capabilities;
pub mod openai;
pub mod openai_responses;
pub mod provider;
pub mod sse;
pub mod tools;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;

use crate::models::message::ProcessedMessage;
use crate::models::settings::{ApiMode, ModelConfig, ModelParameters, Provider};

use self::capabilities::{resolve, ModelCapabilities, ReasoningMode};
use self::openai::OpenAiProvider;
use self::openai_responses::OpenAiResponsesProvider;
use self::provider::{AiProvider, CompletionRequest, StreamChunk};
use self::tools::ToolRegistry;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("model unavailable: {0}")]
    ModelUnavailable(String),

    #[error("authentication failed: {0}")]
    Authentication(String),

    #[error("connection failed: {0}")]
    Connection(String),

    #[error("rate limit exceeded")]
    RateLimit,

    #[error("API error (status {status}): {message}")]
    ApiStatus { status: u16, message: String },

    #[error("stream error: {0}")]
    Stream(String),

    #[error("request failed: {0}")]
    Request(String),
}

struct ProviderEntry {
    provider: Box<dyn AiProvider>,
    model_name: String,
    parameters: ModelParameters,
    provider_type: Provider,
    api_mode: ApiMode,
    capabilities: ModelCapabilities,
}

struct AiServiceInner {
    providers: HashMap<String, ProviderEntry>,
    unavailable_models: HashMap<String, String>,
}

#[derive(Clone)]
pub struct AiService {
    inner: Arc<AiServiceInner>,
}

impl AiService {
    pub fn new(models: &[ModelConfig]) -> Self {
        let mut providers = HashMap::new();
        let mut unavailable_models = HashMap::new();

        for model in models {
            if !model.is_text() {
                continue;
            }
            let provider_type = model.provider.clone().unwrap_or_default();
            match provider_type {
                Provider::Openai => {
                    let api_mode = model.api_mode.clone().unwrap_or_default();
                    log::info!(
                        "model '{}': provider=openai, api_mode={api_mode:?}",
                        model.display_name
                    );
                    let result: Result<Box<dyn AiProvider>, AiError> = match api_mode {
                        ApiMode::Responses => OpenAiResponsesProvider::new(model).map(|p| Box::new(p) as Box<dyn AiProvider>),
                        ApiMode::Completions => OpenAiProvider::new(model).map(|p| Box::new(p) as Box<dyn AiProvider>),
                    };
                    match result {
                        Ok(provider) => {
                            let capabilities = resolve(model);
                            providers.insert(
                                model.id.clone(),
                                ProviderEntry {
                                    provider,
                                    model_name: model.model.clone(),
                                    parameters: model.parameters.clone().unwrap_or_default(),
                                    provider_type,
                                    api_mode: api_mode.clone(),
                                    capabilities,
                                },
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "model '{}' unavailable: {}",
                                model.display_name,
                                e
                            );
                            unavailable_models.insert(model.id.clone(), e.to_string());
                        }
                    }
                }
                Provider::Anthropic => {
                    unavailable_models
                        .insert(model.id.clone(), "Anthropic provider not yet supported".into());
                }
                Provider::Gemini => {
                    unavailable_models
                        .insert(model.id.clone(), "Gemini provider not yet supported".into());
                }
                Provider::ElevenLabs => {
                    unavailable_models.insert(
                        model.id.clone(),
                        "ElevenLabs provider is speech-to-text only".into(),
                    );
                }
            }
        }

        Self {
            inner: Arc::new(AiServiceInner {
                providers,
                unavailable_models,
            }),
        }
    }

    pub async fn complete(
        &self,
        model_id: &str,
        messages: Vec<ProcessedMessage>,
    ) -> Result<String, AiError> {
        let entry = self.get_provider(model_id)?;
        let parameters = normalize_params(&entry.parameters, &entry.capabilities, &entry.model_name);
        let request = CompletionRequest {
            model: entry.model_name.clone(),
            messages,
            parameters,
            tool_payloads: vec![],
        };
        entry.provider.complete(request).await
    }

    pub async fn complete_stream(
        &self,
        model_id: &str,
        messages: Vec<ProcessedMessage>,
        parameter_overrides: Option<ModelParameters>,
        tools_override: Option<Vec<String>>,
        mcp_tools: Vec<rmcp::model::Tool>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError> {
        let entry = self.get_provider(model_id)?;
        let parameters = match parameter_overrides {
            Some(overrides) => merge_parameters(&entry.parameters, &overrides),
            None => entry.parameters.clone(),
        };
        let parameters = normalize_params(&parameters, &entry.capabilities, &entry.model_name);

        let mut tool_payloads: Vec<serde_json::Value> = match tools_override {
            Some(ref requested) => {
                let built_in = ToolRegistry::resolve_tools(
                    requested,
                    &entry.provider_type,
                    &entry.api_mode,
                );
                built_in
                    .iter()
                    .map(|t| ToolRegistry::to_request_payload(t, &entry.provider_type, &entry.api_mode))
                    .collect()
            }
            None => vec![],
        };

        for mcp_tool in &mcp_tools {
            tool_payloads.push(tools::mcp_tool_to_payload(
                mcp_tool,
                &entry.provider_type,
                &entry.api_mode,
            ));
        }

        let request = CompletionRequest {
            model: entry.model_name.clone(),
            messages,
            parameters,
            tool_payloads,
        };
        entry.provider.complete_stream(request).await
    }

    pub fn has_model(&self, model_id: &str) -> bool {
        self.inner.providers.contains_key(model_id)
    }

    pub fn get_available_models(&self) -> Vec<String> {
        self.inner.providers.keys().cloned().collect()
    }

    pub fn get_unavailable_models(&self) -> &HashMap<String, String> {
        &self.inner.unavailable_models
    }

    pub fn model_capabilities(&self, model_id: &str) -> Result<ModelCapabilities, AiError> {
        let entry = self.get_provider(model_id)?;
        Ok(entry.capabilities.clone())
    }

    fn get_provider(&self, model_id: &str) -> Result<&ProviderEntry, AiError> {
        if let Some(reason) = self.inner.unavailable_models.get(model_id) {
            return Err(AiError::ModelUnavailable(reason.clone()));
        }
        self.inner
            .providers
            .get(model_id)
            .ok_or_else(|| AiError::ModelNotFound(model_id.to_string()))
    }
}

fn merge_parameters(base: &ModelParameters, overrides: &ModelParameters) -> ModelParameters {
    let mut extra = base.extra.clone();
    extra.extend(overrides.extra.clone());
    ModelParameters {
        temperature: overrides.temperature.or(base.temperature),
        max_tokens: overrides.max_tokens.or(base.max_tokens),
        top_p: overrides.top_p.or(base.top_p),
        frequency_penalty: overrides.frequency_penalty.or(base.frequency_penalty),
        presence_penalty: overrides.presence_penalty.or(base.presence_penalty),
        reasoning_effort: overrides.reasoning_effort.clone().or(base.reasoning_effort.clone()),
        extra,
    }
}

fn normalize_params(
    params: &ModelParameters,
    caps: &ModelCapabilities,
    model: &str,
) -> ModelParameters {
    let mut result = params.clone();

    if let Some(ref effort) = result.reasoning_effort {
        let keep = match &caps.reasoning {
            ReasoningMode::Effort { .. } => caps.accepts_effort(effort),
            ReasoningMode::BudgetTokens { .. } | ReasoningMode::Toggle => true,
            ReasoningMode::Unsupported => false,
        };
        if !keep {
            log::warn!(
                "{model}: dropping reasoning_effort='{effort}' — not supported by this model",
            );
            result.reasoning_effort = None;
        }
    }

    for (key, value) in &params.extra {
        log::debug!("{model}: passing through custom parameter '{key}': {value}");
    }

    result
}
