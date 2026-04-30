export type Shortcut = {
  key: string;
  mod?: boolean;
  shift?: boolean;
  alt?: boolean;
};

export const SHORTCUTS = {
  reloadActive:    { mod: true, key: "r" },
  openPalette:     { mod: true, key: "p" },
  openChatPalette: { mod: true, key: "k" },
} as const satisfies Record<string, Shortcut>;

export function matches(e: KeyboardEvent, s: Shortcut): boolean {
  const mod = e.metaKey || e.ctrlKey;
  return (
    e.key.toLowerCase() === s.key.toLowerCase() &&
    !!mod === !!s.mod &&
    !!e.shiftKey === !!s.shift &&
    !!e.altKey === !!s.alt
  );
}

const MAC_SYMBOLS = { mod: "⌘", shift: "⇧", alt: "⌥" } as const;
const OTHER_LABELS = { mod: "Ctrl", shift: "Shift", alt: "Alt" } as const;

export function formatShortcut(s: Shortcut, platform: "mac" | "other"): string {
  const sym = platform === "mac" ? MAC_SYMBOLS : OTHER_LABELS;
  const sep = platform === "mac" ? "" : "+";
  const parts: string[] = [];
  if (s.mod) parts.push(sym.mod);
  if (s.shift) parts.push(sym.shift);
  if (s.alt) parts.push(sym.alt);
  parts.push(s.key.length === 1 ? s.key.toUpperCase() : s.key);
  return parts.join(sep);
}

function shortcutSignature(s: Shortcut): string {
  return `${s.key.toLowerCase()}|${!!s.mod}|${!!s.shift}|${!!s.alt}`;
}

(function assertNoCollisions() {
  const seen = new Map<string, string>();
  for (const [name, s] of Object.entries(SHORTCUTS)) {
    const sig = shortcutSignature(s);
    const prev = seen.get(sig);
    if (prev) {
      throw new Error(
        `Shortcut collision: "${name}" and "${prev}" both bind ${formatShortcut(s, "other")}`,
      );
    }
    seen.set(sig, name);
  }
})();
