# Promptheus Tauri

Desktop application built with **Tauri 2**.

## Rules

- Always use the latest versions of Tauri, its plugins, and all dependencies. Before adding any dependency, verify the current latest version (`cargo search`, `npm info`, etc.) — do not trust versions from task files or memory, they may be outdated.
- When using a framework API or plugin not yet documented, follow the process in [`docs/api-verification.md`](docs/api-verification.md).
- When working in any directory, check for a `DOCS.md` file first — it contains conventions and patterns for that area, or references to detailed files for complex topics.

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
- **CSS opacity does not work on transparent WebKitGTK windows on Linux.** Neither `opacity`, `rgba()` backgrounds, `filter: opacity()`, `will-change: opacity`, nor `@keyframes` animations produce visible transparency on elements inside a `.transparent(true)` window. Only the fade-out CSS transition briefly shows transparency (WebKitGTK uses a GPU composite path during transitions but reverts to an opaque CPU paint path for static renders). The **only working approach** is GTK-level window opacity via `gtk_window().set_opacity()` in Rust (see `notification.rs`). This sets `_NET_WM_WINDOW_OPACITY` at the compositor level, bypassing WebKitGTK entirely.

## References

_(to be added as the project grows)_
