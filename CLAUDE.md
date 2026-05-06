# Promptheus Tauri

Desktop application built with **Tauri 2**.

## Rules

- Always use the latest versions of Tauri, its plugins, and all dependencies. Before adding any dependency, verify the current latest version (`cargo search`, `npm info`, etc.) — do not trust versions from task files or memory, they may be outdated.
- **Prefer official `tauri-plugin-*` over rolling your own.** Before writing a service that wraps an OS API, file dialog, shell access, autostart, single-instance, logging, updater, or persistent KV storage, check whether an official plugin already covers it. Only roll your own when the plugin is genuinely insufficient (current example: `services/config/` over `tauri-plugin-store`, justified by hot-reload + migration needs — see `src-tauri/DOCS.md` "Configuration"). When you do deviate, record the rationale in the relevant `DOCS.md`.
- When using a framework API or plugin not yet documented, follow the process in [`docs/api-verification.md`](docs/api-verification.md).
- When working in any directory, check for a `DOCS.md` file first — it contains conventions and patterns for that area, or references to detailed files for complex topics.
- **Global keyboard shortcuts** (`window.addEventListener("keydown", …)`) must use **bubble phase** — never `capture: true`. Components that own a key in some state (open dropdown, modal, autocomplete) call `e.stopPropagation()` in their own handler. This lets the focused element win naturally; capture-phase global listeners break in-context popovers (e.g. Cmd+K hijacked from a skill autocomplete).

## Logging

Add log statements **during implementation**, not as a separate step. When writing or modifying any service, command, or store, include appropriate logging inline following the guides below. Do not add temporary debug logs — use proper levels and increase verbosity at runtime with `RUST_LOG`:

```bash
RUST_LOG=app_lib::services::ai=trace pnpm tauri dev   # trace for one module
RUST_LOG=app_lib=trace,hyper=warn pnpm tauri dev       # app at trace, silence noisy crates
```

Before adding logs, load only the guide you need from `docs/`:

- **Choosing a level?** → read [`docs/logging-levels.docs.md`](docs/logging-levels.docs.md)
- **Handling user data, API keys, or clipboard?** → read [`docs/logging-sensitivity.docs.md`](docs/logging-sensitivity.docs.md)
- **Changing log config, targets, or rotation?** → read [`docs/logging-config.docs.md`](docs/logging-config.docs.md)

Do not load all three — load only what applies to the current task.

## Code style

- **No inline comments.** Code must be self-explanatory — use clear names, small functions, and logical structure instead of comments. If code needs a comment to be understood, refactor it.
- Top-level doc comments (`///` in Rust, `/** */` in TS) are acceptable only when they add real value (e.g., documenting a public API contract that isn't obvious from the signature). Do not add trivial doc comments that restate the function name.
- **Icons**: Always use `lucide-svelte` for icons. Never paste raw SVG — import the named component instead (e.g., `import { MessageSquareShare } from "lucide-svelte"`).

## Documentation convention

Documentation lives close to the code it describes:

- Each directory may have a `DOCS.md` — the entry point for that directory's conventions.
- `DOCS.md` is kept short. For complex topics it references separate files in the same directory (e.g., `auth.docs.md`).
- A top-level index at [`DOCS.md`](DOCS.md) links to all directory-level docs for discoverability.

## Development Commands

All commands run from `promptheus-tauri/` unless noted otherwise.

### Frontend (Svelte/TypeScript)

| Command | What it does |
|---------|-------------|
| `npx svelte-check` | Type-check all Svelte and TS files (0 errors = pass) |
| `pnpm build` | Vite production build to `dist/` |
| `pnpm dev` | Vite dev server |

### Backend (Rust)

Run from `promptheus-tauri/src-tauri/`:

| Command | What it does |
|---------|-------------|
| `cargo check` | Type-check Rust code (fast, no codegen) |
| `cargo test --lib` | Run unit tests |
| `cargo build` | Full debug build |

### Full app

| Command | What it does |
|---------|-------------|
| `pnpm tauri dev` | Run the full Tauri app in dev mode (frontend + backend) |
| `pnpm tauri build` | Production bundle |

### Workflow rules

- `pnpm check` does **not** exist — use `npx svelte-check`.
- `cargo` commands must run from `src-tauri/`, not the project root.
- `pnpm` commands run from the project root (`promptheus-tauri/`).
- Every HTML window must be listed in `vite.config.ts` → `rollupOptions.input` and added to `src-tauri/capabilities/default.json` → `"windows"`. Missing the first → empty white window in production. Missing the second → all `invoke()` calls fail silently.
- `invoke()` naming: command names are **snake_case** (matches Rust fn name), parameter names are **camelCase** (e.g. `invoke("get_context_menu_items", { itemId, shiftPressed })`). Recurring source of silent failures.

### Platform & architecture gotchas

These are recurring traps — each lives in its own file with full background, symptom, and fix pattern. Load the file when working in the listed area; otherwise leave it.

- [`docs/gotchas/tauri-command-threading.md`](docs/gotchas/tauri-command-threading.md) — sync `#[tauri::command]` runs on the GTK main thread on Linux; blocking calls freeze the whole app. **Load when:** writing or modifying any `#[tauri::command]` that touches clipboard, filesystem, subprocess, network, or X11/GTK APIs.
- [`docs/gotchas/paste-handler.md`](docs/gotchas/paste-handler.md) — `Shift+Cmd/Ctrl+V` raw-text paste needs a Mac vs Linux/Windows split; has regressed multiple times. **Load when:** touching `InputArea.svelte → handleKeydown`, `src/lib/utils/paste.ts`, or any clipboard Tauri command, or investigating paste freezes / paste-does-nothing reports.
- [`docs/gotchas/linux-gtk-focus.md`](docs/gotchas/linux-gtk-focus.md) — `set_focus()` is rejected by GNOME/KWin as a focus-steal; need `present_with_time` with a real `x11_get_server_time` timestamp. **Load when:** adding a window that grabs focus, or investigating "window appears but no keyboard focus" on Linux.
- [`docs/gotchas/linux-webkit-opacity.md`](docs/gotchas/linux-webkit-opacity.md) — CSS `opacity` / `rgba` does nothing on `.transparent(true)` windows on Linux; use GTK `set_opacity()` instead. **Load when:** styling or animating any transparent-window UI, or investigating "looks opaque on Linux, transparent on Mac".

## References

_(to be added as the project grows)_
