# Log Configuration

Environment-specific configuration, targets, rotation, and performance.

## Current setup

Configured in `src-tauri/src/lib.rs` via `tauri-plugin-log`:

- **Targets**: stdout, log file (`LogDir`), webview console
- **Global level**: `Info`; app crate (`app_lib`): `Debug`
- **Timestamps**: local timezone
- **Override**: `RUST_LOG` env var

## Environment-based levels

```rust
let base_level = if cfg!(debug_assertions) {
    log::LevelFilter::Debug
} else {
    log::LevelFilter::Info
};
```

### Runtime override

```bash
RUST_LOG=trace pnpm tauri dev                      # everything at trace
RUST_LOG=app_lib=trace pnpm tauri dev              # only app code
RUST_LOG=app_lib=trace,hyper=warn pnpm tauri dev   # fine-grained
```

## Production targets

In release builds, consider removing stdout and webview targets:

```rust
.targets([
    #[cfg(debug_assertions)]
    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }),
    #[cfg(debug_assertions)]
    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
])
```

Silence noisy library crates:

```rust
.level_for("hyper", log::LevelFilter::Warn)
.level_for("reqwest", log::LevelFilter::Warn)
.level_for("tao", log::LevelFilter::Warn)
```

## Log file locations

| Platform | Path |
|----------|------|
| macOS | `~/Library/Logs/{bundleIdentifier}/` |
| Linux | `~/.config/{bundleIdentifier}/` |
| Windows | `%APPDATA%/{bundleIdentifier}/logs/` |

## Rotation

```rust
.max_file_size(10_000_000) // 10 MB per file
.rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
```

Strategies: `KeepAll`, `KeepLastDays(n)`, `DeleteOnStartup`.

## Performance

Log macros short-circuit when the level is disabled — no cost. Guard expensive computations:

```rust
if log::log_enabled!(log::Level::Trace) {
    log::trace!("state: {}", expensive_summary(&state));
}
```

In hot paths (SSE streams), use `trace!` so logs are off by default. Log summaries after completion at `info!` level.

Frontend `info()`, `warn()`, `error()` from `@tauri-apps/plugin-log` are async — await in sequential code, fire-and-forget in event handlers.
