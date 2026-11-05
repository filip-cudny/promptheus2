<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { CircleAlert } from "lucide-svelte";
  import { onSettingsChanged } from "$lib/services/events";
  import { getMissingEnvRefs } from "$lib/services/settings";
  import type { MissingEnvRef } from "$lib/types";

  let missing = $state<MissingEnvRef[]>([]);
  let unlisten: (() => void) | null = null;

  async function refresh() {
    try {
      missing = await getMissingEnvRefs();
    } catch {
      missing = [];
    }
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

{#if missing.length > 0}
  <div class="missing-env" role="alert">
    <CircleAlert size={16} />
    <div class="body">
      <p class="hint">
        These variables have no value on this machine. Add them to <code>.env</code> in the config
        directory and restart the app.
      </p>
      <ul>
        {#each missing as ref (ref.name)}
          <li>
            <code>{ref.name}</code>
            <span class="uses">{ref.used_in.map((u) => u.label).join(", ")}</span>
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style>
  .missing-env {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-6);
    background: var(--surface-raised);
    border-bottom: 1px solid var(--border-default);
    color: var(--warning);
    font-size: var(--font-size-base);
  }

  .body {
    color: var(--text-primary);
  }

  .hint {
    margin: 0 0 var(--space-2);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .uses {
    color: var(--text-secondary);
  }
</style>
