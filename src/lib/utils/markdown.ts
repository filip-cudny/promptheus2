import { Marked, type RendererObject } from "marked";
import hljs from "highlight.js";
import { ICON_SIZE } from "$lib/constants/ui";

let blockIndex = 0;

function createRenderer(): RendererObject {
  return {
    code({ text, lang }) {
      const idx = blockIndex++;
      let codeHtml: string;

      if (lang && hljs.getLanguage(lang)) {
        codeHtml = hljs.highlight(text, { language: lang }).value;
      } else {
        codeHtml = hljs.highlightAuto(text).value;
      }

      const s = ICON_SIZE.md;
      const copyIcon = `<svg xmlns="http://www.w3.org/2000/svg" width="${s}" height="${s}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`;
      return `<div class="code-block"><div class="code-block-header"><span class="code-lang">${lang ?? ""}</span><button class="copy-btn" data-copy-index="${idx}">${copyIcon}</button></div><pre><code class="hljs">${codeHtml}</code></pre></div>`;
    },
  };
}

const markedInstance = new Marked();
markedInstance.use({ renderer: createRenderer() });

export function renderMarkdown(text: string): string {
  blockIndex = 0;
  return markedInstance.parse(text, { async: false }) as string;
}

const CODE_BLOCK_RE = /^```[^\n]*\n([\s\S]*?)^```/gm;

export function extractCodeBlocks(text: string): string[] {
  const blocks: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = CODE_BLOCK_RE.exec(text)) !== null) {
    blocks.push(match[1].replace(/\n$/, ""));
  }
  CODE_BLOCK_RE.lastIndex = 0;
  return blocks;
}
