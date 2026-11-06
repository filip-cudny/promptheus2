# Settings agent

Proposes changes to the app settings; the user applies them. Files: `tools.rs` (tool definitions), `dispatch.rs` (`Dispatcher`, redaction of every tool result), `plan.rs`/`apply.rs`/`testing.rs` (plan, write, test rows), `engine.rs` (session and tool loop), `system_prompt.md`. Commands: `commands/settings_agent.rs`.

## Tools

`settings_read` (values, models, skills, skill, prompt, missing_env, env_vars), `settings_test` (validate and test operations, show nothing), `settings_propose_changes` (one card per request), `settings_secret` (masked input for a `${VAR}`; the agent learns only `saved`, `cancelled` or `exists`). Anything else is an error result.

## Environment variable operations

`add_env_var`, `update_env_var`, `rename_env_var`, `delete_env_var` (`operations.rs`, arms in `plan::apply`) change `Settings::env_vars` and the `${VAR}` references as setting effects only; `.env` is never written in a plan (rename reads it). Errors are keyed `env_vars[<name>]` (`.name` for name problems).

- `rename_env_var` refuses when `new_name` already has a non-empty value on this machine that differs from the old name's (`secrets::current_value`); otherwise it renames the entry, sets `renamed_from` to the old name followed by the earlier ones (newest first, no duplicates, so `copy_renamed_values` takes the latest) and replaces every `${old}` token (exact token, also inside longer strings) in every field of `missing_env::visit_refs_mut`. MCP `env` keys are never touched, only values.
- `delete_env_var` fails with `used in: <labels>` while `collect_missing(settings, |_| false)` lists the name; a row earlier in the same plan that reassigns the reference lets it pass. It does not touch `.env`.
- `set_setting` refuses `env_vars` (`SETTINGS_MANAGED_BY_OTHER_OPERATIONS`).
- Walker: `missing_env::visit_refs` (read, with key and label) and `visit_refs_mut` (`&mut String`) visit the same fields in the same order; `collect_missing` and rename use them, so a new field that may hold `${VAR}` is added there once.
- List key: list items are keyed in effect paths (`<list>[<key>]`) by `id`, `context` or `name` (`plan.rs::item_key`, `catalog.rs::collect_leaves`), so an effect on one `env_vars` entry is independent of the others and its Undo does not conflict with a later edit of another. A rename is a removal of `env_vars[old]` plus an addition of `env_vars[new]`; an apply appends the new entry at the end of the list, an Undo restores the old one at its position.
- Read: `settings_read` `what: "env_vars"` and the command `get_env_vars` (`commands/env_vars.rs`, fresh settings) return `missing_env::env_var_overview`: one `EnvVarStatus` per `Settings::env_vars` entry in list order with `name`, `description`, `secret`, `set` (value present on this machine, `env_is_set`) and `used_in` (key and label of each field that references it, from `collect_missing(settings, |_| false)`). It has no value field.
- GUI: `apply_env_var_operation(operation)` (`commands/env_vars.rs`) accepts only these four, plans one operation with `plan::plan_single` (same `plan` as the agent) and writes a `Card` entry through `apply_plan` without request, reply or tests; it returns the entry id.

## Loop

`engine::run_turn(session, user_text, ctx, live)` appends the user message and loops up to `MAX_TOOL_LOOP_ITERATIONS`: `AiService::complete_stream(agent model, messages, Some(agent parameters), None, tools::definitions())`, `run_stream_loop`, `Dispatcher::dispatch` for each pending call in order, `build_tool_loop_messages`. The final assistant text is `last_reply`. Model: `surfaces.settings_agent.generation.model_id`, falling back to `surfaces.chat.generation.model_id` when it is `None` or empty; `surfaces.settings_agent.generation.parameters` go to `complete_stream` as `parameter_overrides` also after the fallback, the chat's parameters never apply. Neither model set ("no settings agent or chat model configured"), or an id `AiService` has no provider for ("model '<id>' is not available"), ends the turn with an error event before any request. The chosen id is logged at `debug` when the turn starts. System prompt: `system_prompt.md` plus the catalog `ENTRIES` as JSON; no Prompt Base, no MCP tools. `EngineContext` has no `AppHandle` (settings snapshot taken at turn start, dirs, `AiService`, an `Emit` sink), so tests run it with `MockProvider` and a `LiveExecution` without channel.

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
