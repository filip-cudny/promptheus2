# Promptheus Tauri

Desktop application built with **Tauri 2**.

## Rules

- Always use the latest versions of Tauri, its plugins, and all dependencies. Before adding any dependency, verify the current latest version (`cargo search`, `npm info`, etc.) — do not trust versions from task files or memory, they may be outdated.
- When using a framework API or plugin not yet documented, follow the process in [`docs/api-verification.md`](docs/api-verification.md).
- When working in any directory, check for a `DOCS.md` file first — it contains conventions and patterns for that area, or references to detailed files for complex topics.

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

### Common mistakes to avoid

- `pnpm check` does **not** exist — use `npx svelte-check`.
- `cargo` commands must run from `src-tauri/`, not the project root.
- `pnpm` commands run from the project root (`promptheus-tauri/`).
- Every HTML window **must** be listed in `vite.config.ts` → `rollupOptions.input`. Dev mode serves files from disk so missing entries still work, but production builds only include listed inputs — an unlisted window loads as an empty white rectangle.
- Every new window **must** be added to `src-tauri/capabilities/default.json` → `"windows"` array. Without this, the window cannot invoke any Tauri commands — `invoke()` calls fail silently. This is the Tauri 2 security capability system.
- **`invoke()` naming convention — command names vs parameter names are different!** Command names use **snake_case** matching the Rust function name (e.g., `invoke("get_context_menu_items", ...)`). Parameter names use **camelCase** (e.g., `{ itemId, shiftPressed }`). Getting this wrong causes silent failures — the invoke throws but errors are easy to miss in async code. This has caused hard-to-debug issues multiple times.
- **Linux/GTK focus: use `present_with_time()` with a real X11 timestamp.** Tauri's `set_focus()` calls `present_with_time(GDK_CURRENT_TIME)` which passes timestamp 0 — GNOME WM rejects this as a focus-steal attempt. Use `gdkx11::functions::x11_get_server_time()` to get a valid timestamp (see `focus_context_menu` in `commands/menu.rs`).
- **CSS opacity does not work on transparent WebKitGTK windows on Linux.** Neither `opacity`, `rgba()` backgrounds, `filter: opacity()`, `will-change: opacity`, nor `@keyframes` animations produce visible transparency on elements inside a `.transparent(true)` window. Only the fade-out CSS transition briefly shows transparency (WebKitGTK uses a GPU composite path during transitions but reverts to an opaque CPU paint path for static renders). The **only working approach** is GTK-level window opacity via `gtk_window().set_opacity()` in Rust (see `notification.rs`). This sets `_NET_WM_WINDOW_OPACITY` at the compositor level, bypassing WebKitGTK entirely.

## References

_(to be added as the project grows)_
