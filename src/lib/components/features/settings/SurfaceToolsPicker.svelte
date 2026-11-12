<script lang="ts">
  import { onMount } from "svelte";
  import { error as logError } from "@tauri-apps/plugin-log";
  import { setSurfaceTool } from "$lib/services/settings";
  import { listMcpTools } from "$lib/services/mcp";
  import { onSettingsChanged } from "$lib/services/events";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { agentModelHasBuiltinSearch } from "$lib/utils/agentModel";

  interface Props {
    surface: "chat" | "settings_agent";
    enabled: string[];
  }

  let { surface, enabled }: Props = $props();

  const store = getSettingsStore();

  let tools = $state<string[]>([]);

  const chatModelIsOpenAi = $derived.by(() => {
    const id = store.settings?.surfaces.chat.generation.model_id;
    return store.models.find((m) => m.id === id)?.provider === "openai";
  });

  const showBuiltinSearch = $derived(
    store.settings !== null &&
      (surface === "chat"
        ? chatModelIsOpenAi
        : agentModelHasBuiltinSearch(store.settings.surfaces, store.models)),
  );

  function fetchTools() {
    listMcpTools()
      .then(
        (list) =>
          (tools = list.filter((t) => !t.disabled).map((t) => `${t.server}.${t.name}`)),
      )
      .catch((e) => logError(`list_mcp_tools failed: ${e}`));
  }

  onMount(() => {
    fetchTools();
    const unlisten = onSettingsChanged(fetchTools);
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  async function toggle(tool: string, on: boolean) {
    try {
      await setSurfaceTool(surface, tool, on);
    } catch (e) {
      logError(`set_surface_tool ${surface} failed: ${e}`);
    }
  }
</script>

<div class="tools">
  {#if showBuiltinSearch}
    <label class="check">
      <input
        type="checkbox"
        checked={enabled.includes("web_search")}
        onchange={(e) => toggle("web_search", e.currentTarget.checked)}
      />
      <span>Web search (built-in)</span>
    </label>
  {/if}
  {#each tools as tool (tool)}
    <label class="check">
      <input
        type="checkbox"
        checked={enabled.includes(tool)}
        onchange={(e) => toggle(tool, e.currentTarget.checked)}
      />
      <span>{tool}</span>
    </label>
  {/each}
  {#if tools.length === 0}
    <p class="muted">No MCP tools available.</p>
  {/if}
</div>

<style>
  .tools {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    cursor: pointer;
  }

  .check input {
    cursor: pointer;
  }

  .muted {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
</style>
