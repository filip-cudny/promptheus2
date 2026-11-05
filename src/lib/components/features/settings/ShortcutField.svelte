<script lang="ts">
  import { error as logError } from "@tauri-apps/plugin-log";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { setShortcut } from "$lib/services/settings";
  import { isMac } from "$lib/utils/platform";
  import SettingField from "./SettingField.svelte";

  let { action }: { action: string } = $props();

  const store = getSettingsStore();
  const osContext = isMac ? "os==macos" : "os==linux";

  const KEY_NAMES: Record<string, string> = {
    Space: "space",
    Tab: "tab",
    Enter: "enter",
    Backspace: "backspace",
    Delete: "delete",
    ArrowUp: "up",
    ArrowDown: "down",
    ArrowLeft: "left",
    ArrowRight: "right",
    Home: "home",
    End: "end",
    PageUp: "page_up",
    PageDown: "page_down",
  };

  const binding = $derived(
    store.settings?.keymaps
      .filter((group) => group.context.replace(/\s+/g, "") === osContext)
      .flatMap((group) => Object.entries(group.bindings))
      .find(([, bound]) => bound === action)?.[0] ?? null,
  );

  let capturing = $state(false);
  let captureEl = $state<HTMLElement | undefined>();

  function keyName(code: string): string | null {
    if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
    if (/^Digit[0-9]$/.test(code)) return code.slice(5);
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code.toLowerCase();
    return KEY_NAMES[code] ?? null;
  }

  function modifiers(e: KeyboardEvent): string[] {
    const meta = isMac ? "cmd" : "super";
    return [
      e.metaKey && meta,
      e.ctrlKey && "ctrl",
      e.altKey && "alt",
      e.shiftKey && "shift",
    ].filter((m): m is string => !!m);
  }

  async function save(shortcut: string | null) {
    try {
      await setShortcut(action, shortcut);
    } catch (e) {
      logError(`set_shortcut ${action} failed: ${e}`);
    }
  }

  async function startCapture() {
    capturing = true;
    await Promise.resolve();
    captureEl?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Escape") {
      capturing = false;
      return;
    }
    const key = keyName(e.code);
    const mods = modifiers(e);
    if (!key || mods.length === 0) return;
    capturing = false;
    void save([...mods, key].join("+"));
  }
</script>

<SettingField key={`keymaps.${action}`}>
  <div class="shortcut">
    {#if capturing}
      <span
        class="capture"
        role="textbox"
        tabindex="0"
        aria-label="Press the new shortcut"
        bind:this={captureEl}
        onkeydown={onKeydown}
        onblur={() => (capturing = false)}
      >
        Press a shortcut… (Esc cancels)
      </span>
    {:else}
      <kbd class="binding">{binding ?? "Not set"}</kbd>
      <Button onclick={startCapture}>Change</Button>
      {#if binding}
        <Button onclick={() => save(null)}>Clear</Button>
      {/if}
    {/if}
  </div>
</SettingField>

<style>
  .shortcut {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .binding,
  .capture {
    padding: var(--space-2) var(--space-3);
    background: var(--surface-sunken);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .capture {
    border-color: var(--accent);
    outline: none;
    color: var(--text-muted);
  }
</style>
