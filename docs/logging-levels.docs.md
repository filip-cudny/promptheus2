# Log Levels Reference

When to use each level, with examples from the codebase.

## Error

Something failed, operation cannot continue.

```rust
log::error!("failed to load settings: {}", e);
log::error!("AI request failed for model {}: {}", model_id, e);
```

```typescript
error("Prompt execution failed: " + message);
```

Use for: unrecoverable failures, service initialization errors, operations needing user intervention.

## Warn

Unexpected but the app continues.

```rust
log::warn!("model '{}' unavailable: {}", model.id, e);
log::warn!("invalid shortcut {}: {}", shortcut_str, e);
```

Use for: recoverable errors, fallback paths, missing optional config, failed non-critical features.

## Info

High-level events confirming the app works.

```rust
log::info!("config loaded from {}", config_dir.display());
log::info!("registered shortcut: {} -> {}", shortcut_str, action);
log::info!("skill {} executed in {:.1}s, {} tokens", skill_id, elapsed, token_count);
```

Use for: lifecycle events, config loaded/saved, user-triggered actions (log action, not content), counts and durations.

## Debug

Useful during development, too noisy for production.

```rust
log::debug!("completion request: model={}, messages={}", model_id, msg_count);
log::debug!("config key changed: {}", key_name);
```

Use for: request metadata (IDs, sizes — not content), state transitions, branch decisions.

## Trace

Deep debugging only. Almost never enabled.

```rust
log::trace!("SSE line: {:?}", line_type);
```

Use for: function entry/exit in hot paths, raw protocol data. Enable only when hunting a specific bug.
