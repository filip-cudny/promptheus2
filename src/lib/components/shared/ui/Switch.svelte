<script lang="ts">
  let {
    checked,
    label,
    onchange,
    disabled = false,
  }: {
    checked: boolean;
    label: string;
    onchange: (next: boolean) => void;
    disabled?: boolean;
  } = $props();
</script>

<button
  type="button"
  role="switch"
  class="switch"
  class:on={checked}
  aria-checked={checked}
  aria-label={label}
  title={label}
  {disabled}
  onclick={() => onchange(!checked)}
></button>

<style>
  .switch {
    position: relative;
    flex-shrink: 0;
    width: 26px;
    height: 15px;
    padding: 0;
    border: none;
    border-radius: var(--radius-xl);
    background: var(--text-faint);
    cursor: pointer;
  }

  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: #fff;
  }

  .switch.on {
    background: var(--accent);
  }

  .switch.on::after {
    left: 13px;
  }

  .switch:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
