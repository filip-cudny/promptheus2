# Change log

One immutable JSON file per settings-agent operation (card, secret change or undo), written next to the real `settings.json` so settings sync commits and shares it between hosts.

## Directory

`change_log_dir(config_dir)`: `<parent of the canonical target of <config>/settings.json>/change-log` when `settings.json` is a symlink, else `<config>/change-log`. Created on the first write only. `secret_backup_dir(config_dir)` is `<config>/secret-backups` (never synced; backup file `<entry id>.env`, written by the secret flow and by every card or undo applied through `apply.rs`).

## File

`<UTC time>-<host>-<short id>.json`, e.g. `20261106T101500Z-laptop-3fa9c1d2.json`: time `YYYYMMDDTHHMMSSZ`, host with characters outside `[A-Za-z0-9-]` replaced by `-` (the host sync commits use, `SettingsSync::host`), short id = first 8 hex characters of a v4 uuid. `new_id(now, host)` builds it; the entry `id` is the file stem. Files are written with `create_new` and pretty JSON and never rewritten: a second write of an id fails (`write_entry`).

## Entry format

`ChangeLogEntry` (snake_case): `id`, `created_at` (RFC 3339 UTC), `host`, `kind` (`card` | `secret` | `undo`), `request`, `reply`, `operations`, `undoes` (id of the undone entry, `undo` only), `secret` (`secret` only).

- `LoggedOperation`: `label`, `op` (operation type), `effects`, `test` (`TestResult { ok, millis, error, output }` or null).
- `Effect`: `target`, `before`, `after`; null = absent (no setting value, no file).
- `Target`: `{"type": "setting", "path": "models[gpt].display_name"}` (concrete key path as `catalog::catalog_path` accepts) or `{"type": "file", "path": "skills/<slug>/SKILL.md"}` (relative to the config dir; file content is a JSON string).
- `Target::Secret`: `{"type": "secret", "name": "OPENAI_API_KEY"}`; the effect's `before` and `after` are always null (the value is in `.env` and in the card backup, never in the entry). `settings_agent::plan::mismatches` and `apply::write_row` skip it; `change_log_read::effect_json` shows `{"target": "secret", "name"}` only.
- `SecretChange`: `name`, `backup` (file name in `secret-backups`), `had_previous`.

Card:

```json
{"id": "20261106T101500Z-laptop-3fa9c1d2", "created_at": "2026-11-06T10:15:00Z", "host": "laptop", "kind": "card",
 "request": "rename the model", "reply": "Renamed.", "undoes": null, "secret": null,
 "operations": [{"label": "Rename model", "op": "set_setting", "test": null,
   "effects": [{"target": {"type": "setting", "path": "models[gpt].display_name"}, "before": "GPT", "after": "GPT 5"}]}]}
```

Secret:

```json
{"id": "20261106T101600Z-laptop-a1b2c3d4", "created_at": "2026-11-06T10:16:00Z", "host": "laptop", "kind": "secret",
 "request": null, "reply": null, "operations": [], "undoes": null,
 "secret": {"name": "OPENAI_API_KEY", "backup": "20261106T101600Z-laptop-a1b2c3d4.env", "had_previous": true}}
```

A secret entry holds the name and a backup reference only, never a value. `services/secrets.rs` writes it: `save_secret` writes `kind: secret` with `backup` = `<entry id>.env` when a previous value existed. `undo_secret` writes `kind: undo` with `undoes` = the secret entry id and `secret` set; its own backup holds the value it replaced or removed, so the undo can be undone. A host without the backup file skips the undo (`UndoOutcome::Skipped { held_by: <entry host> }`) and writes nothing. See "Secrets" in `services/DOCS.md`.

Undo:

```json
{"id": "20261106T102000Z-laptop-0f9e8d7c", "created_at": "2026-11-06T10:20:00Z", "host": "laptop", "kind": "undo",
 "request": null, "reply": null, "undoes": "20261106T101500Z-laptop-3fa9c1d2", "secret": null,
 "operations": [{"label": "Rename model", "op": "set_setting", "test": null,
   "effects": [{"target": {"type": "setting", "path": "models[gpt].display_name"}, "before": "GPT 5", "after": "GPT"}]}]}
```

## Loading and state

`load_all(dir)` reads every `*.json`, skips unparsable files with a `warn` naming the file, sorts by `created_at` then `id`. `entry_state(entries, id)` is `Undone` when a non-undone `undo` entry has `undoes == id`, else `Applied`; an undo of an undo makes the first entry applied again.

## Card backup

`secret-backups/<entry id>.env` (`secrets::write_card_backup` / `read_card_backup`; dir mode 0700, file 0600, values through `quote`): dotenv lines `before__<NAME>="<previous value>"` per written variable that had a value (`secrets::current_value`) and `after__<NAME>="<value after the write>"` per variable that has one after it. A name is read back by stripping the fixed prefix, so names containing `__` stay unambiguous; a missing `before__` line means the variable had no value, a missing `after__` line that it has none after. Every `Card` apply and every `Undo` written by `apply.rs` writes the file, empty when the entry has no secret effect. It is never synced, so another host finds it missing.

## Pruning

`prune(dir, backup_dir, retention_days, now)` deletes entry files older than the retention, the backup file each names in `secret.backup` and `<backup_dir>/<entry id>.env` of every pruned entry (a missing file is ignored); `retention_days == 0` deletes nothing. Called at startup (`setup/state.rs`) and in `update_history_retention`, after the history sweep; failures are logged with `warn`, never fatal.

## Sync

`settings_sync::detect` adds `<settings target parent>/change-log` to the stage paths only when the directory exists. A host whose directory does not exist yet gets the first pulled entries through the rebase but no `change-log-changed` event until the next cycle. After a pull that changed a path under the directory, `reload_changed` sets `Reloaded.change_log` and the app emits `change-log-changed` (no payload).

## Apply and undo

`services/settings_agent/apply.rs`; commands in `commands/change_log.rs`.

`apply(plan, current, ctx)`: refuses with `ApplyError::Stale(rows)` when `stale_rows` is non-empty (nothing written). It then collects the variable names of the plan's secret effects (row order, each once); a name without a non-empty entry in `ctx.values` fails with `ApplyError::NoValue` (`no value entered for <NAME>`) before anything is written, and keys of `values` that are no such name are ignored. The entry id is created before any write. Under `secrets::lock_env_writes()`, held until the settings rows are written or restored, it snapshots the `.env` bytes and `override_for` of each name, writes the card backup, writes each value to `.env` and the override map (`secrets::set_env_value`), then writes the rows in order: per row, its setting effects as one `ConfigService::try_mutate` (validation kept), then its file effects (content written in place with `std::fs::write`, `after = null` deletes the file and removes empty skill directories). Before the first write to a target its content is snapshotted (`settings.json` bytes once, each file's bytes or absence). On the first failing write every snapshot is restored in reverse order (`settings.json` is rewritten and the config reloaded), then the `.env` bytes (absent file: removed) and each override (`set_override` with the old value, `clear_override` when there was none), the card backup is deleted and `ApplyError::WriteFailed { row, message }` returns; no log file exists then. After the last write one `kind: card` entry is written with the rows' `LoggedOperation`s and their `TestResult`s (`ctx.tests`, keyed by row). `ctx.request` and `ctx.reply` go into the entry.

`undo(entry_id, force, ctx)`: a `secret` entry or an entry with `secret` set goes to `secrets::undo_secret` (`Undone` or `Skipped { held_by }`). Otherwise the entry's rows are inverted (rows and effects in reverse order, `before` and `after` swapped). When the entry has secret effects and `secret-backups/<entry id>.env` is missing (e.g. written on another host), the outcome is `Skipped { held_by: <entry host> }` and nothing is written, settings rows included. Each inverted effect's expected value (the original `after`) is compared with the current value in order: a difference is a `Conflict { row, target, expected_after, current }`; each secret variable's `current_value` must equal its `after__` value (absent = no value), a difference is a `Conflict` with `Target::Secret { name }` and both values `None` (never a value), collected with the others. With conflicts and `force == false` the outcome is `Conflicts { conflicts }` and nothing is written. Otherwise the inverted rows are written with the same snapshot and restore rule (the Undo entry id is created first; under the env lock the Undo writes its own card backup, `before__` = current value and `after__` = restored value, restores each variable from the `before__` value or removes it from `.env` and the override map, then writes the settings rows; a failure restores `.env` and overrides and deletes that backup), then one `kind: undo` entry with `undoes = entry_id` and the inverted effects is written. Undoing an `undo` entry therefore re-applies the original effects and writes a third entry naming the second. For setting targets a `null` and an absent value compare equal (a logged `null` reads back as absent). A removed list item is logged as `{"index", "value"}` in `before` and reinserted at that index.

Outcomes (`status`, snake_case): `undone` (`entry_id` of the new undo entry), `conflicts`, `skipped` (`held_by`).

Commands: `list_change_log` returns `[{entry, state}]` of all hosts, newest first, `state` is `applied` or `undone`; `undo_change(id, force)`. After a successful write `apply_settings_reload(app, "agent")` runs and `change-log-changed` is emitted. `commands::change_log::apply_plan(app, plan, request, reply, tests, values)` runs apply with the same follow-up steps for the agent loop; `apply_env_var_operation` and `undo_change` pass an empty `values`.
