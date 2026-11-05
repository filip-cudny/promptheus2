<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { error as logError } from "@tauri-apps/plugin-log";
  import SelectDropdown from "$lib/components/shared/ui/SelectDropdown.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { getCachedCapabilities, prefetchCapabilities } from "$lib/stores/capabilities.svelte";
  import {
    updateSurfaceEnabledTools,
    updateSurfaceModel,
    updateSurfaceReasoningEffort,
  } from "$lib/services/settings";
  import { effortOptions } from "$lib/utils/effortOptions";
  import SettingsSection from "./SettingsSection.svelte";
  import SettingField from "./SettingField.svelte";
  import PromptEditor from "./PromptEditor.svelte";
  import WebviewProvidersEditor from "./WebviewProvidersEditor.svelte";

  const store = getSettingsStore();
  const chat = $derived(store.settings?.surfaces.chat.generation ?? null);
  const titleModelId = $derived(
    store.settings?.surfaces.title_generation.generation.model_id ?? null,
  );

  const textModels = $derived(
    store.models
      .filter((m) => m.type === "text")
      .map((m) => ({ value: m.id, label: m.display_name || m.model })),
  );
  const chatModelOptions = $derived([{ value: "", label: "Not set" }, ...textModels]);
  const titleModelOptions = $derived([{ value: "", label: "Chat model" }, ...textModels]);

  const chatEffortOptions = $derived(
    effortOptions(
      getCachedCapabilities(chat?.model_id ?? null)?.reasoning,
      chat?.parameters.reasoning_effort,
    ),
  );

  let tools = $state<string[]>([]);

  $effect(() => prefetchCapabilities(chat?.model_id ?? null));

  onMount(() => {
    invoke<{ name: string; server: string }[]>("list_mcp_tools")
      .then((list) => (tools = list.map((t) => `${t.server}.${t.name}`)))
      .catch((e) => logError(`list_mcp_tools failed: ${e}`));
  });

  async function run(label: string, write: () => Promise<void>) {
    try {
      await write();
    } catch (e) {
      logError(`${label} failed: ${e}`);
    }
  }

  function toggleTool(tool: string, on: boolean) {
    if (!chat) return;
    const next = on
      ? [...chat.enabled_tools, tool]
      : chat.enabled_tools.filter((t) => t !== tool);
    void run("update_surface_enabled_tools", () => updateSurfaceEnabledTools("chat", next));
  }
</script>

<SettingsSection title="Chat">
  {#snippet body()}
    {#if chat}
      <div class="group">
        <SettingField key="surfaces.chat.generation.model_id">
          <SelectDropdown
            value={chat.model_id ?? ""}
            options={chatModelOptions}
            ariaLabel="Chat model"
            onchange={(id) =>
              run("update_surface_model chat", () => updateSurfaceModel("chat", id || null))}
          />
        </SettingField>

        <SettingField key="surfaces.chat.generation.parameters.reasoning_effort">
          <SelectDropdown
            value={chat.parameters.reasoning_effort ?? ""}
            options={chatEffortOptions}
            ariaLabel="Reasoning effort"
            onchange={(v) =>
              run("update reasoning_effort chat", () =>
                updateSurfaceReasoningEffort("chat", v || null),
              )}
          />
        </SettingField>

        <SettingField key="surfaces.chat.generation.enabled_tools">
          {#if tools.length === 0}
            <p class="muted">No MCP tools available.</p>
          {:else}
            <div class="tools">
              {#each tools as tool (tool)}
                <label class="check">
                  <input
                    type="checkbox"
                    checked={chat.enabled_tools.includes(tool)}
                    onchange={(e) => toggleTool(tool, e.currentTarget.checked)}
                  />
                  <span>{tool}</span>
                </label>
              {/each}
            </div>
          {/if}
        </SettingField>

        <SettingField key="surfaces.title_generation.generation.model_id">
          <SelectDropdown
            value={titleModelId ?? ""}
            options={titleModelOptions}
            ariaLabel="Title model"
            onchange={(id) =>
              run("update_surface_model title_generation", () =>
                updateSurfaceModel("title_generation", id || null),
              )}
          />
        </SettingField>

        <div class="prompt">
          <PromptEditor
            kind="title_generation"
            description="Used to auto-name conversations after the first user message."
          />
        </div>

        <WebviewProvidersEditor />
      </div>
    {/if}
  {/snippet}
</SettingsSection>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 760px;
  }

  .prompt {
    display: flex;
    flex-direction: column;
    height: 280px;
  }

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
