# Log Configuration

Targets, level resolution, `RUST_LOG`, log file location, rotation, and performance.

## Current setup

Configured in `src-tauri/src/setup/log.rs` via `tauri-plugin-log`, identical in every build:

- **Targets**: stdout, log file (`LogDir { file_name: None }`), webview console
- **Global level** (`.level`): `Info`
- **App crate** (`.level_for("app_lib", ...)`): `Debug`
- **Timestamps**: local timezone (`TimezoneStrategy::UseLocal`)
- **Override**: `RUST_LOG` env var, parsed in `log.rs`
- **Rotation**: not configured, the plugin defaults apply (see "Rotation")

## Runtime override

`log.rs` splits `RUST_LOG` on `,` and handles each directive:

- `module=level` calls `.level_for(module, level)`; it comes after the built-in `app_lib` entry, so it replaces it for the same module name
- a bare `level` calls `.level(level)`, the global fallback
- a directive whose level does not parse is ignored

fern resolves a log record's target (`app_lib::services::ai`) in `Dispatch::shallow_enabled` (`fern-0.7.1/src/log_impl.rs:494-500`): the level from `find_module` wins, and the global level is used only when `find_module` finds nothing (`unwrap_or(self.default_level)`). `find_module` (`log_impl.rs:282-310`) tries the exact target, then each parent module (`hyper::http::h1` → `hyper::http` → `hyper`). Consequence: any target under `app_lib` always matches the `app_lib` entry, so a bare level never changes app code.

```bash
RUST_LOG=trace pnpm tauri dev                          # no effect on app_lib (stays Debug); raises the global level for other crates
RUST_LOG=app_lib=trace pnpm tauri dev                  # all app code at trace
RUST_LOG=app_lib::services::ai=trace pnpm tauri dev    # one module at trace, rest of app_lib at Debug
RUST_LOG=app_lib=trace,hyper=warn pnpm tauri dev       # app at trace, hyper at warn
```

`app_lib::services::ai=trace` works because the exact target match for `app_lib::services::ai` is tried before the parent `app_lib`.

## Log file locations

`TargetKind::LogDir` resolves to `app_log_dir()` (`tauri-plugin-log-2.8.0/src/lib.rs:627-628`; `tauri-2.10.3/src/path/desktop.rs:275-288`). The bundle identifier is `com.promptheus.desktop` (`src-tauri/tauri.conf.json`).

| Platform | Path |
|----------|------|
| macOS | `~/Library/Logs/com.promptheus.desktop/` |
| Linux | `~/.local/share/com.promptheus.desktop/logs/` (`$XDG_DATA_HOME` if set) |

## Rotation

`log.rs` sets no rotation, so the plugin defaults apply (`tauri-plugin-log-2.8.0/src/lib.rs:46-47`):

- max file size: `DEFAULT_MAX_FILE_SIZE = 40000` bytes
- strategy: `DEFAULT_ROTATION_STRATEGY = RotationStrategy::KeepOne`

Strategies (`lib.rs:123-130`, behaviour in `rotate`, `lib.rs:206-226`):

- `KeepAll`: renames the full file to a dated name, keeps every file
- `KeepOne`: deletes the full file, so only the current log survives
- `KeepSome(usize)`: keeps that many files, the current one included, removing the oldest

Change them with `.max_file_size(..)` and `.rotation_strategy(..)` on the builder in `log.rs`.

## Performance

Log macros short-circuit when the level is disabled — no cost. Guard expensive computations:

```rust
if log::log_enabled!(log::Level::Trace) {
    log::trace!("state: {}", expensive_summary(&state));
}
```

In hot paths (SSE streams), use `trace!` so logs are off by default. Log summaries after completion at `info!` level.

Frontend `info()`, `warn()`, `error()` from `@tauri-apps/plugin-log` are async — await in sequential code, fire-and-forget in event handlers.
