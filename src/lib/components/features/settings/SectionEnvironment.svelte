<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { AlertCircle, Check } from "lucide-svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import { onSettingsChanged } from "$lib/services/events";
  import { applyEnvVarOperation, getEnvVars } from "$lib/services/settings";
  import type { EnvVarOperation, EnvVarStatus } from "$lib/types";
  import { groupByKind } from "$lib/utils/envVarOptions";
  import SettingsSection from "./SettingsSection.svelte";
  import SecretField from "./SecretField.svelte";

  type Kind = "secrets" | "variables";

  const NAME_PATTERN = /^[A-Za-z_][A-Za-z0-9_]*$/;
  const NAME_ERROR = "Use letters, digits and underscores; the first character is not a digit.";
  const ROW_PREFIX = /^row \d+: /;

  let vars = $state<EnvVarStatus[]>([]);
  let loaded = $state(false);
  let errors = $state<Record<string, string>>({});
  let valueFor = $state<string | null>(null);
  let editing = $state<string | null>(null);
  let editName = $state("");
  let editDescription = $state("");
  let confirmingDelete = $state<string | null>(null);

  let adds = $state<Record<Kind, { name: string; description: string; error: string | null }>>({
    secrets: { name: "", description: "", error: null },
    variables: { name: "", description: "", error: null },
  });

  const groups = $derived(groupByKind(vars));

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

  async function add(kind: Kind) {
    const form = adds[kind];
    const name = form.name.trim();
    form.error = null;
    if (!NAME_PATTERN.test(name)) {
      form.error = NAME_ERROR;
      return;
    }
    const error = await apply({
      op: "add_env_var",
      name,
      description: form.description,
      secret: kind === "secrets",
    });
    if (error) {
      form.error = error;
      return;
    }
    form.name = "";
    form.description = "";
    valueFor = name;
  }

  function startEdit(v: EnvVarStatus) {
    delete errors[v.name];
    editing = v.name;
    editName = v.name;
    editDescription = v.description;
  }

  function cancelEdit(v: EnvVarStatus) {
    editing = null;
    delete errors[v.name];
  }

  async function save(v: EnvVarStatus) {
    const newName = editName.trim();
    const nameChanged = newName !== v.name;
    delete errors[v.name];
    if (nameChanged && !NAME_PATTERN.test(newName)) {
      errors[v.name] = NAME_ERROR;
      return;
    }
    if (editDescription !== v.description) {
      const error = await apply({
        op: "update_env_var",
        name: v.name,
        description: editDescription,
        secret: v.secret,
      });
      if (error) {
        errors[v.name] = error;
        return;
      }
    }
    if (nameChanged) {
      const error = await apply({ op: "rename_env_var", name: v.name, new_name: newName });
      if (error) {
        errors[v.name] = error;
        return;
      }
      if (valueFor === v.name) valueFor = null;
    }
    editing = null;
  }

  async function move(v: EnvVarStatus) {
    delete errors[v.name];
    const error = await apply({
      op: "update_env_var",
      name: v.name,
      description: v.description,
      secret: !v.secret,
    });
    if (error) {
      errors[v.name] = error;
      return;
    }
    editing = null;
  }

  function onEditKeydown(e: KeyboardEvent, v: EnvVarStatus) {
    if (e.key === "Enter") void save(v);
    else if (e.key === "Escape") {
      e.stopPropagation();
      cancelEdit(v);
    }
  }

  async function remove(v: EnvVarStatus) {
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
  });
</script>

{#snippet row(v: EnvVarStatus)}
  <div class="row">
    <div class="head">
      <code class="name">{v.name}</code>
      <span class="state" class:ok={v.set} class:missing={!v.set}>
        {#if v.set}<Check size={ICON_SIZE.sm} />{:else}<AlertCircle size={ICON_SIZE.sm} />{/if}
        {v.set ? "set" : "missing"}
      </span>
      <span class="spacer"></span>
      <Button onclick={() => (valueFor = valueFor === v.name ? null : v.name)}>
        {v.set ? "Change value" : "Set value"}
      </Button>
      <Button onclick={() => startEdit(v)}>Edit</Button>
      {#if confirmingDelete === v.name}
        <span class="confirm-text">Delete <strong>{v.name}</strong>?</span>
        <Button onclick={() => (confirmingDelete = null)}>Cancel</Button>
        <Button variant="danger" onclick={() => remove(v)}>Yes, delete</Button>
      {:else}
        <Button variant="danger" onclick={() => (confirmingDelete = v.name)}>Delete</Button>
      {/if}
    </div>

    {#if editing === v.name}
      <div class="edit">
        <TextInput
          bind:value={editName}
          placeholder="NAME"
          aria-label="Name of {v.name}"
          error={Boolean(errors[v.name])}
          onkeydown={(e: KeyboardEvent) => onEditKeydown(e, v)}
        />
        <TextInput
          bind:value={editDescription}
          placeholder="Description"
          aria-label="Description of {v.name}"
          onkeydown={(e: KeyboardEvent) => onEditKeydown(e, v)}
        />
        <div class="actions">
          <Button variant="primary" onclick={() => save(v)}>Save</Button>
          <Button onclick={() => cancelEdit(v)}>Cancel</Button>
          <span class="spacer"></span>
          <Button onclick={() => move(v)}>
            {v.secret ? "Move to Variables" : "Move to Secrets"}
          </Button>
        </div>
      </div>
    {/if}

    {#if v.description}<p class="description">{v.description}</p>{/if}

    <p class="uses">
      {v.used_in.length > 0 ? `Used in: ${v.used_in.map((u) => u.label).join(", ")}` : "not used"}
    </p>

    {#if errors[v.name]}<p class="error">{errors[v.name]}</p>{/if}

    {#if valueFor === v.name}
      <SecretField name={v.name} exists={v.set} masked={v.secret} onclose={closeValue} />
    {/if}
  </div>
{/snippet}

{#snippet card(kind: Kind, title: string, items: EnvVarStatus[], emptyText: string)}
  <section class="card">
    <h3>{title}</h3>
    {#if loaded && items.length === 0}
      <p class="muted">{emptyText}</p>
    {/if}
    {#each items as v (v.name)}
      {@render row(v)}
    {/each}
    <div class="add">
      <TextInput
        bind:value={adds[kind].name}
        placeholder="NAME"
        aria-label="{title} name"
        error={Boolean(adds[kind].error)}
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && add(kind)}
      />
      <TextInput
        bind:value={adds[kind].description}
        placeholder="Description"
        aria-label="{title} description"
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && add(kind)}
      />
      <Button variant="primary" onclick={() => add(kind)}>Add</Button>
    </div>
    {#if adds[kind].error}<p class="error">{adds[kind].error}</p>{/if}
  </section>
{/snippet}

<SettingsSection
  title="Environment"
  hint="Pick these in models and MCP servers. Names and descriptions sync; values stay on this machine."
>
  {#snippet body()}
    {@render card("secrets", "Secrets", groups.secrets, "No secrets.")}
    {@render card("variables", "Variables", groups.variables, "No variables.")}
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
  .actions,
  .add {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .edit {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

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

  .description,
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
