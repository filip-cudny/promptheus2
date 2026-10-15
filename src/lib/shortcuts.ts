export type Shortcut = {
  key: string;
  mod?: boolean;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
};

export const SHORTCUTS = {
  reloadActive:    { mod: true, key: "r" },
  openPalette:     { mod: true, key: "p" },
  openChatPalette: { mod: true, key: "k" },
  nextTab:         { ctrl: true, key: "Tab" },
  prevTab:         { ctrl: true, shift: true, key: "Tab" },
  newTab:          { mod: true, key: "t" },
  closeTab:        { mod: true, key: "w" },
} as const satisfies Record<string, Shortcut>;

export function matches(e: KeyboardEvent, s: Shortcut): boolean {
  const mod = s.ctrl ? e.ctrlKey : e.metaKey || e.ctrlKey;
  return (
    e.key.toLowerCase() === s.key.toLowerCase() &&
    !!mod === !!(s.mod || s.ctrl) &&
    !!e.shiftKey === !!s.shift &&
    !!e.altKey === !!s.alt
  );
}

export function tabPosition(e: KeyboardEvent): number | null {
  if (!(e.metaKey || e.ctrlKey) || e.shiftKey || e.altKey) return null;
  return /^[1-9]$/.test(e.key) ? Number(e.key) : null;
}

const MAC_SYMBOLS = { mod: "⌘", shift: "⇧", alt: "⌥" } as const;
const OTHER_LABELS = { mod: "Ctrl", shift: "Shift", alt: "Alt" } as const;

export function formatShortcut(s: Shortcut, platform: "mac" | "other"): string {
  const sym = platform === "mac" ? MAC_SYMBOLS : OTHER_LABELS;
  const sep = platform === "mac" ? "" : "+";
  const parts: string[] = [];
  if (s.ctrl) parts.push("Ctrl");
  if (s.mod) parts.push(sym.mod);
  if (s.shift) parts.push(sym.shift);
  if (s.alt) parts.push(sym.alt);
  parts.push(s.key.length === 1 ? s.key.toUpperCase() : s.key);
  return parts.join(sep);
}

function shortcutSignature(s: Shortcut): string {
  return `${s.key.toLowerCase()}|${!!s.mod}|${!!s.ctrl}|${!!s.shift}|${!!s.alt}`;
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
