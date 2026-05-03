<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    type = "text",
    disabled = false,
    error = false,
    oninput,
    ...rest
  }: {
    value?: string;
    placeholder?: string;
    type?: "text" | "password" | "url" | "email";
    disabled?: boolean;
    error?: boolean;
    oninput?: (e: Event) => void;
    [key: string]: unknown;
  } = $props();
</script>

<input
  class="text-input"
  class:error
  {type}
  bind:value
  {placeholder}
  {disabled}
  {oninput}
  {...rest}
/>

<style>
  .text-input {
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
  .text-input:focus {
    border-color: var(--accent-border);
    box-shadow: 0 0 0 2px var(--accent-ring);
  }
  .text-input.error {
    border-color: var(--danger-border);
  }
  .text-input::placeholder {
    color: var(--text-muted);
  }
  .text-input:disabled {
    opacity: var(--opacity-disabled);
  }
</style>
