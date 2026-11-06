<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { AlertCircle, Check } from "lucide-svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import { onSettingsChanged } from "$lib/services/events";
  import { applyEnvVarOperation, getEnvVars } from "$lib/services/settings";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import type { EnvVarOperation, EnvVarStatus } from "$lib/types";
  import SettingsSection from "./SettingsSection.svelte";
  import SecretField from "./SecretField.svelte";

  const NAME_PATTERN = /^[A-Za-z_][A-Za-z0-9_]*$/;
  const NAME_ERROR = "Use letters, digits and underscores; the first character is not a digit.";
  const ROW_PREFIX = /^row \d+: /;

  const store = getSettingsStore();

  let vars = $state<EnvVarStatus[]>([]);
  let loaded = $state(false);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let valueFor = $state<string | null>(null);
  let renaming = $state<string | null>(null);
  let renameText = $state("");
  let confirmingDelete = $state<string | null>(null);

  let newName = $state("");
  let newDescription = $state("");
  let newSecret = $state(true);
  let addError = $state<string | null>(null);

  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  let unlisten: (() => void) | null = null;

  async function refresh() {
    try {
      vars = await getEnvVars();
      loaded = true;
    } catch (e) {
      logWarn("Failed to read get_env_vars: " + e);
    }
  }

  async function apply(operation: EnvVarOperation): Promise<string | null> {
    try {
      await applyEnvVarOperation(operation);
    } catch (e) {
      return String(e).replace(ROW_PREFIX, "");
    }
    await refresh();
    return null;
  }

  function descriptionOf(v: EnvVarStatus): string {
    return drafts[v.name] ?? v.description;
  }

  async function saveRow(name: string, secret?: boolean): Promise<boolean> {
    clearTimeout(timers.get(name));
    timers.delete(name);
    const v = vars.find((x) => x.name === name);
    if (!v) return false;
    delete errors[name];
    const description = drafts[name] ?? v.description;
    const error = await apply({
      op: "update_env_var",
      name,
      description,
      secret: secret ?? v.secret,
    });
    if (error) {
      errors[name] = error;
      return false;
    }
    if (drafts[name] === description) delete drafts[name];
    return true;
  }

  function editDescription(v: EnvVarStatus, text: string) {
    drafts[v.name] = text;
    clearTimeout(timers.get(v.name));
    timers.set(
      v.name,
      setTimeout(() => void saveRow(v.name), store.settings?.autosave_debounce_ms ?? 1000),
    );
  }

  async function toggleSecret(v: EnvVarStatus, input: HTMLInputElement) {
    if (!(await saveRow(v.name, input.checked))) input.checked = v.secret;
  }

  async function flushPending() {
    for (const name of [...timers.keys()]) await saveRow(name);
  }

  async function add() {
    const name = newName.trim();
    addError = null;
    if (!NAME_PATTERN.test(name)) {
      addError = NAME_ERROR;
      return;
    }
    const error = await apply({
      op: "add_env_var",
      name,
      description: newDescription,
      secret: newSecret,
    });
    if (error) {
      addError = error;
      return;
    }
    newName = "";
    newDescription = "";
    valueFor = name;
  }

  function startRename(v: EnvVarStatus) {
    delete errors[v.name];
    renaming = v.name;
    renameText = v.name;
  }

  async function rename(v: EnvVarStatus) {
    const newNameValue = renameText.trim();
    if (newNameValue === v.name) {
      renaming = null;
      return;
    }
    if (!NAME_PATTERN.test(newNameValue)) {
      errors[v.name] = NAME_ERROR;
      return;
    }
    if (timers.has(v.name) && !(await saveRow(v.name))) return;
    delete errors[v.name];
    const error = await apply({ op: "rename_env_var", name: v.name, new_name: newNameValue });
    if (error) {
      errors[v.name] = error;
      return;
    }
    renaming = null;
    if (valueFor === v.name) valueFor = null;
  }

  function onRenameKeydown(e: KeyboardEvent, v: EnvVarStatus) {
    if (e.key === "Enter") void rename(v);
    else if (e.key === "Escape") {
      e.stopPropagation();
      renaming = null;
      delete errors[v.name];
    }
  }

  async function remove(v: EnvVarStatus) {
    clearTimeout(timers.get(v.name));
    timers.delete(v.name);
    delete errors[v.name];
    const error = await apply({ op: "delete_env_var", name: v.name });
    confirmingDelete = null;
    if (error) errors[v.name] = error;
  }

  async function closeValue() {
    valueFor = null;
    await refresh();
  }

  onMount(async () => {
    window.addEventListener("focus", refresh);
    unlisten = await onSettingsChanged(refresh);
    await refresh();
  });

  onDestroy(() => {
    window.removeEventListener("focus", refresh);
    unlisten?.();
    void flushPending();
  });
</script>

<SettingsSection
  title="Environment"
  hint={"Variables referenced as ${NAME} in models and MCP servers. Names and descriptions sync; values stay on this machine."}
>
  {#snippet body()}
    <section class="card">
      <h3>Variables</h3>
      {#if loaded && vars.length === 0}
        <p class="muted">No variables yet.</p>
      {/if}
      {#each vars as v (v.name)}
        <div class="row">
          <div class="head">
            {#if renaming === v.name}
              <TextInput
                bind:value={renameText}
                aria-label="New name"
                error={Boolean(errors[v.name])}
                onkeydown={(e: KeyboardEvent) => onRenameKeydown(e, v)}
              />
              <Button variant="primary" onclick={() => rename(v)}>Rename</Button>
              <Button onclick={() => (renaming = null)}>Cancel</Button>
            {:else}
              <code class="name">{v.name}</code>
              {#if v.secret}<span class="marker">secret</span>{/if}
              <span class="state" class:ok={v.set} class:missing={!v.set}>
                {#if v.set}<Check size={ICON_SIZE.sm} />{:else}<AlertCircle size={ICON_SIZE.sm} />{/if}
                {v.set ? "set" : "missing"}
              </span>
              <span class="spacer"></span>
              <Button onclick={() => (valueFor = valueFor === v.name ? null : v.name)}>
                {v.set ? "Change value" : "Set value"}
              </Button>
              <Button onclick={() => startRename(v)}>Rename</Button>
              {#if confirmingDelete === v.name}
                <span class="confirm-text">Delete <strong>{v.name}</strong>?</span>
                <Button onclick={() => (confirmingDelete = null)}>Cancel</Button>
                <Button variant="danger" onclick={() => remove(v)}>Yes, delete</Button>
              {:else}
                <Button variant="danger" onclick={() => (confirmingDelete = v.name)}>Delete</Button>
              {/if}
            {/if}
          </div>

          <div class="edit">
            <TextInput
              value={descriptionOf(v)}
              placeholder="Description"
              aria-label="Description of {v.name}"
              oninput={(e: Event) => editDescription(v, (e.target as HTMLInputElement).value)}
            />
            <label class="check">
              <input
                type="checkbox"
                checked={v.secret}
                onchange={(e) => toggleSecret(v, e.currentTarget)}
              />
              Secret
            </label>
          </div>

          <p class="uses">
            {v.used_in.length > 0 ? v.used_in.map((u) => u.label).join(", ") : "not used"}
          </p>

          {#if errors[v.name]}<p class="error">{errors[v.name]}</p>{/if}

          {#if valueFor === v.name}
            <SecretField name={v.name} exists={v.set} masked={v.secret} onclose={closeValue} />
          {/if}
        </div>
      {/each}
    </section>

    <section class="card">
      <h3>Add variable</h3>
      <div class="add">
        <TextInput
          bind:value={newName}
          placeholder="NAME"
          aria-label="Variable name"
          error={Boolean(addError)}
          onkeydown={(e: KeyboardEvent) => e.key === "Enter" && add()}
        />
        <TextInput
          bind:value={newDescription}
          placeholder="Description"
          aria-label="Variable description"
          onkeydown={(e: KeyboardEvent) => e.key === "Enter" && add()}
        />
        <label class="check">
          <input type="checkbox" bind:checked={newSecret} />
          Secret
        </label>
        <Button variant="primary" onclick={add}>Add</Button>
      </div>
      {#if addError}<p class="error">{addError}</p>{/if}
    </section>
  {/snippet}
</SettingsSection>

<style>
  .card {
    background: var(--surface-base);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-lg);
    padding: var(--space-7) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 760px;
    margin-bottom: var(--space-6);
  }

  h3 {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
    margin: var(--space-0);
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-bottom: var(--space-6);
    border-bottom: 1px solid var(--border-faint);
  }

  .row:last-child {
    padding-bottom: var(--space-0);
    border-bottom: none;
  }

  .head,
  .edit,
  .add {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .head :global(.text-input),
  .edit :global(.text-input),
  .add :global(.text-input) {
    flex: 1;
    min-width: 0;
  }

  .spacer {
    flex: 1;
  }

  .name {
    font-family: var(--font-mono);
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .marker {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    padding: var(--space-1) var(--space-2);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
  }

  .state.ok {
    color: var(--success);
  }

  .state.missing {
    color: var(--danger);
  }

  .confirm-text {
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    white-space: nowrap;
  }

  .uses,
  .muted {
    margin: var(--space-0);
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .error {
    margin: var(--space-0);
    font-size: var(--font-size-sm);
    color: var(--danger);
  }
</style>
