use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::sync::Mutex as TokioMutex;

use super::apply::{apply, Context as ApplyContext};
use super::dispatch::SecretAnswer;
use super::engine::{
    lock, run_turn, Emit, EngineContext, SharedSession, PROPOSAL_EVENT, SECRET_REQUEST_EVENT,
};
use super::plan::{plan, Current};
use crate::models::capabilities::{Effort, ModelCapabilities, ReasoningMode};
use crate::models::settings::{ModelConfig, ModelType, Provider, Settings};
use crate::services::ai::mock::{MockProvider, MockReply};
use crate::services::ai::{AiError, AiService};
use crate::services::change_log::{change_log_dir, load_all};
use crate::services::config::ConfigService;
use crate::services::env_resolve::set_override;
use crate::services::execution::lifecycle::{ExecutionSnapshot, LiveExecution};
use crate::services::placeholder_registry::PlaceholderContext;

const SECRETS: [&str; 3] = ["sk-env-111", "sk-literal-333", "sk-new-444"];

fn model(id: &str, display_name: &str, api_key: &str) -> ModelConfig {
    ModelConfig {
        id: id.to_string(),
        model: "m".to_string(),
        display_name: display_name.to_string(),
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

fn fixture_settings(chat_model: Option<&str>) -> Settings {
    let mut settings = Settings::default();
    settings.models = vec![
        model("env-model", "Env", "${OPENAI_KEY}"),
        model("literal-model", "Literal", "sk-literal-333"),
    ];
    settings.surfaces.chat.generation.model_id = chat_model.map(str::to_string);
    settings
}

struct SavingFrontend {
    session: SharedSession,
    config_dir: PathBuf,
    events: Mutex<Vec<(String, Value)>>,
}

impl Emit for SavingFrontend {
    fn emit(&self, event: &str, payload: Value) {
        self.events.lock().unwrap().push((event.to_string(), payload.clone()));
        if event == SECRET_REQUEST_EVENT {
            let name = payload["name"].as_str().unwrap();
            crate::services::secrets::save_secret(&self.config_dir, "host", name, "sk-new-444").unwrap();
            let id = payload["request_id"].as_str().unwrap();
            assert!(lock(&self.session).resolve_secret(id, SecretAnswer::Saved));
        }
    }
}

struct Run {
    dir: TempDir,
    session: SharedSession,
    frontend: Arc<SavingFrontend>,
    requests: Arc<Mutex<Vec<String>>>,
    live: Arc<TokioMutex<LiveExecution>>,
    outcome: crate::Result<()>,
}

fn call(id: &str, tool: &str, arguments: Value) -> MockReply {
    MockReply::ToolCalls(vec![(id.to_string(), tool.to_string(), arguments)])
}

fn scripted_turns() -> Vec<MockReply> {
    let operations = json!([
        {"op": "add_model", "model": model("new-model", "New", "${NEW_KEY}")},
        {"op": "update_model", "id": "literal-model", "model": model("literal-model", "Literal renamed", "")},
    ]);
    vec![
        call("c1", "settings_read", json!({"what": "values"})),
        call("c2", "settings_read", json!({"what": "models"})),
        call("c3", "settings_secret", json!({"name": "NEW_KEY", "purpose": "new model"})),
        call(
            "c4",
            "settings_propose_changes",
            json!({"operations": operations}),
        ),
        MockReply::Text("Proposed two changes.".to_string()),
    ]
}

async fn run(chat_model: Option<&str>, script: Vec<MockReply>) -> Run {
    let (provider, requests) = MockProvider::new(script);
    let ai = AiService::with_provider("env-model", Box::new(provider));
    run_with(fixture_settings(chat_model), ai, requests).await
}

async fn run_with(settings: Settings, ai: AiService, requests: Arc<Mutex<Vec<String>>>) -> Run {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".env"), "OPENAI_KEY=sk-env-111\n").unwrap();
    std::fs::write(dir.path().join("settings.json"), serde_json::to_string_pretty(&settings).unwrap()).unwrap();
    std::fs::create_dir_all(dir.path().join("skills")).unwrap();

    let session = SharedSession::default();
    let frontend = Arc::new(SavingFrontend {
        session: session.clone(),
        config_dir: dir.path().to_path_buf(),
        events: Mutex::default(),
    });
    let ctx = EngineContext {
        settings: Arc::new(move || settings.clone()),
        config_dir: dir.path().to_path_buf(),
        skills_dir: dir.path().join("skills"),
        ai,
        ai_factory: Arc::new(|_| {
            let error = AiError::ApiStatus {
                status: 401,
                message: "Incorrect API key provided: sk-env-111".into(),
            };
            let (provider, _) = MockProvider::new(vec![MockReply::Error(error)]);
            AiService::with_provider("literal-model", Box::new(provider))
        }),
        placeholders: PlaceholderContext::with_apps("app", "apps"),
        emit: frontend.clone(),
    };
    let live = Arc::new(TokioMutex::new(LiveExecution {
        snapshot: ExecutionSnapshot {
            execution_id: "req".into(),
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
    }));
    let outcome = run_turn(&session, "add a model with a new key", ctx, live.clone()).await;
    set_override("NEW_KEY", None);
    Run { dir, session, frontend, requests, live, outcome }
}

fn env_file(run: &Run) -> String {
    std::fs::read_to_string(run.dir.path().join(".env")).unwrap()
}

#[tokio::test]
async fn no_secret_reaches_any_provider_request() {
    let run = run(Some("env-model"), scripted_turns()).await;
    run.outcome.as_ref().unwrap();
    let requests = run.requests.lock().unwrap();
    assert_eq!(requests.len(), 5);
    for request in requests.iter() {
        for secret in SECRETS {
            assert!(!request.contains(secret), "{secret} leaked in {request}");
        }
    }
    assert!(requests[4].contains("[redacted]"), "provider error of the update row is redacted");
    assert!(env_file(&run).contains("NEW_KEY=\"sk-new-444\"") || env_file(&run).contains("NEW_KEY=sk-new-444"));
}

#[tokio::test]
async fn proposal_and_secret_request_reach_the_frontend() {
    let run = run(Some("env-model"), scripted_turns()).await;
    let events = run.frontend.events.lock().unwrap();
    let secret = events.iter().find(|(name, _)| name == SECRET_REQUEST_EVENT).unwrap();
    assert_eq!(secret.1["name"], "NEW_KEY");
    assert!(secret.1["exists"].is_boolean());
    let proposal = events.iter().find(|(name, _)| name == PROPOSAL_EVENT).unwrap();
    assert_eq!(proposal.1["rows"].as_array().unwrap().len(), 2);
    assert_eq!(proposal.1["rows"][1]["test"]["ok"], false);
    assert_eq!(proposal.1["rows"][0]["stale"], false);
    assert_eq!(lock(&run.session).last_reply.as_deref(), Some("Proposed two changes."));
    assert!(!proposal.1.to_string().contains("sk-new-444"));
}

#[tokio::test]
async fn rejecting_the_proposal_keeps_the_saved_secret() {
    let run = run(Some("env-model"), scripted_turns()).await;
    assert!(lock(&run.session).proposal.take().is_some());
    let env = env_file(&run);
    assert!(env.contains("sk-new-444"), "{env}");
    let settings = std::fs::read_to_string(run.dir.path().join("settings.json")).unwrap();
    assert!(!settings.contains("new-model"));
}

#[tokio::test]
async fn apply_stores_request_and_reply_in_the_log_entry() {
    let run = run(Some("env-model"), scripted_turns()).await;
    let (proposal, request, reply) = {
        let s = lock(&run.session);
        (s.proposal.clone().unwrap(), s.last_request.clone(), s.last_reply.clone())
    };
    let dir = run.dir.path();
    let mut config = ConfigService::load(dir).unwrap();
    let settings = config.settings().clone();
    let current = Current { settings: &settings, config_dir: dir };
    let planned = plan(&proposal.operations, &[true, true], &current).unwrap();
    let log_dir = change_log_dir(dir);
    let mut ctx = ApplyContext {
        config: &mut config,
        config_dir: dir,
        log_dir: &log_dir,
        host: "host",
        request,
        reply,
        tests: proposal.tests.clone(),
    };
    let id = apply(&planned, &current, &mut ctx).unwrap();
    let entry = load_all(&log_dir).into_iter().find(|e| e.id == id).unwrap();
    assert_eq!(entry.request.as_deref(), Some("add a model with a new key"));
    assert_eq!(entry.reply.as_deref(), Some("Proposed two changes."));
    assert_eq!(entry.operations.len(), 2);
}

#[tokio::test]
async fn six_model_field_rows_propose_without_errors() {
    let mut settings = fixture_settings(Some("env-model"));
    settings.models = (1..=6).map(|n| model(&format!("m{n}"), "M", "${OPENAI_KEY}")).collect();
    settings.models.push(model("env-model", "Env", "${OPENAI_KEY}"));
    let rows: Vec<Value> = (1..=6)
        .map(|n| json!({"op": "set_setting", "key": format!("models[m{n}].store"), "value": false}))
        .collect();
    let script = vec![
        call("c1", "settings_propose_changes", json!({"operations": rows})),
        MockReply::Text("Proposed.".to_string()),
    ];
    let (provider, requests) = MockProvider::new(script);
    let ai = AiService::with_provider("env-model", Box::new(provider));

    let run = run_with(settings, ai, requests).await;

    run.outcome.as_ref().unwrap();
    let events = run.frontend.events.lock().unwrap();
    let proposal = events.iter().find(|(name, _)| name == PROPOSAL_EVENT).unwrap();
    assert_eq!(proposal.1["rows"].as_array().unwrap().len(), 6);
    let requests = run.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(!requests[1].contains("\\\"errors\\\""), "{}", requests[1]);
}

#[tokio::test]
async fn no_chat_model_ends_the_turn_with_an_error_and_no_request() {
    let run = run(None, vec![MockReply::Text("never".into())]).await;
    assert!(run.outcome.is_err());
    assert!(run.requests.lock().unwrap().is_empty());
    let error = run.live.lock().await.snapshot.error.clone().unwrap();
    assert!(error.contains("chat model"), "{error}");
}

#[tokio::test]
async fn unavailable_chat_model_is_named_in_the_error() {
    let run = run(Some("ghost-model"), vec![MockReply::Text("never".into())]).await;
    assert!(run.outcome.is_err());
    assert!(run.requests.lock().unwrap().is_empty());
    let error = run.live.lock().await.snapshot.error.clone().unwrap();
    assert!(error.contains("ghost-model"), "{error}");
}

struct TwoModels {
    run: Run,
    chat_requests: Arc<Mutex<Vec<String>>>,
}

async fn run_on_agent_and_chat_models(
    agent_model: Option<&str>,
    agent_effort: Option<&str>,
    chat_effort: Option<&str>,
) -> TwoModels {
    let (agent_provider, agent_requests) = MockProvider::new(vec![MockReply::Text("done".into())]);
    let (chat_provider, chat_requests) = MockProvider::new(vec![MockReply::Text("done".into())]);
    let ai = AiService::with_providers(
        vec![("agent-model", Box::new(agent_provider)), ("chat-model", Box::new(chat_provider))],
        ModelCapabilities {
            reasoning: ReasoningMode::Effort { allowed: vec![Effort::Low, Effort::Medium, Effort::High] },
        },
    );
    let mut settings = fixture_settings(Some("chat-model"));
    settings.surfaces.settings_agent.generation.model_id = agent_model.map(str::to_string);
    settings.surfaces.settings_agent.generation.parameters.reasoning_effort = agent_effort.map(str::to_string);
    settings.surfaces.chat.generation.parameters.reasoning_effort = chat_effort.map(str::to_string);
    let run = run_with(settings, ai, agent_requests).await;
    TwoModels { run, chat_requests }
}

fn only_request(requests: &Arc<Mutex<Vec<String>>>) -> Value {
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    serde_json::from_str(&requests[0]).unwrap()
}

#[tokio::test]
async fn agent_model_serves_the_turn_when_set() {
    let two = run_on_agent_and_chat_models(Some("agent-model"), None, None).await;
    two.run.outcome.as_ref().unwrap();
    assert_eq!(only_request(&two.run.requests)["model"], "agent-model");
    assert!(two.chat_requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn empty_agent_model_falls_back_to_the_chat_model() {
    let two = run_on_agent_and_chat_models(None, None, None).await;
    two.run.outcome.as_ref().unwrap();
    assert_eq!(only_request(&two.chat_requests)["model"], "chat-model");
    assert!(two.run.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn agent_reasoning_effort_reaches_the_request_over_the_chat_one() {
    let two = run_on_agent_and_chat_models(Some("agent-model"), Some("high"), Some("low")).await;
    two.run.outcome.as_ref().unwrap();
    assert_eq!(only_request(&two.run.requests)["parameters"]["reasoning_effort"], "high");
}

#[tokio::test]
async fn chat_reasoning_effort_never_reaches_the_request() {
    let two = run_on_agent_and_chat_models(None, None, Some("low")).await;
    two.run.outcome.as_ref().unwrap();
    assert!(only_request(&two.chat_requests)["parameters"]["reasoning_effort"].is_null());
}

#[tokio::test]
async fn reasoning_item_of_a_tool_round_goes_back_on_the_assistant_message() {
    let item = json!({"type": "reasoning", "id": "rs_1", "summary": [], "encrypted_content": "enc"});
    let script = vec![
        MockReply::ReasoningThenToolCalls {
            reasoning_item: item.clone(),
            calls: vec![("c1".into(), "settings_read".into(), json!({}))],
        },
        MockReply::Text("done".into()),
    ];

    let run = run(Some("env-model"), script).await;

    run.outcome.as_ref().unwrap();
    let requests = run.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let second: Value = serde_json::from_str(&requests[1]).unwrap();
    let with_items: Vec<&Value> = second["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m.get("reasoning_items").is_some())
        .collect();
    assert_eq!(with_items.len(), 1);
    assert_eq!(with_items[0]["role"], "assistant");
    assert_eq!(with_items[0]["reasoning_items"], json!([item]));
    assert!(with_items[0]["tool_calls"].is_array());
}
