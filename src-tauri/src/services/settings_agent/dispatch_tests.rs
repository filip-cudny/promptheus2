use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tempfile::TempDir;

use super::dispatch::{Context, Dispatcher, Proposal, ProposalSink, SecretAnswer, SecretPrompt};
use crate::models::settings::{EnvVar, McpServerConfig, ModelConfig, ModelType, Provider, Settings};
use crate::services::ai::mock::{MockProvider, MockReply};
use crate::services::ai::{AiError, AiService};
use crate::services::execution::types::PendingToolCall;
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::secrets;

const SECRETS: [&str; 3] = ["sk-env-111", "tok-222", "sk-literal-333"];

fn model(id: &str, api_key: &str) -> ModelConfig {
    ModelConfig {
        id: id.to_string(),
        model: "m".to_string(),
        display_name: format!("Model {id}"),
        model_type: ModelType::Text,
        provider: Some(Provider::default()),
        group: None,
        api_key: Some(api_key.to_string()),
        base_url: None,
        parameters: None,
        context_window_size: None,
        api_mode: None,
        capabilities: None,
        store: true,
    }
}

fn fixture_settings() -> Settings {
    let mut settings = Settings::default();
    settings.models = vec![model("env-model", "${OPENAI_KEY}"), model("literal-model", "sk-literal-333")];
    settings.mcp_servers = HashMap::from([(
        "tools".to_string(),
        McpServerConfig {
            command: "run-tools".into(),
            args: vec![],
            env: HashMap::from([("TOKEN".to_string(), "${MCP_TOKEN}".to_string())]),
            timeout_secs: 30,
            tool_timeouts: HashMap::new(),
        },
    )]);
    settings.surfaces.chat.generation.model_id = Some("env-model".into());
    settings.env_vars = vec![
        EnvVar {
            name: "OPENAI_KEY".into(),
            description: "OpenAI key".into(),
            secret: true,
            renamed_from: vec![],
        },
        EnvVar {
            name: "TOOLS_DIR".into(),
            description: "Tools path".into(),
            secret: false,
            renamed_from: vec![],
        },
    ];
    settings
}

struct Sink(std::sync::Mutex<Vec<String>>);

impl ProposalSink for Sink {
    fn proposed(&self, proposal: &Proposal) {
        self.0.lock().unwrap().push(proposal.id.clone());
    }
}

struct SavingPrompt {
    config_dir: std::path::PathBuf,
}

#[async_trait]
impl SecretPrompt for SavingPrompt {
    async fn ask(&self, name: &str, _purpose: &str) -> SecretAnswer {
        secrets::save_secret(&self.config_dir, "host", name, "sk-new-444").unwrap();
        SecretAnswer::Saved
    }
}

struct CancellingPrompt;

#[async_trait]
impl SecretPrompt for CancellingPrompt {
    async fn ask(&self, _name: &str, _purpose: &str) -> SecretAnswer {
        SecretAnswer::Cancelled
    }
}

fn fixture(prompt: impl FnOnce(&std::path::Path) -> Arc<dyn SecretPrompt>) -> (TempDir, Dispatcher, Arc<Sink>) {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".env"), "OPENAI_KEY=sk-env-111\nMCP_TOKEN=tok-222\n").unwrap();
    let skill_dir = dir.path().join("skills/old-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: old-skill\ndescription: Old\n---\n\nOld body.\n",
    )
    .unwrap();
    let settings = fixture_settings();
    std::fs::write(dir.path().join("settings.json"), serde_json::to_string_pretty(&settings).unwrap()).unwrap();
    let sink = Arc::new(Sink(Default::default()));
    let ctx = Context {
        settings: Box::new(move || settings.clone()),
        config_dir: dir.path().to_path_buf(),
        skills_dir: dir.path().join("skills"),
        ai_factory: Box::new(|_| {
            let error = AiError::ApiStatus {
                status: 401,
                message: "Incorrect API key provided: sk-env-111".into(),
            };
            let (provider, _) = MockProvider::new(vec![MockReply::Error(error)]);
            AiService::with_provider("env-model", Box::new(provider))
        }),
        placeholders: PlaceholderContext::with_apps("app", "apps"),
        proposals: sink.clone(),
        secrets: prompt(dir.path()),
    };
    (dir, Dispatcher::new(ctx), sink)
}

async fn call(dispatcher: &Dispatcher, tool: &str, arguments: Value) -> (String, bool) {
    let result = dispatcher
        .dispatch(&PendingToolCall {
            tool_call_id: "call-1".into(),
            tool_name: tool.into(),
            arguments,
        })
        .await;
    (result.result_text, result.is_error)
}

fn assert_clean(text: &str) {
    for secret in SECRETS.iter().chain(&["sk-new-444"]) {
        assert!(!text.contains(secret), "{secret} leaked in {text}");
    }
}

fn update_model_row() -> Value {
    json!({"op": "update_model", "id": "env-model", "model": model("env-model", "${OPENAI_KEY}")})
}

#[tokio::test]
async fn every_read_variant_is_free_of_secrets() {
    let (_dir, dispatcher, _) = fixture(|_| Arc::new(CancellingPrompt));
    let reads = [
        json!({"what": "values"}),
        json!({"what": "models"}),
        json!({"what": "skills"}),
        json!({"what": "skill", "name": "old-skill"}),
        json!({"what": "prompt", "name": "system"}),
        json!({"what": "prompt", "name": "stt_keyterms"}),
        json!({"what": "missing_env"}),
        json!({"what": "env_vars"}),
    ];
    for arguments in reads {
        let (text, is_error) = call(&dispatcher, "settings_read", arguments.clone()).await;
        assert!(!is_error, "{arguments}: {text}");
        assert_clean(&text);
    }
}

#[tokio::test]
async fn env_vars_read_lists_state_and_usages_without_values() {
    let (_dir, dispatcher, _) = fixture(|_| Arc::new(CancellingPrompt));
    let (text, is_error) = call(&dispatcher, "settings_read", json!({"what": "env_vars"})).await;
    assert!(!is_error, "{text}");
    assert_clean(&text);
    let rows: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        for field in ["name", "description", "secret", "set", "used_in"] {
            assert!(row.get(field).is_some(), "{field} missing in {row}");
        }
        assert!(row["set"].is_boolean());
    }
    assert_eq!(rows[0]["name"], "OPENAI_KEY");
    assert_eq!(rows[0]["secret"], true);
    assert!(rows[0]["used_in"][0]["label"].as_str().unwrap().ends_with("API key"));
    assert_eq!(rows[1]["name"], "TOOLS_DIR");
    assert_eq!(rows[1]["secret"], false);
    assert_eq!(rows[1]["used_in"], json!([]));
}

#[tokio::test]
async fn values_read_keeps_env_references_and_hides_literal_keys() {
    let (_dir, dispatcher, _) = fixture(|_| Arc::new(CancellingPrompt));
    let (text, _) = call(&dispatcher, "settings_read", json!({"what": "values"})).await;
    assert!(text.contains("${OPENAI_KEY}"));
    assert!(text.contains("[redacted]"));
}

#[tokio::test]
async fn test_tool_redacts_provider_error() {
    let (_dir, dispatcher, _) = fixture(|_| Arc::new(CancellingPrompt));
    let (text, is_error) =
        call(&dispatcher, "settings_test", json!({"operations": [update_model_row()]})).await;
    assert!(!is_error);
    assert!(text.contains("[redacted]"), "{text}");
    assert_clean(&text);
}

#[tokio::test]
async fn propose_tool_redacts_provider_error_and_hands_proposal_to_sink() {
    let (_dir, dispatcher, sink) = fixture(|_| Arc::new(CancellingPrompt));
    let (text, is_error) = call(
        &dispatcher,
        "settings_propose_changes",
        json!({"request_summary": "update", "operations": [update_model_row()]}),
    )
    .await;
    assert!(!is_error);
    assert!(text.contains("[redacted]"), "{text}");
    assert_clean(&text);
    let proposal = dispatcher.proposal().expect("proposal held");
    assert_eq!(sink.0.lock().unwrap().as_slice(), [proposal.id.clone()]);
    assert!(text.contains(&proposal.id));
    assert!(!proposal.tests[&0].ok);
}

#[tokio::test]
async fn propose_with_row_errors_shows_no_card() {
    let (_dir, dispatcher, sink) = fixture(|_| Arc::new(CancellingPrompt));
    let bad = json!({"op": "delete_model", "id": "missing"});
    let (text, is_error) = call(
        &dispatcher,
        "settings_propose_changes",
        json!({"request_summary": "x", "operations": [bad]}),
    )
    .await;
    assert!(!is_error);
    assert!(text.contains("\"errors\""), "{text}");
    assert!(dispatcher.proposal().is_none());
    assert!(sink.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn secret_tool_returns_name_and_outcome_only() {
    let (_dir, dispatcher, _) = fixture(|dir| Arc::new(SavingPrompt { config_dir: dir.to_path_buf() }));
    let (text, is_error) = call(
        &dispatcher,
        "settings_secret",
        json!({"name": "OPENAI_KEY", "purpose": "OpenAI"}),
    )
    .await;
    crate::services::env_resolve::set_override("OPENAI_KEY", None);
    assert!(!is_error);
    assert!(text.contains("OPENAI_KEY") && text.contains("outcome"), "{text}");
    assert_clean(&text);
}

#[tokio::test]
async fn secret_tool_reports_exists_and_cancelled() {
    let (_dir, dispatcher, _) = fixture(|_| Arc::new(CancellingPrompt));
    let (existing, _) = call(&dispatcher, "settings_secret", json!({"name": "MCP_TOKEN", "purpose": "p"})).await;
    assert!(existing.contains("MCP_TOKEN") && existing.contains("outcome"), "{existing}");
    let (fresh, _) =
        call(&dispatcher, "settings_secret", json!({"name": "BRAND_NEW_UNSET_VAR_097", "purpose": "p"})).await;
    assert!(fresh.contains("BRAND_NEW_UNSET_VAR_097"), "{fresh}");
    assert_ne!(existing.replace("MCP_TOKEN", ""), fresh.replace("BRAND_NEW_UNSET_VAR_097", ""));
    let (invalid, is_error) = call(&dispatcher, "settings_secret", json!({"name": "1 bad", "purpose": "p"})).await;
    assert!(is_error, "{invalid}");
}

#[tokio::test]
async fn unknown_tool_and_bad_arguments_are_errors() {
    let (_dir, dispatcher, _) = fixture(|_| Arc::new(CancellingPrompt));
    let (_, unknown) = call(&dispatcher, "settings_delete_everything", json!({})).await;
    assert!(unknown);
    let (_, bad) = call(&dispatcher, "settings_read", json!({"what": "nonsense"})).await;
    assert!(bad);
    let (_, missing_name) = call(&dispatcher, "settings_read", json!({"what": "skill"})).await;
    assert!(missing_name);
}

#[test]
fn tool_names_are_valid_function_names() {
    for tool in super::tools::definitions() {
        assert!(tool.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        assert!(tool.name.starts_with("settings_"));
    }
}
