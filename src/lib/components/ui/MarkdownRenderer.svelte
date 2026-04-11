<script lang="ts">
  import { onDestroy } from "svelte";
  import morphdom from "morphdom";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { save } from "@tauri-apps/plugin-dialog";
  import { renderMarkdown, extractCodeBlocks } from "$lib/utils/markdown";
  import { icon, icons } from "$lib/utils/icons";

  let {
    content,
    isStreaming = false,
  }: {
    content: string;
    isStreaming: boolean;
  } = $props();

  const DRAIN_FRACTION = 0.15;
  const RENDER_EVERY_N_FRAMES = 2;

  let displayedLength = $state(isStreaming ? 0 : content.length);
  let animFrameId: number | null = null;
  let wasStreaming = isStreaming;
  let frameCount = 0;
  let lastRenderedHtml = "";

  let displayedText = $derived(
    isStreaming ? content : content.slice(0, displayedLength),
  );
  let isFullyRevealed = $derived(
    !isStreaming && displayedLength >= content.length,
  );

  let renderedHtml = $derived(renderMarkdown(displayedText));

  let codeBlocks = $derived(
    isFullyRevealed ? extractCodeBlocks(displayedText) : [],
  );

  let markdownContainer: HTMLDivElement | undefined = $state();
  const morphWrapper = document.createElement("div");

  let mermaidModule: typeof import("mermaid") | null = null;

  async function ensureMermaid() {
    if (mermaidModule) return mermaidModule;
    mermaidModule = await import("mermaid");
    mermaidModule.default.initialize({
      startOnLoad: false,
      theme: "dark",
      fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    });
    return mermaidModule;
  }

  async function renderMermaidBlocks() {
    if (!markdownContainer) return;
    const blocks = markdownContainer.querySelectorAll<HTMLElement>(".mermaid-block:not(.mermaid-rendered)");
    if (blocks.length === 0) return;

    const mod = await ensureMermaid();
    for (const block of blocks) {
      const encoded = block.dataset.mermaidSource;
      if (!encoded) continue;
      const source = decodeURIComponent(atob(encoded));
      const id = `mermaid-${block.dataset.mermaidIndex}-${Date.now()}`;
      try {
        const { svg } = await mod.default.render(id, source);
        block.innerHTML = svg;
        block.classList.add("mermaid-rendered");
      } catch {
        block.classList.add("mermaid-error");
      }
    }
  }

  $effect(() => {
    if (!markdownContainer) return;
    const html = renderedHtml;
    if (html === lastRenderedHtml) return;
    lastRenderedHtml = html;
    morphWrapper.innerHTML = html;
    morphdom(markdownContainer, morphWrapper, { childrenOnly: true });
    renderMermaidBlocks();
  });

  function animate() {
    const remaining = content.length - displayedLength;

    if (remaining <= 0) {
      animFrameId = null;
      return;
    }

    frameCount++;
    if (frameCount % RENDER_EVERY_N_FRAMES === 0) {
      displayedLength += Math.max(1, Math.ceil(remaining * DRAIN_FRACTION));
    }

    animFrameId = requestAnimationFrame(animate);
  }

  function startAnimation() {
    if (animFrameId !== null) return;
    animFrameId = requestAnimationFrame(animate);
  }

  $effect(() => {
    if (isStreaming) {
      wasStreaming = true;
      displayedLength = content.length;
    } else if (content.length > displayedLength) {
      if (wasStreaming) {
        startAnimation();
      } else {
        displayedLength = content.length;
      }
    }
  });

  onDestroy(() => {
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
  });

  function handleMermaidToggle(toggleBtn: HTMLElement) {
    const idx = toggleBtn.dataset.mermaidToggle;
    if (!idx || !markdownContainer) return;

    const block = markdownContainer.querySelector<HTMLElement>(`.mermaid-block[data-mermaid-index="${idx}"]`);
    if (!block) return;

    const codeIcon = icon(icons.Code);
    const eyeIcon = icon(icons.Eye);

    const isRaw = block.classList.contains("mermaid-raw");

    if (isRaw) {
      const savedSvg = block.dataset.mermaidSvg;
      if (savedSvg) {
        block.innerHTML = decodeURIComponent(atob(savedSvg));
        block.classList.remove("mermaid-raw");
        toggleBtn.innerHTML = codeIcon;
      }
    } else if (block.classList.contains("mermaid-rendered")) {
      block.dataset.mermaidSvg = btoa(encodeURIComponent(block.innerHTML));
      const encoded = block.dataset.mermaidSource;
      if (!encoded) return;
      const source = decodeURIComponent(atob(encoded));
      block.textContent = source;
      block.classList.add("mermaid-raw");
      toggleBtn.innerHTML = eyeIcon;
    }
  }

  function getMermaidSvg(idx: string): string | null {
    if (!markdownContainer) return null;
    const block = markdownContainer.querySelector<HTMLElement>(`.mermaid-block[data-mermaid-index="${idx}"]`);
    if (!block) return null;

    if (block.classList.contains("mermaid-raw")) {
      const saved = block.dataset.mermaidSvg;
      return saved ? decodeURIComponent(atob(saved)) : null;
    }
    if (block.classList.contains("mermaid-rendered")) {
      return block.innerHTML;
    }
    return null;
  }

  function closeAllMermaidMenus() {
    if (!markdownContainer) return;
    for (const panel of markdownContainer.querySelectorAll<HTMLElement>(".mermaid-menu.open")) {
      panel.classList.remove("open");
    }
  }

  async function saveSvgToFile(content: string) {
    const path = await save({
      defaultPath: "mermaid-diagram.svg",
      filters: [{ name: "SVG", extensions: ["svg"] }],
    });
    if (!path) return;
    await invoke("write_text_file", { path, content });
  }

  function handleClick(e: MouseEvent) {
    const target = e.target as HTMLElement;

    const anchor = target.closest("a") as HTMLAnchorElement | null;
    if (anchor?.href) {
      e.preventDefault();
      openUrl(anchor.href);
      return;
    }

    const toggleBtn = target.closest("[data-mermaid-toggle]") as HTMLElement | null;
    if (toggleBtn) {
      closeAllMermaidMenus();
      handleMermaidToggle(toggleBtn);
      return;
    }

    const menuBtn = target.closest("[data-mermaid-menu]") as HTMLElement | null;
    if (menuBtn && markdownContainer) {
      const idx = menuBtn.dataset.mermaidMenu;
      const panel = markdownContainer.querySelector<HTMLElement>(`.mermaid-menu[data-mermaid-menu-panel="${idx}"]`);
      if (panel) {
        const wasOpen = panel.classList.contains("open");
        closeAllMermaidMenus();
        if (!wasOpen) panel.classList.add("open");
      }
      return;
    }

    const copySvgBtn = target.closest("[data-mermaid-copy-svg]") as HTMLElement | null;
    if (copySvgBtn) {
      const svg = getMermaidSvg(copySvgBtn.dataset.mermaidCopySvg!);
      if (svg) navigator.clipboard.writeText(svg);
      closeAllMermaidMenus();
      return;
    }

    const saveSvgBtn = target.closest("[data-mermaid-save-svg]") as HTMLElement | null;
    if (saveSvgBtn) {
      const idx = saveSvgBtn.dataset.mermaidSaveSvg!;
      const svg = getMermaidSvg(idx);
      if (svg) {
        saveSvgToFile(svg);
      }
      closeAllMermaidMenus();
      return;
    }

    closeAllMermaidMenus();

    const copyBtn = target.closest("[data-copy-index]") as HTMLElement | null;
    if (!copyBtn) return;

    const index = parseInt(copyBtn.dataset.copyIndex ?? "", 10);
    if (Number.isNaN(index) || index >= codeBlocks.length) return;

    const checkIcon = icon(icons.Check);
    const originalHtml = copyBtn.innerHTML;
    navigator.clipboard.writeText(codeBlocks[index]);
    copyBtn.innerHTML = checkIcon;
    setTimeout(() => {
      copyBtn.innerHTML = originalHtml;
    }, 1200);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="markdown-renderer" bind:this={markdownContainer} onclick={handleClick}></div>

<style>
  .markdown-renderer {
    font-size: 14px;
    line-height: 1.6;
    color: #e0e0e0;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  .markdown-renderer :global(h1),
  .markdown-renderer :global(h2),
  .markdown-renderer :global(h3) {
    margin: 12px 0 6px;
    color: #f0f0f0;
  }

  .markdown-renderer :global(h1) {
    font-size: 1.4em;
  }

  .markdown-renderer :global(h2) {
    font-size: 1.2em;
  }

  .markdown-renderer :global(h3) {
    font-size: 1.05em;
  }

  .markdown-renderer :global(p) {
    margin: 6px 0;
  }

  .markdown-renderer :global(a) {
    color: #6ba3d6;
  }

  .markdown-renderer :global(ul),
  .markdown-renderer :global(ol) {
    padding-left: 20px;
    margin: 6px 0;
  }

  .markdown-renderer :global(blockquote) {
    border-left: 3px solid rgba(255, 255, 255, 0.2);
    padding-left: 12px;
    margin: 8px 0;
    color: rgba(255, 255, 255, 0.7);
  }

  .markdown-renderer :global(code) {
    background: rgba(255, 255, 255, 0.08);
    padding: 2px 5px;
    border-radius: 3px;
    font-size: 0.9em;
    font-family: "Fira Code", "Cascadia Code", monospace;
  }

  .markdown-renderer :global(.code-block) {
    margin: 8px 0;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .markdown-renderer :global(.code-block-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 12px;
    background: rgba(255, 255, 255, 0.06);
    font-size: 12px;
  }

  .markdown-renderer :global(.code-lang) {
    color: rgba(255, 255, 255, 0.5);
  }

  .markdown-renderer :global(.copy-btn) {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .markdown-renderer :global(.code-block-header:hover .copy-btn) {
    opacity: 1;
  }

  .markdown-renderer :global(.copy-btn:hover) {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .markdown-renderer :global(pre) {
    margin: 0;
    padding: 12px;
    background: rgba(0, 0, 0, 0.3);
    overflow-x: auto;
  }

  .markdown-renderer :global(pre code) {
    background: none;
    padding: 0;
    border-radius: 0;
    font-size: 13px;
  }

  .markdown-renderer :global(.mermaid-wrapper) {
    margin: 8px 0;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .markdown-renderer :global(.mermaid-actions) {
    display: flex;
    gap: 2px;
  }

  .markdown-renderer :global(.mermaid-block) {
    padding: 16px;
    background: rgba(0, 0, 0, 0.3);
    overflow-x: auto;
    display: flex;
    justify-content: center;
  }

  .markdown-renderer :global(.mermaid-block:not(.mermaid-rendered)),
  .markdown-renderer :global(.mermaid-block.mermaid-raw) {
    white-space: pre;
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.5);
    justify-content: flex-start;
  }

  .markdown-renderer :global(.mermaid-block.mermaid-error) {
    color: #e06c75;
  }

  .markdown-renderer :global(.mermaid-block svg) {
    max-width: 100%;
    height: auto;
  }

  .markdown-renderer :global(.mermaid-menu-anchor) {
    position: relative;
  }

  .markdown-renderer :global(.mermaid-menu) {
    display: none;
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    min-width: 140px;
    background: #2a2a2a;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 4px 0;
    z-index: 10;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .markdown-renderer :global(.mermaid-menu.open) {
    display: block;
  }

  .markdown-renderer :global(.mermaid-menu-item) {
    display: block;
    width: 100%;
    padding: 6px 12px;
    border: none;
    background: none;
    color: rgba(255, 255, 255, 0.8);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }

  .markdown-renderer :global(.mermaid-menu-item:hover) {
    background: rgba(255, 255, 255, 0.08);
  }
</style>
