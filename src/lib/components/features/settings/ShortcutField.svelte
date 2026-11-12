<script lang="ts">
  import { error as logError } from "@tauri-apps/plugin-log";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { Plus, X } from "lucide-svelte";
  import { addShortcut, removeShortcut } from "$lib/services/settings";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { actionShortcuts, formatShortcut } from "$lib/utils/keymapShortcuts";
  import { isMac } from "$lib/utils/platform";
  import SettingField from "./SettingField.svelte";

  let { action }: { action: string } = $props();

  const store = getSettingsStore();

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

  const shortcuts = $derived(
    actionShortcuts(store.settings?.keymaps ?? [], action, isMac ? "macos" : "linux"),
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

  async function add(shortcut: string) {
    try {
      await addShortcut(action, shortcut);
    } catch (e) {
      logError(`add_shortcut ${action} failed: ${e}`);
    }
  }

  async function remove(shortcut: string) {
    try {
      await removeShortcut(action, shortcut);
    } catch (e) {
      logError(`remove_shortcut ${action} failed: ${e}`);
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
    void add([...mods, key].join("+"));
  }
</script>

<SettingField key={`keymaps.${action}`}>
  <div class="shortcuts">
    {#each shortcuts as shortcut (shortcut)}
      <kbd class="chip">
        {formatShortcut(shortcut)}
        <button
          type="button"
          title={`Remove ${formatShortcut(shortcut)}`}
          aria-label={`Remove ${formatShortcut(shortcut)}`}
          onclick={() => remove(shortcut)}
        >
          <X size={ICON_SIZE.sm} />
        </button>
      </kbd>
    {:else}
      <span class="none">Not set</span>
    {/each}
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
      <Button icon={Plus} onclick={startCapture}>Add shortcut</Button>
    {/if}
  </div>
</SettingField>

<style>
  .shortcuts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .chip,
  .capture {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    background: var(--surface-sunken);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .chip {
    padding-right: var(--space-2);
  }

  .chip button {
    display: inline-flex;
    align-items: center;
    padding: 0 var(--space-1);
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .chip button:hover {
    color: var(--danger);
  }

  .none {
    font-family: var(--font-mono);
    font-size: var(--font-size-md);
    color: var(--text-muted);
  }

  .capture {
    border-color: var(--accent);
    outline: none;
    color: var(--text-muted);
  }
</style>
