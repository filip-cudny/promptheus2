<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import SelectDropdown from "$lib/components/shared/ui/SelectDropdown.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import { onSettingsChanged } from "$lib/services/events";
  import { applyEnvVarOperation, getEnvVars } from "$lib/services/settings";
  import { classifyReference, pickerOptions } from "$lib/utils/envVarOptions";
  import type { EnvVarStatus } from "$lib/types";
  import SecretField, { type SecretOutcome } from "./SecretField.svelte";

  const NEW_ACTION = "__new_env_var__";
  const ROW_PREFIX = /^row \d+: /;

  let {
    value,
    onchange,
  }: {
    value: string;
    onchange: (value: string) => void;
  } = $props();

  type Step =
    | { kind: "form"; moving: boolean }
    | { kind: "value"; name: string; moving: boolean };

  let vars = $state<EnvVarStatus[]>([]);
  let step = $state<Step | null>(null);
  let newName = $state("");
  let newDescription = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let unlisten: (() => void) | null = null;

  const reference = $derived(
    classifyReference(
      value,
      vars.map((v) => v.name),
    ),
  );
  const options = $derived(pickerOptions(vars, value, [{ value: NEW_ACTION, label: "New…" }]));

  async function refresh() {
    try {
      vars = await getEnvVars();
    } catch (e) {
      logWarn("Failed to read get_env_vars: " + e);
    }
  }

  async function addToList(name: string, description: string): Promise<boolean> {
    error = null;
    busy = true;
    try {
      await applyEnvVarOperation({ op: "add_env_var", name, description, secret: true });
    } catch (e) {
      error = String(e).replace(ROW_PREFIX, "");
      return false;
    } finally {
      busy = false;
    }
    await refresh();
    return true;
  }

  function pick(next: string) {
    error = null;
    if (next === NEW_ACTION) {
      startForm(false);
      return;
    }
    step = null;
    if (next !== value) onchange(next);
  }

  function startForm(moving: boolean) {
    newName = "";
    newDescription = "";
    error = null;
    step = { kind: "form", moving };
  }

  async function submitForm() {
    if (step?.kind !== "form" || busy) return;
    const name = newName.trim();
    if (!name) {
      error = "Name is required.";
      return;
    }
    if (await addToList(name, newDescription)) {
      step = { kind: "value", name, moving: step.moving };
    }
  }

  function finishValue(outcome: SecretOutcome) {
    if (step?.kind !== "value") return;
    const { name, moving } = step;
    step = null;
    if (outcome === "saved" || !moving) onchange(`\${${name}}`);
  }

  function swallowEnterSubmit(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.target as HTMLElement).tagName === "INPUT") e.preventDefault();
  }

  onMount(async () => {
    unlisten = await onSettingsChanged(refresh);
    await refresh();
  });

  onDestroy(() => unlisten?.());
</script>

<div class="picker" role="presentation" onkeydown={swallowEnterSubmit}>
  <div class="row">
    <SelectDropdown bind:value={() => value, pick} {options} ariaLabel="API key" />
    {#if reference.kind === "unlisted"}
      <Button disabled={busy} onclick={() => addToList(reference.name, "")}>Add to list</Button>
    {:else if reference.kind === "literal"}
      <Button onclick={() => startForm(true)}>Move to a variable</Button>
    {/if}
  </div>

  {#if step?.kind === "form"}
    <div class="form">
      <TextInput
        bind:value={newName}
        placeholder="NAME"
        aria-label="Variable name"
        error={Boolean(error)}
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && submitForm()}
      />
      <TextInput
        bind:value={newDescription}
        placeholder="Description"
        aria-label="Variable description"
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && submitForm()}
      />
      <div class="actions">
        <Button onclick={() => (step = null)}>Cancel</Button>
        <Button variant="primary" disabled={busy} onclick={submitForm}>Add</Button>
      </div>
    </div>
  {:else if step?.kind === "value"}
    <SecretField name={step.name} exists={false} masked onclose={finishValue} />
  {/if}

  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .picker {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .error {
    margin: var(--space-0);
    font-size: var(--font-size-sm);
    color: var(--danger);
  }
</style>
