<script lang="ts" module>
  export type SecretOutcome = "saved" | "cancelled";
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { Eye, EyeOff } from "lucide-svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { getSecretValue, saveSecret } from "$lib/services/settingsDialog";

  let {
    name,
    purpose = "",
    exists = false,
    masked = true,
    readonly = false,
    onclose,
  }: {
    name: string;
    purpose?: string;
    exists?: boolean;
    masked?: boolean;
    readonly?: boolean;
    onclose?: (outcome: SecretOutcome) => void;
  } = $props();

  const MASK = "••••••••••••";

  let value = $state("");
  let revealed = $state(false);
  let existing = $state("");
  let loaded = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  const canSave = $derived(!!value && value !== existing && !saving);

  async function loadExisting() {
    if (!readonly && !exists) return;
    try {
      existing = (await getSecretValue(name)) ?? "";
      value = existing;
      loaded = true;
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    void loadExisting();
  });

  async function save() {
    if (!canSave) return;
    saving = true;
    error = null;
    try {
      await saveSecret(name, value);
      onclose?.("saved");
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter") save();
    else if (e.key === "Escape") {
      e.stopPropagation();
      onclose?.("cancelled");
    }
  }
</script>

{#snippet revealToggle()}
  <button class="reveal" type="button" title={revealed ? "Hide" : "Show"} onclick={() => (revealed = !revealed)}>
    {#if revealed}<EyeOff size={ICON_SIZE.md} />{:else}<Eye size={ICON_SIZE.md} />{/if}
  </button>
{/snippet}

<div class="secret">
  <div class="title"><code>{name}</code>{#if exists && !readonly}<span class="note">has a value</span>{/if}</div>
  {#if purpose}<p class="purpose">{purpose}</p>{/if}
  {#if readonly}
    <div class="input-row">
      {#if loaded && existing}
        <code class="read-value">{revealed ? existing : MASK}</code>
        {@render revealToggle()}
      {:else if loaded}
        <span class="note">not set</span>
      {/if}
    </div>
    {#if error}<div class="error">{error}</div>{/if}
  {:else}
    <div class="input-row">
      <TextInput
        bind:value
        type={masked && !revealed ? "password" : "text"}
        placeholder={exists ? "Enter a new value" : "Value"}
        autocomplete="off"
        {onkeydown}
      />
      {#if masked}
        {@render revealToggle()}
      {/if}
    </div>
    {#if error}<div class="error">{error}</div>{/if}
    <div class="actions">
      <Button variant="ghost" onclick={() => onclose?.("cancelled")}>Cancel</Button>
      <Button variant="primary" disabled={!canSave} onclick={save}>Save</Button>
    </div>
  {/if}
</div>

<style>
  .secret {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }

  .title {
    display: flex;
    gap: var(--space-3);
    align-items: baseline;
  }

  .note {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .purpose {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .input-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .input-row :global(.text-input) {
    flex: 1;
    min-width: 0;
  }

  .read-value {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .reveal {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .error {
    color: var(--danger);
    font-size: var(--font-size-sm);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
