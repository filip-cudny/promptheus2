import { Marked, type RendererObject } from "marked";
import hljs from "highlight.js";
import { icon, icons } from "$lib/utils/icons";

let blockIndex = 0;

const LANG_DISPLAY_NAMES: Record<string, string> = {
  ts: "TypeScript",
  tsx: "TypeScript (JSX)",
  js: "JavaScript",
  jsx: "JavaScript (JSX)",
  py: "Python",
  rb: "Ruby",
  rs: "Rust",
  go: "Go",
  java: "Java",
  kt: "Kotlin",
  cs: "C#",
  cpp: "C++",
  c: "C",
  sh: "Shell",
  bash: "Bash",
  zsh: "Zsh",
  ps1: "PowerShell",
  sql: "SQL",
  html: "HTML",
  css: "CSS",
  scss: "SCSS",
  json: "JSON",
  yaml: "YAML",
  yml: "YAML",
  toml: "TOML",
  xml: "XML",
  md: "Markdown",
  dockerfile: "Dockerfile",
  swift: "Swift",
  php: "PHP",
};

function langDisplayName(lang: string | undefined): string {
  if (!lang) return "";
  return LANG_DISPLAY_NAMES[lang.toLowerCase()] ?? lang;
}

function createRenderer(): RendererObject {
  return {
    code({ text, lang }) {
      const idx = blockIndex++;

      if (lang?.toLowerCase() === "mermaid") {
        const escaped = text
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;");
        const codeIcon = icon(icons.Code);
        const copyIcon = icon(icons.Copy);
        const dotsIcon = icon(icons.EllipsisVertical);
        return `<div class="mermaid-wrapper"><div class="code-block-header"><span class="code-lang">Mermaid</span><div class="mermaid-actions"><button class="copy-btn" data-mermaid-toggle="${idx}">${codeIcon}</button><button class="copy-btn" data-copy-index="${idx}">${copyIcon}</button><div class="mermaid-menu-anchor"><button class="copy-btn" data-mermaid-menu="${idx}">${dotsIcon}</button><div class="mermaid-menu" data-mermaid-menu-panel="${idx}"><button class="mermaid-menu-item" data-mermaid-copy-svg="${idx}">Copy as SVG</button><button class="mermaid-menu-item" data-mermaid-save-svg="${idx}">Save as SVG</button></div></div></div></div><div class="mermaid-block" data-mermaid-index="${idx}" data-mermaid-source="${btoa(encodeURIComponent(text))}">${escaped}</div></div>`;
      }

      let codeHtml: string;

      if (lang && hljs.getLanguage(lang)) {
        codeHtml = hljs.highlight(text, { language: lang }).value;
      } else {
        codeHtml = hljs.highlightAuto(text).value;
      }

      const copyIcon = icon(icons.Copy);
      return `<div class="code-block"><div class="code-block-header"><span class="code-lang">${langDisplayName(lang)}</span><button class="copy-btn" data-copy-index="${idx}">${copyIcon}</button></div><pre><code class="hljs">${codeHtml}</code></pre></div>`;
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
