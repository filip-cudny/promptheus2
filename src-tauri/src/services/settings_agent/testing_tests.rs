use std::collections::{BTreeMap, HashMap};

use super::operations::Operation;
use super::plan::Plan;
use super::testing::test_row;
use crate::models::settings::{McpServerConfig, ModelConfig, ModelType, Provider, Settings};
use crate::models::skill::SkillFrontmatter;
use crate::services::ai::mock::{MockProvider, MockReply};
use crate::services::ai::{AiError, AiService};
use crate::services::placeholder_registry::PlaceholderContext;

fn model(id: &str, model_type: ModelType) -> ModelConfig {
    ModelConfig {
        id: id.to_string(),
        model: "m".to_string(),
        display_name: format!("Model {id}"),
        model_type,
        provider: Some(Provider::default()),
        group: None,
        api_key: None,
        base_url: None,
        parameters: None,
        context_window_size: None,
        api_mode: None,
        capabilities: None,
        store: true,
    }
}

fn plan_with(settings: Settings) -> Plan {
    Plan { settings, files: BTreeMap::new(), rows: vec![] }
}

fn ctx() -> PlaceholderContext {
    PlaceholderContext::with_apps("app", "apps")
}

fn skill_operation(model: Option<&str>) -> Operation {
    Operation::CreateSkill {
        slug: "s".into(),
        frontmatter: SkillFrontmatter {
            name: "s".into(),
            display_name: None,
            description: None,
            model: model.map(str::to_string),
            parameters: None,
            offered_in: None,
        },
        body: "Do it.".into(),
        sample_input: "sample input text".into(),
    }
}

#[tokio::test]
async fn text_model_ok() {
    let (provider, _) = MockProvider::new(vec![MockReply::Text("OK".into())]);
    let ai = AiService::with_provider("x", Box::new(provider));
    let operation = Operation::AddModel { model: model("x", ModelType::Text) };
    let result = test_row(&operation, &plan_with(Settings::default()), &ai, &ctx())
        .await
        .unwrap();
    assert!(result.ok);
    assert!(result.millis.is_some());
    assert_eq!(result.output.as_deref(), Some("OK"));
}

#[tokio::test]
async fn text_model_error_text_is_unchanged() {
    let error = AiError::ApiStatus { status: 400, message: "model 'x' does not exist".into() };
    let expected = error.to_string();
    let (provider, _) = MockProvider::new(vec![MockReply::Error(error)]);
    let ai = AiService::with_provider("x", Box::new(provider));
    let operation = Operation::AddModel { model: model("x", ModelType::Text) };
    let result = test_row(&operation, &plan_with(Settings::default()), &ai, &ctx())
        .await
        .unwrap();
    assert!(!result.ok);
    assert_eq!(result.error, Some(expected));
}

#[tokio::test]
async fn skill_test_sends_one_request_without_tools() {
    let (provider, requests) = MockProvider::new(vec![MockReply::Text("done".into())]);
    let ai = AiService::with_provider("x", Box::new(provider));
    let result = test_row(&skill_operation(Some("x")), &plan_with(Settings::default()), &ai, &ctx())
        .await
        .unwrap();
    assert!(result.ok);
    assert_eq!(result.output.as_deref(), Some("done"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let recorded: serde_json::Value = serde_json::from_str(&requests[0]).unwrap();
    assert_eq!(recorded["tool_payloads"], serde_json::json!([]));
    assert!(recorded["messages"].to_string().contains("sample input text"));
}

#[tokio::test]
async fn skill_without_model_uses_quick_actions_model() {
    let (provider, requests) = MockProvider::new(vec![MockReply::Text("done".into())]);
    let ai = AiService::with_provider("quick", Box::new(provider));
    let mut settings = Settings::default();
    settings.surfaces.quick_actions.generation.model_id = Some("quick".into());
    let result = test_row(&skill_operation(None), &plan_with(settings), &ai, &ctx())
        .await
        .unwrap();
    assert!(result.ok);
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn mcp_missing_command_fails_with_start_error() {
    let config = McpServerConfig {
        command: "/nonexistent/promptheus-test-binary".into(),
        args: vec![],
        env: HashMap::new(),
        timeout_secs: 5,
        tool_timeouts: HashMap::new(),
    };
    let operation = Operation::AddMcpServer { name: "bad".into(), config: config.clone() };
    let mut settings = Settings::default();
    settings.mcp_servers.insert("bad".into(), config);
    let (provider, _) = MockProvider::new(vec![]);
    let ai = AiService::with_provider("x", Box::new(provider));
    let result = test_row(&operation, &plan_with(settings), &ai, &ctx()).await.unwrap();
    assert!(!result.ok);
    assert!(result.error.unwrap().contains("failed to start MCP server 'bad'"));
}

#[tokio::test]
async fn untestable_row_returns_none() {
    let (provider, _) = MockProvider::new(vec![]);
    let ai = AiService::with_provider("x", Box::new(provider));
    let operation = Operation::DeleteModel { id: "x".into() };
    assert!(test_row(&operation, &plan_with(Settings::default()), &ai, &ctx()).await.is_none());
}

#[test]
fn stt_sample_is_mono_16k_16bit_2_to_4_seconds() {
    let reader = hound::WavReader::new(std::io::Cursor::new(include_bytes!(
        "../../../resources/stt-sample.wav"
    )))
    .unwrap();
    let spec = reader.spec();
    assert_eq!((spec.channels, spec.sample_rate, spec.bits_per_sample), (1, 16000, 16));
    let seconds = reader.duration() as f64 / spec.sample_rate as f64;
    assert!((2.0..=4.0).contains(&seconds), "{seconds}");
}
