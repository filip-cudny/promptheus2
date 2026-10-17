import { onMount } from "svelte";

export function useHistoryKeyboard(opts: {
  searchInput: () => HTMLInputElement | null;
  entriesList: () => HTMLDivElement | null;
  hasResults: () => boolean;
  stepPage: (delta: 1 | -1) => boolean;
}) {
  function getRowButtons(): HTMLButtonElement[] {
    const list = opts.entriesList();
    if (!list) return [];
    return Array.from(list.querySelectorAll<HTMLButtonElement>(".entry-row"));
  }

  function focusedRowIndex(): number {
    return getRowButtons().findIndex((row) => row === document.activeElement);
  }

  function focusRow(idx: number) {
    const rows = getRowButtons();
    if (rows.length === 0) return;
    const clamped = Math.max(0, Math.min(rows.length - 1, idx));
    rows[clamped]?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    const searchInput = opts.searchInput();

    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
      return;
    }

    if (e.key === "PageDown" || e.key === "PageUp") {
      if (opts.stepPage(e.key === "PageDown" ? 1 : -1)) e.preventDefault();
      return;
    }

    const inSearch = !!searchInput && e.target === searchInput;
    const rowIdx = focusedRowIndex();

    if (inSearch && e.key === "ArrowDown") {
      if (opts.hasResults()) {
        e.preventDefault();
        focusRow(0);
      }
      return;
    }

    if (rowIdx < 0) return;

    if (e.key === "ArrowDown" || e.key === "j") {
      e.preventDefault();
      focusRow(rowIdx + 1);
    } else if (e.key === "ArrowUp" || e.key === "k") {
      e.preventDefault();
      if (rowIdx === 0) searchInput?.focus();
      else focusRow(rowIdx - 1);
    } else if (e.key === "Escape") {
      e.preventDefault();
      searchInput?.focus();
    } else if (e.key === "Home") {
      e.preventDefault();
      focusRow(0);
    } else if (e.key === "End") {
      e.preventDefault();
      focusRow(getRowButtons().length - 1);
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onkeydown);
    return () => window.removeEventListener("keydown", onkeydown);
  });
}
