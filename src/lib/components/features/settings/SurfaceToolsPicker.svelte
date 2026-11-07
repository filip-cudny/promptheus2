<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { error as logError } from "@tauri-apps/plugin-log";
  import { updateSurfaceEnabledTools } from "$lib/services/settings";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { agentModelHasBuiltinSearch } from "$lib/utils/agentModel";

  interface Props {
    surface: "chat" | "settings_agent";
    enabled: string[];
  }

  let { surface, enabled }: Props = $props();

  const store = getSettingsStore();

  let tools = $state<string[]>([]);

  const showBuiltinSearch = $derived(
    surface === "settings_agent" &&
      store.settings !== null &&
      agentModelHasBuiltinSearch(store.settings.surfaces, store.models),
  );

  onMount(() => {
    invoke<{ name: string; server: string }[]>("list_mcp_tools")
      .then((list) => (tools = list.map((t) => `${t.server}.${t.name}`)))
      .catch((e) => logError(`list_mcp_tools failed: ${e}`));
  });

  async function toggle(tool: string, on: boolean) {
    const next = on ? [...enabled, tool] : enabled.filter((t) => t !== tool);
    try {
      await updateSurfaceEnabledTools(surface, next);
    } catch (e) {
      logError(`update_surface_enabled_tools ${surface} failed: ${e}`);
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
