# Settings agent

Proposes changes to the app settings; the user applies them. Files: `tools.rs` (tool definitions), `dispatch.rs` (`Dispatcher`, redaction of every tool result), `plan.rs`/`apply.rs`/`testing.rs` (plan, write, test rows), `engine.rs` (session and tool loop), `system_prompt.md`. Commands: `commands/settings_agent.rs`.

## Tools

`settings_read` (values, models, skills, skill, prompt, missing_env), `settings_test` (validate and test operations, show nothing), `settings_propose_changes` (one card per request), `settings_secret` (masked input for a `${VAR}`; the agent learns only `saved`, `cancelled` or `exists`). Anything else is an error result.

## Loop

`engine::run_turn(session, user_text, ctx, live)` appends the user message and loops up to `MAX_TOOL_LOOP_ITERATIONS`: `AiService::complete_stream(chat model, messages, None, None, tools::definitions())`, `run_stream_loop`, `Dispatcher::dispatch` for each pending call in order, `build_tool_loop_messages`. The final assistant text is `last_reply`. Model: `surfaces.chat.generation.model_id`; none, or one `AiService` has no provider for, ends the turn with an error event naming the problem before any request. System prompt: `system_prompt.md` plus the catalog `ENTRIES` as JSON; no Prompt Base, no MCP tools. `EngineContext` has no `AppHandle` (settings snapshot taken at turn start, dirs, `AiService`, an `Emit` sink), so tests run it with `MockProvider` and a `LiveExecution` without channel.

## Session

One `Session` (`Arc<std::sync::Mutex<Session>>`, managed in `setup/state.rs`, never locked across an await): messages, `last_request`, `last_reply`, the current `proposal`, pending secret prompts, the cancel sender. In memory only: `settings_agent_reset` and the `Destroyed` event of the `settings-dialog` window (`services/dialog.rs`) reset it, resolve pending prompts as `cancelled` and cancel a running turn (a generation counter keeps a cancelled turn from writing back). One turn at a time. Each applied card keeps request and reply in its change-log entry.

## Events and commands

- Event `settings-agent-proposal { proposal_id, rows: [{ row, label, op, effects, test, stale }] }`; `row` is the 0-based operation index.
- Event `settings-agent-secret-request { request_id, name, purpose, exists }`; the frontend saves the value with `save_secret`, then calls `settings_agent_secret_result(request_id, "saved" | "cancelled")`.
- `settings_agent_send(message, on_event)`, `_cancel`, `_reset`, `_secret_result`.
- `settings_agent_apply(proposal_id, selected)` returns `{ entry_id }`, `{ stale_rows }` or `{ error }` (no rows selected, dependency errors with row numbers, failed write). It plans with the selection, writes through `apply_plan` (097-04) with `last_request`, `last_reply` and the row tests, and clears the proposal.
- `settings_agent_reject(proposal_id)` drops the proposal and writes nothing; a secret already saved stays.
- `settings_agent_refresh(proposal_id)` re-plans the same operations on the current settings and re-emits the proposal; row tests are kept.
- `settings_agent_test_row(proposal_id, row)` re-runs one row's test and stores it in the proposal.

## Secret guarantees

A secret value never enters a provider request: the agent never receives one (redacted settings, `${VAR}` references, `settings_secret` returns an outcome only), and every tool result passes `Redactor` before it becomes a message. Proven by `engine_tests.rs` (`no_secret_reaches_any_provider_request`: a full scripted turn with an `.env` value, a literal model key and a freshly saved key, none of which occurs in any recorded request, including a provider error echoing a key), `rejecting_the_proposal_keeps_the_saved_secret`, and `dispatch_tests.rs` per tool. No log line carries a secret or user text.
