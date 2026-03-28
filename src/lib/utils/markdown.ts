import { Marked, type RendererObject } from "marked";
import hljs from "highlight.js";

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

      return `<div class="code-block"><div class="code-block-header"><span class="code-lang">${lang ?? ""}</span><button class="copy-btn" data-copy-index="${idx}">Copy</button></div><pre><code class="hljs">${codeHtml}</code></pre></div>`;
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
