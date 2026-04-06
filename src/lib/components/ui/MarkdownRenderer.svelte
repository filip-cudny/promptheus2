<script lang="ts">
  import { onDestroy } from "svelte";
  import morphdom from "morphdom";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { renderMarkdown, extractCodeBlocks } from "$lib/utils/markdown";
  import { ICON_SIZE } from "$lib/constants/ui";

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

  let displayedText = $derived(content.slice(0, displayedLength));
  let isFullyRevealed = $derived(
    !isStreaming && displayedLength >= content.length,
  );

  let renderedHtml = $derived(renderMarkdown(displayedText));

  let codeBlocks = $derived(
    isFullyRevealed ? extractCodeBlocks(displayedText) : [],
  );

  let markdownContainer: HTMLDivElement | undefined = $state();
  const morphWrapper = document.createElement("div");

  $effect(() => {
    if (!markdownContainer) return;
    const html = renderedHtml;
    if (html === lastRenderedHtml) return;
    lastRenderedHtml = html;
    morphWrapper.innerHTML = html;
    morphdom(markdownContainer, morphWrapper, { childrenOnly: true });
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

  function handleClick(e: MouseEvent) {
    const target = e.target as HTMLElement;

    const anchor = target.closest("a") as HTMLAnchorElement | null;
    if (anchor?.href) {
      e.preventDefault();
      openUrl(anchor.href);
      return;
    }

    const copyBtn = target.closest("[data-copy-index]") as HTMLElement | null;
    if (!copyBtn) return;

    const index = parseInt(copyBtn.dataset.copyIndex ?? "", 10);
    if (Number.isNaN(index) || index >= codeBlocks.length) return;

    const s = ICON_SIZE.md;
    const checkIcon = `<svg xmlns="http://www.w3.org/2000/svg" width="${s}" height="${s}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>`;
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
</style>
