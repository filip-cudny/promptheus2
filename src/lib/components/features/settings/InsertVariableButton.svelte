<script lang="ts">
  import { Braces } from "lucide-svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import { getEnvVars } from "$lib/services/settings";
  import { groupByKind, insertAt } from "$lib/utils/envVarOptions";
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

  const groups = $derived.by(() => {
    const { secrets, variables } = groupByKind(vars);
    return [
      { label: "Secrets", items: secrets },
      { label: "Variables", items: variables },
    ].filter((g) => g.items.length > 0);
  });

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
    if (!el) {
      oninsert(`\${${name}}`);
      return;
    }
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

<ActionIconButton icon={Braces} label={ariaLabel} active={open} bind:element={buttonEl} onclick={toggle} />

<FloatingPanel visible={open} anchorEl={buttonEl} position="below" fitContent onclose={() => (open = false)}>
  <MenuList>
    {#each groups as group (group.label)}
      <div class="group-label">{group.label}</div>
      {#each group.items as v (v.name)}
        <MenuItem label={v.name} onclick={() => insert(v.name)}>
          {#snippet shortcut()}{#if !v.set}missing{/if}{/snippet}
        </MenuItem>
      {/each}
    {:else}
      <div class="menu-list-info">No variables listed.</div>
    {/each}
  </MenuList>
</FloatingPanel>

<style>
  .group-label {
    padding: var(--space-2) var(--space-4) var(--space-1);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    color: var(--text-muted);
  }
</style>
