# Change log

One immutable JSON file per settings-agent operation (card, secret change or undo), written next to the real `settings.json` so settings sync commits and shares it between hosts.

## Directory

`change_log_dir(config_dir)`: `<parent of the canonical target of <config>/settings.json>/change-log` when `settings.json` is a symlink, else `<config>/change-log`. Created on the first write only. `secret_backup_dir(config_dir)` is `<config>/secret-backups` (never synced; written by the secret flow).

## File

`<UTC time>-<host>-<short id>.json`, e.g. `20261106T101500Z-laptop-3fa9c1d2.json`: time `YYYYMMDDTHHMMSSZ`, host with characters outside `[A-Za-z0-9-]` replaced by `-` (the host sync commits use, `SettingsSync::host`), short id = first 8 hex characters of a v4 uuid. `new_id(now, host)` builds it; the entry `id` is the file stem. Files are written with `create_new` and pretty JSON and never rewritten: a second write of an id fails (`write_entry`).

## Entry format

`ChangeLogEntry` (snake_case): `id`, `created_at` (RFC 3339 UTC), `host`, `kind` (`card` | `secret` | `undo`), `request`, `reply`, `operations`, `undoes` (id of the undone entry, `undo` only), `secret` (`secret` only).

- `LoggedOperation`: `label`, `op` (operation type), `effects`, `test` (`TestResult { ok, millis, error, output }` or null).
- `Effect`: `target`, `before`, `after`; null = absent (no setting value, no file).
- `Target`: `{"type": "setting", "path": "models[gpt].display_name"}` (concrete key path as `catalog::catalog_path` accepts) or `{"type": "file", "path": "skills/<slug>/SKILL.md"}` (relative to the config dir; file content is a JSON string).
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
 "secret": {"name": "OPENAI_API_KEY", "backup": "20261106T101600Z-OPENAI_API_KEY.bak", "had_previous": true}}
```

Undo:

```json
{"id": "20261106T102000Z-laptop-0f9e8d7c", "created_at": "2026-11-06T10:20:00Z", "host": "laptop", "kind": "undo",
 "request": null, "reply": null, "undoes": "20261106T101500Z-laptop-3fa9c1d2", "secret": null,
 "operations": [{"label": "Rename model", "op": "set_setting", "test": null,
   "effects": [{"target": {"type": "setting", "path": "models[gpt].display_name"}, "before": "GPT 5", "after": "GPT"}]}]}
```

## Loading and state

`load_all(dir)` reads every `*.json`, skips unparsable files with a `warn` naming the file, sorts by `created_at` then `id`. `entry_state(entries, id)` is `Undone` when a non-undone `undo` entry has `undoes == id`, else `Applied`; an undo of an undo makes the first entry applied again.

## Pruning

`prune(dir, backup_dir, retention_days, now)` deletes entry files older than the retention and the backup file each names in `secret.backup`; `retention_days == 0` deletes nothing. Called at startup (`setup/state.rs`) and in `update_history_retention`, after the history sweep; failures are logged with `warn`, never fatal.

## Sync

`settings_sync::detect` adds `<settings target parent>/change-log` to the stage paths only when the directory exists. A host whose directory does not exist yet gets the first pulled entries through the rebase but no `change-log-changed` event until the next cycle. After a pull that changed a path under the directory, `reload_changed` sets `Reloaded.change_log` and the app emits `change-log-changed` (no payload).
