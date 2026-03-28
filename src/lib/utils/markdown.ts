import { Marked, type RendererObject } from "marked";
import hljs from "highlight.js";

function createRenderer(enableHighlighting: boolean): RendererObject {
  let blockIndex = 0;

  return {
    code({ text, lang }) {
      const idx = blockIndex++;
      let codeHtml: string;

      if (enableHighlighting && lang && hljs.getLanguage(lang)) {
        codeHtml = hljs.highlight(text, { language: lang }).value;
      } else if (enableHighlighting) {
        codeHtml = hljs.highlightAuto(text).value;
      } else {
        codeHtml = escapeHtml(text);
      }

      return `<div class="code-block"><div class="code-block-header"><span class="code-lang">${lang ?? ""}</span><button class="copy-btn" data-copy-index="${idx}">Copy</button></div><pre><code class="hljs">${codeHtml}</code></pre></div>`;
    },
  };
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function renderMarkdown(
  text: string,
  enableHighlighting = true,
): string {
  const instance = new Marked();
  instance.use({ renderer: createRenderer(enableHighlighting) });
  return instance.parse(text, { async: false }) as string;
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
