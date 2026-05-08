<script lang="ts">
  import { onMount } from "svelte";
  import { X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { validateSkillSlug } from "$lib/services/skills";

  let {
    existingSlugs,
    templates,
    onClose,
    onSubmit,
  }: {
    existingSlugs: Set<string>;
    templates: { slug: string; label: string }[];
    onClose: () => void;
    onSubmit: (payload: {
      slug: string;
      displayName: string;
      description: string | null;
      template: string | null;
    }) => Promise<void>;
  } = $props();

  let slug = $state("");
  let displayName = $state("");
  let description = $state("");
  let template = $state<string>("");
  let slugError = $state<string | null>(null);
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  let slugInputEl = $state<HTMLInputElement | undefined>(undefined);

  onMount(() => {
    slugInputEl?.focus();
  });

  let validationToken = 0;

  $effect(() => {
    const value = slug.trim();
    validationToken += 1;
    const ticket = validationToken;
    if (!value) {
      slugError = null;
      return;
    }
    if (existingSlugs.has(value)) {
      slugError = `Skill '${value}' already exists`;
      return;
    }
    void validateSkillSlug(value).then((res) => {
      if (ticket !== validationToken) return;
      slugError = res.ok ? null : res.error;
    });
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!slug.trim() || slugError) return;
    submitting = true;
    submitError = null;
    try {
      await onSubmit({
        slug: slug.trim(),
        displayName: displayName.trim(),
        description: description.trim() ? description.trim() : null,
        template: template ? template : null,
      });
      onClose();
    } catch (e) {
      submitError = e instanceof Error ? e.message : String(e);
    } finally {
      submitting = false;
    }
  }

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains("backdrop")) {
      onClose();
    }
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="backdrop" onmousedown={handleBackdrop} role="presentation">
  <form class="dialog" onsubmit={handleSubmit}>
    <header>
      <h2>Create skill</h2>
      <button type="button" class="close" onclick={onClose} aria-label="Close">
        <X size={ICON_SIZE.md} />
      </button>
    </header>

    <div class="field">
      <label for="csd-slug">Slug</label>
      <div class="slug-input">
        <span class="prefix">/</span>
        <input
          id="csd-slug"
          type="text"
          bind:value={slug}
          bind:this={slugInputEl}
          placeholder="translate-english"
          spellcheck="false"
          autocomplete="off"
          class:error={slugError}
        />
      </div>
      {#if slugError}
        <span class="hint err">{slugError}</span>
      {:else}
        <span class="hint">Lowercase letters, digits, dashes. 2–48 characters.</span>
      {/if}
    </div>

    <div class="field">
      <label for="csd-name">Display name</label>
      <input
        id="csd-name"
        type="text"
        placeholder="Optional"
        bind:value={displayName}
      />
    </div>

    <div class="field">
      <label for="csd-desc">Description</label>
      <input
        id="csd-desc"
        type="text"
        placeholder="Optional"
        bind:value={description}
      />
    </div>

    <div class="field">
      <label for="csd-template">Start from template</label>
      <select id="csd-template" bind:value={template}>
        <option value="">Empty body</option>
        {#each templates as t (t.slug)}
          <option value={t.slug}>{t.label}</option>
        {/each}
      </select>
    </div>

    {#if submitError}
      <p class="submit-error">{submitError}</p>
    {/if}

    <footer>
      <button type="button" class="cancel" onclick={onClose}>Cancel</button>
      <button
        type="submit"
        class="primary"
        disabled={!slug.trim() || !!slugError || submitting}
      >
        {submitting ? "Creating…" : "Create"}
      </button>
    </footer>
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal, 1000);
  }

  .dialog {
    width: 480px;
    max-width: calc(100vw - var(--space-8));
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-6) var(--space-7) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    box-shadow: var(--shadow-lg);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  header h2 {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: var(--space-1);
    border-radius: var(--radius-sm);
  }

  .close:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  input[type="text"],
  select {
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-md);
  }

  .slug-input {
    display: flex;
    align-items: stretch;
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
  }

  .slug-input .prefix {
    padding: var(--space-3) var(--space-2) var(--space-3) var(--space-4);
    font-family: var(--font-mono);
    color: var(--text-faint);
    font-size: var(--font-size-md);
  }

  .slug-input input {
    flex: 1;
    background: transparent;
    border: none;
    padding-left: 0;
    font-family: var(--font-mono);
  }

  input.error {
    color: var(--danger);
  }

  .hint {
    font-size: var(--font-size-xs);
    color: var(--text-faint);
  }

  .hint.err {
    color: var(--danger);
  }

  .submit-error {
    font-size: var(--font-size-sm);
    color: var(--danger);
    margin: 0;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    padding-top: var(--space-2);
  }

  .cancel,
  .primary {
    padding: var(--space-2) var(--space-5);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-hard-2);
    background: var(--surface-elevated);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .primary {
    background: var(--accent);
    color: var(--text-inverted, #0a0a0a);
    border-color: var(--accent);
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
