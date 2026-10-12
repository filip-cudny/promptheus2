# Promptheus

A tray-resident AI prompt launcher with a global hotkey, clipboard-aware context menu, and streaming chat — built on Tauri 2, Svelte 5, and Rust.

## Status

Pre-1.0. No public binaries yet — build from source.

## Features

- Tray-only desktop app with a global-hotkey context menu over the focused app's clipboard/selection.
- Streaming chat against the OpenAI Responses and Completions APIs and any OpenAI-compatible endpoint (vLLM, MiniMax, local servers).
- AI provider webviews: chat with provider web apps in their own windows.
- Prompts as skills with `{{placeholder}}` variables.
- MCP client (rmcp) for external tool servers.
- Speech-to-text — both as an alternate input in chat and a standalone dictation flow (OpenAI Whisper or ElevenLabs).
- Voice recording pill, with a GNOME Shell extension on GNOME Wayland.
- Persistent conversation history in SQLite with full-text search.
- Multi-window UI: context menu, conversation dialog, command palette, history browser, settings.
- Platforms: macOS and Linux.

## Quick start

Install [Node.js ≥ 20](https://nodejs.org), [pnpm ≥ 9](https://pnpm.io/installation), and [Rust stable](https://www.rust-lang.org/tools/install). Then:

```bash
pnpm install
pnpm tauri dev
```

The first run compiles the Rust backend (a few minutes). Subsequent runs are incremental.

<details>
<summary>System dependencies</summary>

### macOS

```bash
xcode-select --install
```

### Linux (Debian / Ubuntu)

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libasound2-dev
```

### Linux (Fedora)

```bash
sudo dnf install -y \
  gcc-c++ openssl-devel gtk3-devel \
  libappindicator-gtk3-devel librsvg2-devel \
  webkit2gtk4.1-devel javascriptcoregtk4.1-devel libsoup3-devel alsa-lib-devel
```

### Linux (Arch)

```bash
sudo pacman -S --needed \
  base-devel openssl gtk3 libappindicator-gtk3 \
  librsvg webkit2gtk-4.1 libsoup3 alsa-lib
```

</details>

## Configuration

Bundle identifier: `com.promptheus.desktop`. App config and data live under the platform-standard directory for that ID:

| Platform | Config directory |
|----------|------------------|
| macOS    | `~/Library/Application Support/com.promptheus.desktop/` |
| Linux    | `~/.config/com.promptheus.desktop/` |

API keys are read from `<config directory>/.env` first; then a `.env` found in the working directory or any parent (in development, `promptheus-tauri/.env`) is loaded and overrides it. The full settings schema is the `Settings` struct in `src-tauri/src/models/settings.rs`.

## Project layout

- `src/` — Svelte 5 frontend ([`src/DOCS.md`](src/DOCS.md))
- `src-tauri/` — Rust backend ([`src-tauri/DOCS.md`](src-tauri/DOCS.md))
- `docs/` — gotchas, logging, API verification process
- `*.html` — one entry per Tauri window (context menu, palette, settings, …)
- `package.json`, `pnpm-lock.yaml` — frontend dependencies
- `vite.config.ts`, `svelte.config.js` — bundler and Svelte preprocessor

## Development

Run all `pnpm` commands from the repository root; run `cargo` commands from `src-tauri/`.

| Command | Where | What it does |
|---------|-------|--------------|
| `pnpm tauri dev`           | root        | Run the full app in development mode |
| `pnpm tauri build`         | root        | Build a production bundle |
| `npx svelte-check`         | root        | Type-check the Svelte frontend |
| `cargo check`              | `src-tauri` | Type-check the Rust backend |
| `cargo test --lib`         | `src-tauri` | Run backend unit tests |

For verbose backend logs:

```bash
RUST_LOG=app_lib=trace pnpm tauri dev
```

See [`docs/logging-config.docs.md`](docs/logging-config.docs.md) for per-module targets and rotation.

## Documentation

- [`CLAUDE.md`](CLAUDE.md) — code style, workflow rules, recurring gotchas
- [`DOCS.md`](DOCS.md) — full per-directory documentation index
- [`docs/gotchas/`](docs/gotchas/) — Linux- and macOS-specific traps with fix patterns
- [`docs/api-verification.md`](docs/api-verification.md) — process for verifying framework APIs

## License

MIT — see [LICENSE](LICENSE).
