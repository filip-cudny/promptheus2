<script lang="ts" module>
  const ENV_REF_RE = /^\$\{([A-Z_][A-Z0-9_]*)\}$/;

  export function parseEnvRef(value: string | null | undefined): string | null {
    if (!value) return null;
    const m = value.match(ENV_REF_RE);
    return m ? m[1] : null;
  }
</script>

<script lang="ts">
  import { Check, AlertCircle } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    varName,
    resolved,
  }: {
    varName: string;
    resolved: boolean;
  } = $props();
</script>

<span class="env-ref" class:ok={resolved} class:missing={!resolved} title={resolved ? "Environment variable is set" : "Environment variable is not set"}>
  {#if resolved}
    <Check size={ICON_SIZE.sm} />
  {:else}
    <AlertCircle size={ICON_SIZE.sm} />
  {/if}
  <span class="label">Env ref: <code>{varName}</code></span>
</span>

<style>
  .env-ref {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard);
  }

  .env-ref.ok {
    color: var(--success);
    border-color: var(--success-border);
  }

  .env-ref.missing {
    color: var(--danger);
    border-color: var(--danger-border);
  }

  code {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
  }
</style>
