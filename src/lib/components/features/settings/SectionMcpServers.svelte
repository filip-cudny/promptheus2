<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import { Plus, X } from "lucide-svelte";
  import { error as logError } from "@tauri-apps/plugin-log";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import NumberInput from "$lib/components/shared/ui/NumberInput.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import SaveStatusIndicator from "$lib/components/shared/widgets/SaveStatusIndicator.svelte";
  import { useSaveTracker } from "$lib/stores/saveTracker.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { updateSetting } from "$lib/services/settings";
  import { listMcpServers, type McpServerConfig, type McpServerState } from "$lib/services/mcp";
  import SettingsSection from "./SettingsSection.svelte";
  import SettingField from "./SettingField.svelte";
  import EnvVarPicker from "./EnvVarPicker.svelte";
  import InsertVariableButton from "./InsertVariableButton.svelte";

  type Pair<V> = { key: number; name: string; value: V };
  type Row = {
    key: number;
    name: string;
    command: string;
    args: string;
    env: Pair<string>[];
    timeout: number;
    toolTimeouts: Pair<number>[];
  };

  const store = getSettingsStore();
  const navigation = getSettingsNavigation();
  const tracker = useSaveTracker({
    debounceMs: store.settings?.autosave_debounce_ms ?? 1000,
  });

  let nextKey = 0;
  const configured = untrack(
    () =>
      ((store.settings as unknown as { mcp_servers?: Record<string, McpServerConfig> })
        ?.mcp_servers ?? {}),
  );

  function toPairs<V>(map: Record<string, V>): Pair<V>[] {
    return Object.entries(map).map(([name, value]) => ({ key: nextKey++, name, value }));
  }

  let rows = $state<Row[]>(
    Object.entries(configured)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, c]) => ({
        key: nextKey++,
        name,
        command: c.command,
        args: c.args.join("\n"),
        env: toPairs(c.env),
        timeout: c.timeout_secs,
        toolTimeouts: toPairs(c.tool_timeouts),
      })),
  );
  let selectedKey = $state<number | null>(untrack(() => rows[0]?.key ?? null));
  let removedName = $state<string | null>(null);
  let confirmDelete = $state(false);
  let states = $state<McpServerState[]>([]);
  let commandRow = $state<HTMLElement | undefined>();
  let argsEl = $state<HTMLTextAreaElement | undefined>();

  const selected = $derived(rows.find((r) => r.key === selectedKey) ?? null);
  const stateOf = $derived(new Map(states.map((s) => [s.name, s])));
  const removed = $derived(states.filter((s) => !rows.some((r) => r.name === s.name)));
  const selectedRemoved = $derived(
    selected ? null : (removed.find((s) => s.name === removedName) ?? null),
  );
  const duplicateName = $derived(
    selected !== null && rows.some((r) => r !== selected && r.name === selected.name),
  );

  onMount(() => {
    listMcpServers()
      .then((s) => (states = s))
      .catch((e) => logError(`list_mcp_servers failed: ${e}`));
  });

  onDestroy(() => {
    void tracker.flush();
    tracker.destroy();
  });

  $effect(() => {
    const item = navigation.selectedItem;
    if (item?.list !== "mcp_servers") return;
    const row = untrack(() => rows.find((r) => r.name === item.id));
    if (row) select(row.key);
  });

  function chip(row: Row): string {
    const s = stateOf.get(row.name);
    return s ? (s.state === "running" ? "running" : "error") : "starts after restart";
  }

  function persist() {
    const names = rows.map((r) => r.name);
    if (new Set(names).size !== names.length) return Promise.resolve();
    const map: Record<string, McpServerConfig> = {};
    for (const r of rows) {
      map[r.name] = {
        command: r.command,
        args: r.args.split("\n").filter((a) => a !== ""),
        env: Object.fromEntries(r.env.filter((p) => p.name).map((p) => [p.name, p.value])),
        timeout_secs: r.timeout,
        tool_timeouts: Object.fromEntries(
          r.toolTimeouts.filter((p) => p.name).map((p) => [p.name, p.value]),
        ),
      };
    }
    return updateSetting("mcp_servers", map);
  }

  function edited() {
    tracker.scheduleSave(persist);
  }

  function select(key: number) {
    selectedKey = key;
    removedName = null;
    confirmDelete = false;
  }

  function add() {
    const row: Row = {
      key: nextKey++,
      name: "",
      command: "",
      args: "",
      env: [],
      timeout: 180,
      toolTimeouts: [],
    };
    rows.push(row);
    select(row.key);
  }

  function remove() {
    rows = rows.filter((r) => r.key !== selectedKey);
    selectedKey = rows[0]?.key ?? null;
    confirmDelete = false;
    void tracker.flush(persist);
  }

  function addPair<V>(list: Pair<V>[], value: V) {
    list.push({ key: nextKey++, name: "", value });
  }

  function removePair<V>(list: Pair<V>[], key: number) {
    list.splice(
      list.findIndex((p) => p.key === key),
      1,
    );
    edited();
  }

  const nameError = $derived(
    selected
      ? duplicateName
        ? "Another server already has this name."
        : store.fieldError(`mcp_servers.${selected.name}`)
      : undefined,
  );
</script>

<SettingsSection title="MCP Servers" hint="Changes apply after restart.">
  {#snippet body()}
    <div class="layout">
      <aside class="list">
        {#each rows as row (row.key)}
          <button class="item" class:active={row.key === selectedKey} onclick={() => select(row.key)}>
            <span class="name">{row.name || "(unnamed)"}</span>
            <span class="chip {chip(row).replace(/ /g, '-')}">{chip(row)}</span>
          </button>
        {/each}
        {#each removed as s (s.name)}
          <button
            class="item"
            class:active={selectedKey === null && removedName === s.name}
            onclick={() => {
              selectedKey = null;
              removedName = s.name;
            }}
          >
            <span class="name">{s.name}</span>
            <span class="chip removed">removed after restart</span>
          </button>
        {/each}
        <Button onclick={add}><Plus size={ICON_SIZE.sm} /> Add server</Button>
      </aside>

      <div class="editor">
        {#if selected}
          {@const state = stateOf.get(selected.name)}
          <header class="head">
            <h1>{selected.name || "(unnamed)"}</h1>
            <SaveStatusIndicator {tracker} />
          </header>

          {#if state?.state === "error"}
            <pre class="start-error">{state.error}</pre>
          {:else if state}
            <div class="tools">
              <h3>Tools</h3>
              {#if state.tools.length === 0}
                <p class="muted">No tools.</p>
              {:else}
                <ul>
                  {#each state.tools as tool (tool)}<li><code>{tool}</code></li>{/each}
                </ul>
              {/if}
            </div>
          {/if}

          <FormRow label="Name" error={nameError}>
            <TextInput
              value={selected.name}
              aria-label="Server name"
              oninput={(e: Event) => {
                selected.name = (e.target as HTMLInputElement).value;
                edited();
              }}
            />
          </FormRow>

          <SettingField key="mcp_servers.*.command">
            <div class="with-action" bind:this={commandRow}>
              <TextInput
                value={selected.command}
                aria-label="Command"
                oninput={(e: Event) => {
                  selected.command = (e.target as HTMLInputElement).value;
                  edited();
                }}
              />
              <InsertVariableButton
                ariaLabel="Insert variable into command"
                field={() => commandRow?.querySelector("input")}
                oninsert={(text) => {
                  selected.command = text;
                  edited();
                }}
              />
            </div>
          </SettingField>

          <SettingField key="mcp_servers.*.args">
            <div class="with-action top">
              <textarea
                bind:this={argsEl}
                rows="3"
                aria-label="Arguments, one per line"
                placeholder="One argument per line"
                value={selected.args}
                oninput={(e) => {
                  selected.args = e.currentTarget.value;
                  edited();
                }}
              ></textarea>
              <InsertVariableButton
                ariaLabel="Insert variable into arguments"
                field={() => argsEl}
                oninsert={(text) => {
                  selected.args = text;
                  edited();
                }}
              />
            </div>
          </SettingField>

          <SettingField key="mcp_servers.*.env">
            <div class="pairs">
              {#each selected.env as pair (pair.key)}
                <div class="pair env">
                  <TextInput
                    value={pair.name}
                    placeholder="NAME"
                    aria-label="Variable name"
                    oninput={(e: Event) => {
                      pair.name = (e.target as HTMLInputElement).value;
                      edited();
                    }}
                  />
                  <EnvVarPicker
                    allowPlain
                    value={pair.value}
                    onchange={(next) => {
                      pair.value = next;
                      edited();
                    }}
                  />
                  <button
                    type="button"
                    class="icon-btn"
                    aria-label="Remove variable"
                    onclick={() => removePair(selected.env, pair.key)}
                  >
                    <X size={ICON_SIZE.sm} />
                  </button>
                </div>
              {/each}
              <div>
                <Button onclick={() => addPair(selected.env, "")}>
                  <Plus size={ICON_SIZE.sm} /> Add variable
                </Button>
              </div>
            </div>
          </SettingField>

          <SettingField key="mcp_servers.*.timeout_secs">
            <NumberInput
              min={1}
              step={1}
              value={selected.timeout}
              oninput={(e: Event) => {
                const n = Math.floor(Number((e.currentTarget as HTMLInputElement).value));
                if (!(n >= 1)) return;
                selected.timeout = n;
                edited();
              }}
            />
          </SettingField>

          <SettingField key="mcp_servers.*.tool_timeouts">
            <div class="pairs">
              {#each selected.toolTimeouts as pair (pair.key)}
                <div class="pair">
                  <TextInput
                    value={pair.name}
                    placeholder="tool"
                    aria-label="Tool name"
                    oninput={(e: Event) => {
                      pair.name = (e.target as HTMLInputElement).value;
                      edited();
                    }}
                  />
                  <NumberInput
                    min={1}
                    step={1}
                    value={pair.value}
                    aria-label="Timeout seconds"
                    oninput={(e: Event) => {
                      const n = Math.floor(Number((e.currentTarget as HTMLInputElement).value));
                      if (!(n >= 1)) return;
                      pair.value = n;
                      edited();
                    }}
                  />
                  <button
                    type="button"
                    class="icon-btn"
                    aria-label="Remove tool timeout"
                    onclick={() => removePair(selected.toolTimeouts, pair.key)}
                  >
                    <X size={ICON_SIZE.sm} />
                  </button>
                </div>
              {/each}
              <div>
                <Button onclick={() => addPair(selected.toolTimeouts, 180)}>
                  <Plus size={ICON_SIZE.sm} /> Add tool timeout
                </Button>
              </div>
            </div>
          </SettingField>

          <section class="danger">
            {#if !confirmDelete}
              <Button variant="danger" onclick={() => (confirmDelete = true)}>Delete server</Button>
            {:else}
              <p>Delete <strong>{selected.name || "this server"}</strong>?</p>
              <div class="confirm">
                <Button onclick={() => (confirmDelete = false)}>Cancel</Button>
                <Button variant="danger" onclick={remove}>Yes, delete</Button>
              </div>
            {/if}
          </section>
        {:else if selectedRemoved}
          <p class="muted">
            <code>{selectedRemoved.name}</code> is no longer configured and stops after restart.
          </p>
        {:else}
          <p class="muted">Add a server or pick one to edit.</p>
        {/if}
      </div>
    </div>
  {/snippet}
</SettingsSection>

<style>
  .layout {
    display: flex;
    gap: var(--space-8);
    align-items: flex-start;
  }

  .list {
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-4);
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
    text-align: left;
  }

  .item.active {
    background: var(--surface-overlay-faint);
    border-color: var(--accent);
  }

  .chip {
    font-size: var(--font-size-xs);
    color: var(--text-muted);
  }

  .chip.running {
    color: var(--success);
  }

  .chip.error {
    color: var(--danger);
  }

  .editor {
    flex: 1;
    min-width: 0;
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-5);
  }

  .head h1 {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin: var(--space-0);
    line-height: 1.2;
  }

  .start-error {
    margin: 0;
    padding: var(--space-4);
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--surface-sunken);
    border: 1px solid var(--danger-border);
    border-radius: var(--radius-md);
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .tools h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .tools ul {
    margin: 0;
    padding-left: var(--space-8);
  }

  .muted {
    color: var(--text-muted);
    font-size: var(--font-size-md);
  }

  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    padding: var(--space-3);
    background: var(--surface-sunken);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-md);
  }

  .with-action {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }

  .with-action.top {
    align-items: flex-start;
  }

  .pairs {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .pair {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }

  .pair.env > :global(.text-input) {
    flex: 0 0 38%;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .danger {
    border: 1px solid var(--danger-border);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
  }

  .danger p {
    margin: 0 0 var(--space-4);
  }

  .confirm {
    display: flex;
    gap: var(--space-4);
  }
</style>
