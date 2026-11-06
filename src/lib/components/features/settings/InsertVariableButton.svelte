<script lang="ts">
  import { Braces } from "lucide-svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import { ICON_SIZE } from "$lib/constants/ui";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import { getEnvVars } from "$lib/services/settings";
  import { insertAt } from "$lib/utils/envVarOptions";
  import type { EnvVarStatus } from "$lib/types";

  let {
    field,
    oninsert,
    ariaLabel = "Insert variable",
  }: {
    field: () => HTMLInputElement | HTMLTextAreaElement | null | undefined;
    oninsert: (text: string) => void;
    ariaLabel?: string;
  } = $props();

  let buttonEl = $state<HTMLButtonElement | undefined>();
  let open = $state(false);
  let vars = $state<EnvVarStatus[]>([]);

  async function toggle() {
    open = !open;
    if (!open) return;
    try {
      vars = await getEnvVars();
    } catch (e) {
      logWarn("Failed to read get_env_vars: " + e);
    }
  }

  function insert(name: string) {
    open = false;
    const el = field();
    if (!el) return;
    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? el.value.length;
    const { text, caret } = insertAt(el.value, start, end, `\${${name}}`);
    el.value = text;
    oninsert(text);
    el.focus();
    el.setSelectionRange(caret, caret);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape" || !open) return;
    e.stopPropagation();
    open = false;
    buttonEl?.focus();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<button
  bind:this={buttonEl}
  type="button"
  class="editor-icon-btn"
  class:active={open}
  aria-label={ariaLabel}
  title={ariaLabel}
  onclick={toggle}
>
  <Braces size={ICON_SIZE.md} />
</button>

<FloatingPanel visible={open} anchorEl={buttonEl} position="below" fitContent onclose={() => (open = false)}>
  <MenuList>
    {#each vars as v (v.name)}
      <MenuItem label={v.name} onclick={() => insert(v.name)}>
        {#snippet shortcut()}{#if !v.set}missing{/if}{/snippet}
      </MenuItem>
    {:else}
      <div class="menu-list-info">No variables listed.</div>
    {/each}
  </MenuList>
</FloatingPanel>
