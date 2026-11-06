use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::{oneshot, watch, Mutex as TokioMutex};

use crate::models::message::ProcessedMessage;
use crate::models::settings::Settings;
use crate::services::ai::AiService;
use crate::services::config::catalog;
use crate::services::execution::lifecycle::LiveExecution;
use crate::services::execution::stream::{build_tool_loop_messages, run_stream_loop};
use crate::services::execution::types::{PendingToolCall, MAX_TOOL_LOOP_ITERATIONS};
use crate::services::placeholder_registry::PlaceholderContext;
use crate::services::secrets;
use crate::Error;

use super::dispatch::{Context, Dispatcher, Proposal, ProposalSink, SecretAnswer, SecretPrompt};
use super::plan::{plan, stale_rows, Current, RowError};
use super::tools;

pub const PROPOSAL_EVENT: &str = "settings-agent-proposal";
pub const SECRET_REQUEST_EVENT: &str = "settings-agent-secret-request";

pub trait Emit: Send + Sync {
    fn emit(&self, event: &str, payload: Value);
}

#[derive(Default)]
pub struct Session {
    messages: Vec<ProcessedMessage>,
    pub last_request: Option<String>,
    pub last_reply: Option<String>,
    pub proposal: Option<Proposal>,
    pending_secrets: HashMap<String, oneshot::Sender<SecretAnswer>>,
    cancel: Option<watch::Sender<bool>>,
    generation: u64,
}

pub type SharedSession = Arc<Mutex<Session>>;

pub fn lock(session: &SharedSession) -> MutexGuard<'_, Session> {
    session.lock().unwrap_or_else(|e| e.into_inner())
}

impl Session {
    fn answer_pending(&mut self, answer: SecretAnswer) {
        for (_, sender) in self.pending_secrets.drain() {
            let _ = sender.send(answer);
        }
    }

    pub fn cancel(&mut self) {
        self.answer_pending(SecretAnswer::Cancelled);
        if let Some(cancel) = &self.cancel {
            let _ = cancel.send(true);
        }
    }

    pub fn reset(&mut self) {
        self.cancel();
        self.messages.clear();
        self.last_request = None;
        self.last_reply = None;
        self.proposal = None;
        self.generation += 1;
    }

    pub fn resolve_secret(&mut self, request_id: &str, answer: SecretAnswer) -> bool {
        self.pending_secrets
            .remove(request_id)
            .is_some_and(|sender| sender.send(answer).is_ok())
    }
}

pub struct EngineContext {
    pub settings: Arc<dyn Fn() -> Settings + Send + Sync>,
    pub config_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub ai: AiService,
    pub ai_factory: Arc<dyn Fn(&Settings) -> AiService + Send + Sync>,
    pub placeholders: PlaceholderContext,
    pub emit: Arc<dyn Emit>,
}

struct SessionSink {
    session: SharedSession,
    settings: Arc<dyn Fn() -> Settings + Send + Sync>,
    config_dir: PathBuf,
    emit: Arc<dyn Emit>,
}

impl ProposalSink for SessionSink {
    fn proposed(&self, proposal: &Proposal) {
        lock(&self.session).proposal = Some(proposal.clone());
        let settings = (self.settings)();
        let current = Current { settings: &settings, config_dir: &self.config_dir };
        self.emit.emit(PROPOSAL_EVENT, proposal_payload(proposal, &current));
    }
}

#[async_trait]
impl SecretPrompt for SessionSink {
    async fn ask(&self, name: &str, purpose: &str) -> SecretAnswer {
        let request_id = uuid::Uuid::new_v4().simple().to_string();
        let (sender, receiver) = oneshot::channel();
        lock(&self.session).pending_secrets.insert(request_id.clone(), sender);
        let exists = secrets::current_value(&self.config_dir, name).is_some();
        self.emit.emit(
            SECRET_REQUEST_EVENT,
            json!({"request_id": request_id, "name": name, "purpose": purpose, "exists": exists}),
        );
        receiver.await.unwrap_or(SecretAnswer::Cancelled)
    }
}

pub fn proposal_payload(proposal: &Proposal, current: &Current) -> Value {
    let stale = stale_rows(&proposal.plan, current);
    let rows: Vec<Value> = proposal
        .plan
        .rows
        .iter()
        .map(|planned| {
            json!({
                "row": planned.row,
                "label": planned.operation.label,
                "op": planned.operation.op,
                "effects": planned.operation.effects,
                "test": proposal.tests.get(&planned.row),
                "stale": stale.contains(&planned.row),
            })
        })
        .collect();
    json!({"proposal_id": proposal.id, "rows": rows})
}

pub fn refresh(proposal: &Proposal, current: &Current) -> Result<Proposal, Vec<RowError>> {
    let operations = &proposal.operations;
    let planned = plan(operations, &vec![true; operations.len()], current)?;
    Ok(Proposal { plan: planned, ..proposal.clone() })
}

pub fn describe(errors: &[RowError]) -> String {
    errors
        .iter()
        .map(|e| format!("row {}: {}", e.row + 1, e.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn system_prompt() -> String {
    let entries = serde_json::to_string(catalog::ENTRIES).unwrap_or_default();
    format!("{}{entries}\n", include_str!("system_prompt.md"))
}

fn begin(
    session: &SharedSession,
    user_text: &str,
) -> crate::Result<(Vec<ProcessedMessage>, u64, watch::Receiver<bool>)> {
    let mut s = lock(session);
    if s.cancel.is_some() {
        return Err(Error::Other("the settings agent is already working on a request".into()));
    }
    let (cancel, cancel_rx) = watch::channel(false);
    s.cancel = Some(cancel);
    if s.messages.is_empty() {
        s.messages.push(ProcessedMessage::text("system", system_prompt()));
    }
    s.last_request = Some(user_text.to_string());
    s.last_reply = None;
    let mut messages = s.messages.clone();
    messages.push(ProcessedMessage::text("user", user_text));
    Ok((messages, s.generation, cancel_rx))
}

async fn fail(live: &Arc<TokioMutex<LiveExecution>>, message: String) -> Error {
    live.lock().await.finish_with_error(message.clone());
    Error::Other(message)
}

async fn chat_model(
    settings: &Settings,
    ai: &AiService,
    live: &Arc<TokioMutex<LiveExecution>>,
) -> crate::Result<String> {
    let Some(model_id) = settings.surfaces.chat.generation.model_id.clone() else {
        return Err(fail(live, "no chat model configured".into()).await);
    };
    if !ai.has_model(&model_id) {
        return Err(fail(live, format!("chat model '{model_id}' is not available")).await);
    }
    Ok(model_id)
}

async fn tool_loop(
    messages: &mut Vec<ProcessedMessage>,
    session: &SharedSession,
    ctx: &EngineContext,
    live: &Arc<TokioMutex<LiveExecution>>,
    cancel_rx: watch::Receiver<bool>,
) -> crate::Result<String> {
    let sink = Arc::new(SessionSink {
        session: Arc::clone(session),
        settings: Arc::clone(&ctx.settings),
        config_dir: ctx.config_dir.clone(),
        emit: Arc::clone(&ctx.emit),
    });
    let settings_source = Arc::clone(&ctx.settings);
    let factory = Arc::clone(&ctx.ai_factory);
    let dispatcher = Dispatcher::new(Context {
        settings: Box::new(move || settings_source()),
        config_dir: ctx.config_dir.clone(),
        skills_dir: ctx.skills_dir.clone(),
        ai_factory: Box::new(move |settings| factory(settings)),
        placeholders: ctx.placeholders.clone(),
        proposals: sink.clone(),
        secrets: sink,
    });
    let model_id = chat_model(&(ctx.settings)(), &ctx.ai, live).await?;

    let mut prefix = String::new();
    for iteration in 0..=MAX_TOOL_LOOP_ITERATIONS {
        if *cancel_rx.borrow() {
            return Err(fail(live, "Cancelled".into()).await);
        }
        let stream = match ctx
            .ai
            .complete_stream(&model_id, messages.clone(), None, None, tools::definitions())
            .await
        {
            Ok(stream) => stream,
            Err(e) => return Err(fail(live, e.to_string()).await),
        };
        let result = run_stream_loop(stream, Arc::clone(live), Some(cancel_rx.clone()), &prefix).await?;
        if result.pending_tool_calls.is_empty() {
            if !result.full_text.is_empty() {
                messages.push(ProcessedMessage::text("assistant", result.full_text.clone()));
            }
            return Ok(result.full_text);
        }
        if iteration == MAX_TOOL_LOOP_ITERATIONS {
            log::warn!("settings agent tool loop reached {MAX_TOOL_LOOP_ITERATIONS} iterations");
            live.lock().await.finish_done(
                result.full_text.clone(),
                result.full_thinking,
                result.prompt_tokens,
                result.completion_tokens,
            );
            return Ok(result.full_text);
        }
        prefix.clone_from(&result.full_text);
        live.lock().await.snapshot.finished = false;
        let results = dispatch_all(&dispatcher, &result.pending_tool_calls, live).await;
        messages.extend(build_tool_loop_messages(&result.pending_tool_calls, &result.full_text, &results));
    }
    unreachable!("the loop returns at its last iteration")
}

async fn dispatch_all(
    dispatcher: &Dispatcher,
    pending: &[PendingToolCall],
    live: &Arc<TokioMutex<LiveExecution>>,
) -> Vec<crate::services::execution::types::ToolExecutionResult> {
    let mut results = Vec::with_capacity(pending.len());
    for call in pending {
        let result = dispatcher.dispatch(call).await;
        let (output, error) = if result.is_error {
            (None, Some(result.result_text.clone()))
        } else {
            (Some(result.result_text.clone()), None)
        };
        live.lock().await.finish_tool_call(result.tool_call_id.clone(), output, error);
        results.push(result);
    }
    results
}

pub async fn run_turn(
    session: &SharedSession,
    user_text: &str,
    ctx: EngineContext,
    live: Arc<TokioMutex<LiveExecution>>,
) -> crate::Result<()> {
    let (mut messages, generation, cancel_rx) = begin(session, user_text)?;
    log::info!("settings agent turn started");
    let outcome = tool_loop(&mut messages, session, &ctx, &live, cancel_rx).await;
    let mut s = lock(session);
    s.cancel = None;
    if s.generation == generation {
        s.messages = messages;
        if let Ok(reply) = &outcome {
            s.last_reply = Some(reply.clone());
        }
    }
    log::info!("settings agent turn finished, ok={}", outcome.is_ok());
    outcome.map(|_| ())
}
