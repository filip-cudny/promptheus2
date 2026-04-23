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
    gap: 4px;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
  }

  .env-ref.ok {
    color: #6dd49a;
    border-color: rgba(109, 212, 154, 0.3);
  }

  .env-ref.missing {
    color: #d97373;
    border-color: rgba(217, 115, 115, 0.3);
  }

  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
  }
</style>
