use std::time::{Duration, Instant};

use crate::models::message::ProcessedMessage;
use crate::models::settings::{McpServerConfig, ModelConfig, Settings};
use crate::services::ai::AiService;
use crate::services::change_log::TestResult;
use crate::services::mcp::client::McpClient;
use crate::services::placeholder_registry::{substitute, PlaceholderContext};
use crate::services::skill_message::wrap_skill;
use crate::services::speech::{transcribe, SttOptions};

use super::operations::Operation;
use super::plan::Plan;

const TEXT_PROBE: &str = "Reply with the single word OK.";
const STT_SAMPLE: &[u8] = include_bytes!("../../../resources/stt-sample.wav");
const MCP_CLOSE_TIMEOUT: Duration = Duration::from_secs(4);

pub async fn test_row(
    operation: &Operation,
    plan: &Plan,
    ai: &AiService,
    placeholders: &PlaceholderContext,
) -> Option<TestResult> {
    let start = Instant::now();
    let outcome = match operation {
        Operation::AddModel { model } | Operation::UpdateModel { model, .. } => {
            test_model(model, ai).await
        }
        Operation::CreateSkill { frontmatter, body, sample_input, .. }
        | Operation::UpdateSkill { frontmatter, body, sample_input, .. } => {
            let model_id = skill_model_id(frontmatter.model.as_deref(), &plan.settings);
            let message = wrap_skill(
                &frontmatter.name,
                &substitute(body, placeholders),
                sample_input,
            );
            complete_text(ai, &model_id, &message).await
        }
        Operation::AddMcpServer { name, config } | Operation::UpdateMcpServer { name, config } => {
            let config = plan.settings.mcp_servers.get(name).unwrap_or(config);
            test_mcp(name, config).await
        }
        _ => return None,
    };
    let millis = Some(start.elapsed().as_millis() as u64);
    Some(match outcome {
        Ok(output) => TestResult { ok: true, millis, error: None, output: Some(output) },
        Err(error) => TestResult { ok: false, millis, error: Some(error), output: None },
    })
}

fn skill_model_id(skill_model: Option<&str>, settings: &Settings) -> String {
    let surfaces = &settings.surfaces;
    skill_model
        .map(str::to_string)
        .or_else(|| surfaces.quick_actions.generation.model_id.clone())
        .or_else(|| surfaces.chat.generation.model_id.clone())
        .unwrap_or_default()
}

async fn complete_text(ai: &AiService, model_id: &str, message: &str) -> Result<String, String> {
    ai.complete(model_id, vec![ProcessedMessage::text("user", message)])
        .await
        .map_err(|e| e.to_string())
}

async fn test_model(model: &ModelConfig, ai: &AiService) -> Result<String, String> {
    if model.is_stt() {
        transcribe(STT_SAMPLE.to_vec(), model, &SttOptions::default())
            .await
            .map_err(|e| e.to_string())
    } else {
        complete_text(ai, &model.id, TEXT_PROBE).await
    }
}

async fn test_mcp(name: &str, config: &McpServerConfig) -> Result<String, String> {
    let client = McpClient::start(
        name,
        &config.resolved_command(),
        &config.resolved_args(),
        &config.resolved_env(),
        config.timeout_secs,
        &config.tool_timeouts,
    )
    .await
    .map_err(|e| e.to_string())?;
    let tools = client.list_tools().await.map_err(|e| e.to_string());
    client.close(MCP_CLOSE_TIMEOUT).await;
    Ok(tools?.iter().map(|t| t.name.to_string()).collect::<Vec<_>>().join(", "))
}
