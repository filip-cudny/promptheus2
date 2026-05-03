<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    disabled = false,
    oninput,
    onchange,
    ...rest
  }: {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    oninput?: (e: Event) => void;
    onchange?: (e: Event) => void;
    [key: string]: unknown;
  } = $props();
</script>

<div class="text-field">
  <input
    class="text-field-input"
    type="text"
    bind:value
    {placeholder}
    {disabled}
    {oninput}
    {onchange}
    {...rest}
  />
</div>

<style>
  .text-field {
    display: flex;
    align-items: center;
    background: var(--surface-elevated);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    transition: border-color var(--motion-fast) var(--ease-default);
  }

  .text-field:focus-within {
    border-color: var(--accent-border);
  }

  .text-field-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-sans);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    width: 100%;
  }

  .text-field-input::placeholder {
    color: var(--text-muted);
  }

  .text-field-input:disabled {
    opacity: var(--opacity-disabled);
  }
</style>
