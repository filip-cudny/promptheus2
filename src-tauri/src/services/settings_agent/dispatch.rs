use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::models::settings::Settings;
use crate::services::ai::AiService;
use crate::services::change_log::{change_log_dir, load_all, TestResult};
use crate::services::config::ConfigService;
use crate::services::execution::types::{PendingToolCall, ToolExecutionResult};
use crate::services::missing_env::{collect_missing, env_is_set, env_var_overview};
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::secrets::{self, Redactor};
use crate::services::skill::SkillService;

use super::change_log_read::{self, Filters};
use super::operations::{Operation, PromptFile};
use super::plan::{plan, Current, Plan, RowError};
use super::testing::{run_row_test, RowTest, RowTestRequest};
use super::tools;

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: String,
    pub operations: Vec<Operation>,
    pub plan: Plan,
    pub tests: BTreeMap<usize, TestResult>,
    pub waits: BTreeMap<usize, Vec<String>>,
    pub new_destinations: BTreeMap<usize, String>,
}

#[derive(Default)]
struct RowOutcomes {
    tests: BTreeMap<usize, TestResult>,
    waits: BTreeMap<usize, Vec<String>>,
    new_destinations: BTreeMap<usize, String>,
}

pub trait ProposalSink: Send + Sync {
    fn proposed(&self, proposal: &Proposal);
}

pub trait SecretViewSink: Send + Sync {
    fn shown(&self, name: &str);
}

pub struct Context {
    pub settings: Box<dyn Fn() -> Settings + Send + Sync>,
    pub config_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub ai_factory: Box<dyn Fn(&Settings) -> AiService + Send + Sync>,
    pub placeholders: PlaceholderContext,
    pub proposals: Arc<dyn ProposalSink>,
    pub secret_views: Arc<dyn SecretViewSink>,
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
    ChangeLog,
}

#[derive(Deserialize)]
struct ReadArgs {
    what: What,
    #[serde(flatten)]
    filters: Filters,
}

#[derive(Deserialize)]
struct OperationsArgs {
    operations: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SecretArgs {
    name: String,
}

type Handled = Result<Value, String>;

fn parse<T: serde::de::DeserializeOwned>(arguments: &Value) -> Result<T, String> {
    serde_json::from_value(arguments.clone()).map_err(|e| format!("invalid arguments: {e}"))
}

const NOTHING_PROPOSED: &str =
    "Nothing was proposed. Fix the rows named in `errors` and send the whole corrected list of operations again.";
const NOTHING_TESTED: &str =
    "Nothing was tested. Fix the rows named in `errors` and send the whole corrected list of operations again.";

fn row_errors(errors: &[RowError], note: &str) -> Value {
    json!({
        "errors": errors
            .iter()
            .map(|e| json!({"row": e.row + 1, "key": e.key, "message": e.message}))
            .collect::<Vec<_>>(),
        "note": note
    })
}

fn parse_operation(item: &Value) -> Result<Operation, String> {
    let valid_ops = Operation::NAMES.join(", ");
    let Some(given) = item.get("op") else {
        return Err(format!("`op` is missing; valid ops: {valid_ops}"));
    };
    let name = given.as_str().map_or_else(|| given.to_string(), String::from);
    let Some((required, optional)) = tools::operation_fields(&name) else {
        return Err(format!("unknown op '{name}'; valid ops: {valid_ops}"));
    };
    serde_json::from_value(item.clone()).map_err(|e| {
        let optional = if optional.is_empty() { "none".to_string() } else { optional.join(", ") };
        format!("{name}: {e}; fields: {}, optional: {optional}", required.join(", "))
    })
}

fn parse_operations(arguments: &Value) -> Result<Result<Vec<Operation>, Vec<RowError>>, String> {
    let args: OperationsArgs = parse(arguments)?;
    let (operations, errors): (Vec<_>, Vec<_>) = args
        .operations
        .iter()
        .enumerate()
        .map(|(row, item)| parse_operation(item).map_err(|message| RowError { row, key: None, message }))
        .partition(Result::is_ok);
    Ok(if errors.is_empty() {
        Ok(operations.into_iter().flatten().collect())
    } else {
        Err(errors.into_iter().filter_map(Result::err).collect())
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
            tools::SECRET => self.secret(&call.arguments),
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
        let name = || args.filters.name.clone().ok_or_else(|| "`name` is required".to_string());
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
                let overview =
                    env_var_overview(&settings, env_is_set, |name| secrets::current_value(&self.ctx.config_dir, name));
                serde_json::to_value(overview).map_err(|e| e.to_string())
            }
            What::ChangeLog => {
                change_log_read::read(&load_all(&change_log_dir(&self.ctx.config_dir)), &args.filters)
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

    async fn run_tests(&self, operations: &[Operation], plan: &Plan) -> RowOutcomes {
        let before = (self.ctx.settings)();
        let values = BTreeMap::new();
        let mut outcomes = RowOutcomes::default();
        for row in 0..operations.len() {
            let request = RowTestRequest {
                row,
                operations,
                plan,
                before: &before,
                values: &values,
                ai_factory: &*self.ctx.ai_factory,
                placeholders: &self.ctx.placeholders,
                config_dir: &self.ctx.config_dir,
                allow_new_destinations: false,
            };
            match run_row_test(&request).await {
                RowTest::NotTestable => {}
                RowTest::Waits(names) => {
                    outcomes.waits.insert(row, names);
                }
                RowTest::NewDestination(text) => {
                    outcomes.new_destinations.insert(row, text);
                }
                RowTest::Tested(result) => {
                    outcomes.tests.insert(row, result);
                }
            }
        }
        outcomes
    }

    fn rows_json(plan: &Plan, outcomes: &RowOutcomes) -> Vec<Value> {
        plan.rows
            .iter()
            .map(|r| {
                json!({
                    "row": r.row + 1,
                    "label": r.operation.label,
                    "test": outcomes.tests.get(&r.row),
                    "waits_for": outcomes.waits.get(&r.row).cloned().unwrap_or_default(),
                    "new_destination": outcomes.new_destinations.get(&r.row),
                })
            })
            .collect()
    }

    async fn test(&self, arguments: &Value) -> Handled {
        let operations = match parse_operations(arguments)? {
            Ok(operations) => operations,
            Err(errors) => return Ok(row_errors(&errors, NOTHING_TESTED)),
        };
        match self.plan_all(&operations) {
            Err(errors) => Ok(row_errors(&errors, NOTHING_TESTED)),
            Ok(planned) => {
                let outcomes = self.run_tests(&operations, &planned).await;
                Ok(json!({"rows": Self::rows_json(&planned, &outcomes)}))
            }
        }
    }

    async fn propose(&self, arguments: &Value) -> Handled {
        let operations = match parse_operations(arguments)? {
            Ok(operations) => operations,
            Err(errors) => return Ok(row_errors(&errors, NOTHING_PROPOSED)),
        };
        let planned = match self.plan_all(&operations) {
            Err(errors) => return Ok(row_errors(&errors, NOTHING_PROPOSED)),
            Ok(planned) => planned,
        };
        let outcomes = self.run_tests(&operations, &planned).await;
        let rows = Self::rows_json(&planned, &outcomes);
        let proposal = Proposal {
            id: uuid::Uuid::new_v4().simple().to_string(),
            operations,
            plan: planned,
            tests: outcomes.tests,
            waits: outcomes.waits,
            new_destinations: outcomes.new_destinations,
        };
        log::info!("settings agent proposal {} with {} rows", proposal.id, rows.len());
        let id = proposal.id.clone();
        self.ctx.proposals.proposed(&proposal);
        Ok(json!({"proposal_id": id, "rows": rows}))
    }

    fn secret(&self, arguments: &Value) -> Handled {
        let args: SecretArgs = parse(arguments)?;
        if !secrets::validate_name(&args.name) {
            return Err(format!("invalid variable name '{}'", args.name));
        }
        let is_set = secrets::current_value(&self.ctx.config_dir, &args.name).is_some_and(|value| !value.is_empty());
        let outcome = if is_set {
            self.ctx.secret_views.shown(&args.name);
            "shown"
        } else {
            "not_set"
        };
        log::info!("settings agent secret '{}': {outcome}", args.name);
        Ok(json!({"name": args.name, "outcome": outcome}))
    }
}
