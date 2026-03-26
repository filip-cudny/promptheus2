# Promptheus Tauri

Desktop application built with **Tauri 2**.

## Rules

- Always use the latest versions of Tauri, its plugins, and all dependencies. Before adding any dependency, verify the current latest version (`cargo search`, `npm info`, etc.) — do not trust versions from task files or memory, they may be outdated.
- When using a framework API or plugin not yet documented, follow the process in [`docs/api-verification.md`](docs/api-verification.md).
- When working in any directory, check for a `DOCS.md` file first — it contains conventions and patterns for that area, or references to detailed files for complex topics.

## Code style

- **No inline comments.** Code must be self-explanatory — use clear names, small functions, and logical structure instead of comments. If code needs a comment to be understood, refactor it.
- Top-level doc comments (`///` in Rust, `/** */` in TS) are acceptable only when they add real value (e.g., documenting a public API contract that isn't obvious from the signature). Do not add trivial doc comments that restate the function name.

## Documentation convention

Documentation lives close to the code it describes:

- Each directory may have a `DOCS.md` — the entry point for that directory's conventions.
- `DOCS.md` is kept short. For complex topics it references separate files in the same directory (e.g., `auth.docs.md`).
- A top-level index at [`DOCS.md`](DOCS.md) links to all directory-level docs for discoverability.

## References

_(to be added as the project grows)_
