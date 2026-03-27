# UI Components

Reusable UI primitives shared across features. See [src/DOCS.md](../../../DOCS.md) for general frontend conventions.

## Directory Structure

```
ui/
└── NotificationToast.svelte   # Global toast overlay (fixed position, bottom-right)
```

## Conventions

### Component patterns

- Components import state from `$lib/stores/` and types from `$lib/types` — no direct `invoke()` calls.
- Scoped `<style>` blocks for all CSS — no global style leakage.
- Custom Svelte transitions defined inline when built-in transitions (`fade`, `fly`) don't match the required animation curve (e.g., capped opacity).

### Toast component

- `NotificationToast` is a global overlay mounted once in `App.svelte` — not instantiated per-notification.
- Notifications are driven by the `notifications` store; the component iterates the reactive array with a keyed `{#each}`.
- Removal is triggered by the store's auto-dismiss timer; `onoutroend` calls `remove()` as a cleanup safety net after the out-transition completes.
- Visual config (icon colors, icons, opacity) is hardcoded as constants — will be wired to the settings store when available.
