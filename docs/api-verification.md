# API Verification Process

This process applies whenever you need to use a Tauri framework API, plugin, or library feature that is **not yet documented** in the relevant directory's `DOCS.md`.

Do not rely on knowledge cutoff — assume it may be outdated.

## Pre-task: Verify before implementing

1. Identify which framework APIs/plugins the task will require.
2. Check the `DOCS.md` in the directory you're working in for existing verified patterns.
3. For any API **not yet documented**, look up the latest official documentation using Context7 (`mcp__context7__resolve-library-id` → `mcp__context7__query-docs`).
4. Note the correct, up-to-date usage patterns — these are what you will use during implementation.

## Post-task: Document verified patterns

After completing the implementation:

1. Add or update the `DOCS.md` in the directory where the code lives.
2. Keep `DOCS.md` short — if a topic is complex, create a separate file (e.g., `auth.docs.md`) and reference it from `DOCS.md`.
3. Include only practical usage patterns and gotchas — not full API references.
4. Add a link to the new doc in the top-level `DOCS.md` index.
