use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::models::settings::Settings;
use crate::services::ai::AiService;
use crate::services::change_log::TestResult;
use crate::services::config::ConfigService;
use crate::services::execution::types::{PendingToolCall, ToolExecutionResult};
use crate::services::missing_env::{collect_missing, env_is_set, env_var_overview};
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::secrets::{self, Redactor};
use crate::services::skill::SkillService;

use super::operations::{Operation, PromptFile};
use super::plan::{plan, Current, Plan, RowError};
use super::testing::test_row;
use super::tools;

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: String,
    pub operations: Vec<Operation>,
    pub plan: Plan,
    pub tests: BTreeMap<usize, TestResult>,
}

pub trait ProposalSink: Send + Sync {
    fn proposed(&self, proposal: &Proposal);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretAnswer {
    Saved,
    Cancelled,
}

#[async_trait]
pub trait SecretPrompt: Send + Sync {
    async fn ask(&self, name: &str, purpose: &str) -> SecretAnswer;
}

pub struct Context {
    pub settings: Box<dyn Fn() -> Settings + Send + Sync>,
    pub config_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub ai_factory: Box<dyn Fn(&Settings) -> AiService + Send + Sync>,
    pub placeholders: PlaceholderContext,
    pub proposals: Arc<dyn ProposalSink>,
    pub secrets: Arc<dyn SecretPrompt>,
}

pub struct Dispatcher {
    ctx: Context,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum What {
    Values,
    Models,
    Skills,
    Skill,
    Prompt,
    MissingEnv,
    EnvVars,
}

#[derive(Deserialize)]
struct ReadArgs {
    what: What,
    name: Option<String>,
}

#[derive(Deserialize)]
struct TestArgs {
    operations: Vec<Operation>,
}

#[derive(Deserialize)]
struct ProposeArgs {
    operations: Vec<Operation>,
}

#[derive(Deserialize)]
struct SecretArgs {
    name: String,
    purpose: String,
}

type Handled = Result<Value, String>;

fn parse<T: serde::de::DeserializeOwned>(arguments: &Value) -> Result<T, String> {
    serde_json::from_value(arguments.clone()).map_err(|e| format!("invalid arguments: {e}"))
}

fn row_errors(errors: &[RowError]) -> Value {
    json!({
        "errors": errors
            .iter()
            .map(|e| json!({"row": e.row + 1, "key": e.key, "message": e.message}))
            .collect::<Vec<_>>()
    })
}

impl Dispatcher {
    pub fn new(ctx: Context) -> Self {
        Self { ctx }
    }

    pub async fn dispatch(&self, call: &PendingToolCall) -> ToolExecutionResult {
        log::debug!("settings agent tool call: {}", call.tool_name);
        let handled = match call.tool_name.as_str() {
            tools::READ => self.read(&call.arguments),
            tools::TEST => self.test(&call.arguments).await,
            tools::PROPOSE_CHANGES => self.propose(&call.arguments).await,
            tools::SECRET => self.secret(&call.arguments).await,
            other => Err(format!("unknown tool '{other}'; a request outside the settings tools cannot be done")),
        };
        let (text, is_error) = match handled {
            Ok(value) => (value.to_string(), false),
            Err(message) => (message, true),
        };
        let settings = (self.ctx.settings)();
        ToolExecutionResult {
            tool_call_id: call.tool_call_id.clone(),
            result_text: Redactor::new(&self.ctx.config_dir, &settings).redact(&text),
            is_error,
        }
    }

    fn read(&self, arguments: &Value) -> Handled {
        let args: ReadArgs = parse(arguments)?;
        let settings = (self.ctx.settings)();
        let redactor = Redactor::new(&self.ctx.config_dir, &settings);
        let name = || args.name.clone().ok_or_else(|| "`name` is required".to_string());
        match args.what {
            What::Values => Ok(redactor.redact_settings(&settings)),
            What::Models => Ok(redactor.redact_settings(&settings)["models"].clone()),
            What::Skills => {
                let skills = self.skills(&settings)?;
                Ok(skills
                    .list_skills()
                    .iter()
                    .map(|s| {
                        json!({
                            "name": s.name,
                            "description": s.description,
                            "model": s.model,
                            "offered_in": s.offered_in,
                        })
                    })
                    .collect())
            }
            What::Skill => {
                let name = name()?;
                let skills = self.skills(&settings)?;
                let skill = skills.get_skill(&name).ok_or_else(|| format!("skill '{name}' not found"))?;
                serde_json::to_value(skill).map_err(|e| e.to_string())
            }
            What::Prompt => self.prompt(&name()?),
            What::MissingEnv => {
                serde_json::to_value(collect_missing(&settings, env_is_set)).map_err(|e| e.to_string())
            }
            What::EnvVars => {
                serde_json::to_value(env_var_overview(&settings, env_is_set)).map_err(|e| e.to_string())
            }
        }
    }

    fn skills(&self, settings: &Settings) -> Result<SkillService, String> {
        SkillService::load(&self.ctx.skills_dir, &settings.skills_order).map_err(|e| e.to_string())
    }

    fn prompt(&self, name: &str) -> Handled {
        let kind: PromptFile = serde_json::from_value(json!(name))
            .map_err(|_| format!("unknown prompt '{name}'"))?;
        let config = ConfigService::detached((self.ctx.settings)(), &self.ctx.config_dir);
        let content = match kind.prompt_kind() {
            Some(kind) => config.read_prompt(kind),
            None => config.read_stt_keyterms_file().map_err(|e| e.to_string())?.content,
        };
        Ok(json!({"name": name, "content": content}))
    }

    fn plan_all(&self, operations: &[Operation]) -> Result<Plan, Vec<RowError>> {
        let settings = (self.ctx.settings)();
        let current = Current { settings: &settings, config_dir: &self.ctx.config_dir };
        plan(operations, &vec![true; operations.len()], &current)
    }

    async fn run_tests(&self, operations: &[Operation], plan: &Plan) -> BTreeMap<usize, TestResult> {
        let ai = (self.ctx.ai_factory)(&plan.settings);
        let mut tests = BTreeMap::new();
        for (row, operation) in operations.iter().enumerate() {
            if let Some(result) = test_row(operation, plan, &ai, &self.ctx.placeholders).await {
                tests.insert(row, result);
            }
        }
        tests
    }

    fn rows_json(operations: &[Operation], tests: &BTreeMap<usize, TestResult>) -> Vec<Value> {
        operations
            .iter()
            .enumerate()
            .map(|(row, op)| json!({"row": row + 1, "label": op.label(), "test": tests.get(&row)}))
            .collect()
    }

    async fn test(&self, arguments: &Value) -> Handled {
        let args: TestArgs = parse(arguments)?;
        match self.plan_all(&args.operations) {
            Err(errors) => Ok(row_errors(&errors)),
            Ok(planned) => {
                let tests = self.run_tests(&args.operations, &planned).await;
                Ok(json!({"rows": Self::rows_json(&args.operations, &tests)}))
            }
        }
    }

    async fn propose(&self, arguments: &Value) -> Handled {
        let args: ProposeArgs = parse(arguments)?;
        let planned = match self.plan_all(&args.operations) {
            Err(errors) => return Ok(row_errors(&errors)),
            Ok(planned) => planned,
        };
        let tests = self.run_tests(&args.operations, &planned).await;
        let rows = Self::rows_json(&args.operations, &tests);
        let proposal = Proposal {
            id: uuid::Uuid::new_v4().simple().to_string(),
            operations: args.operations,
            plan: planned,
            tests,
        };
        log::info!("settings agent proposal {} with {} rows", proposal.id, rows.len());
        let id = proposal.id.clone();
        self.ctx.proposals.proposed(&proposal);
        Ok(json!({"proposal_id": id, "rows": rows}))
    }

    async fn secret(&self, arguments: &Value) -> Handled {
        let args: SecretArgs = parse(arguments)?;
        if !secrets::validate_name(&args.name) {
            return Err(format!("invalid variable name '{}'", args.name));
        }
        let outcome = match self.ctx.secrets.ask(&args.name, &args.purpose).await {
            SecretAnswer::Saved => "saved",
            SecretAnswer::Cancelled
                if secrets::current_value(&self.ctx.config_dir, &args.name).is_some() =>
            {
                "exists"
            }
            SecretAnswer::Cancelled => "cancelled",
        };
        log::info!("settings agent secret '{}': {outcome}", args.name);
        Ok(json!({"name": args.name, "outcome": outcome}))
    }
}
