<script lang="ts">
  let {
    value = $bindable(0),
    min,
    max,
    step = 1,
    disabled = false,
    oninput,
    ...rest
  }: {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    oninput?: (e: Event) => void;
    [key: string]: unknown;
  } = $props();
</script>

<input
  class="number-input"
  type="number"
  bind:value
  {min}
  {max}
  {step}
  {disabled}
  {oninput}
  {...rest}
/>

<style>
  .number-input {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--surface-sunken);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    font-family: var(--font-sans);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    outline: none;
    transition: border-color var(--motion-fast) var(--ease-default);
  }
  .number-input:focus {
    border-color: var(--accent-border);
    box-shadow: 0 0 0 2px var(--accent-ring);
  }
  .number-input:disabled {
    opacity: var(--opacity-disabled);
  }
</style>
