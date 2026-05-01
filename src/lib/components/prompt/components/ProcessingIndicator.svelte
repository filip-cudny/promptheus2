<script lang="ts">
  let {
    label = "Generating",
    inline = false,
  }: {
    label?: string;
    inline?: boolean;
  } = $props();
</script>

{#if inline}
  <span class="processing-label">{label}</span>
{:else}
  <div class="processing-indicator" role="status" aria-live="polite">
    <span class="processing-label">{label}</span>
  </div>
{/if}

<style>
  .processing-indicator {
    display: flex;
    align-items: center;
    padding: var(--space-3) var(--space-0);
  }

  .processing-label {
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    background: linear-gradient(
      90deg,
      var(--accent-ring) 0%,
      var(--accent) 50%,
      var(--accent-ring) 100%
    );
    background-size: 200% auto;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: shimmer 2s linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .processing-label {
      animation: none;
      background: none;
      -webkit-text-fill-color: var(--accent);
    }
  }
</style>
