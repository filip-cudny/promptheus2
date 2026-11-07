import type { KeymapGroup } from "../types";

function groupMatchesOs(context: string, os: string): boolean {
  const trimmed = context.trim();
  const rest = trimmed.startsWith("os ==")
    ? trimmed.slice("os ==".length)
    : trimmed.startsWith("os==")
      ? trimmed.slice("os==".length)
      : trimmed;
  return rest.trim() === os;
}

export function actionShortcuts(
  keymaps: KeymapGroup[],
  action: string,
  os: string,
): string[] {
  return keymaps
    .filter((group) => groupMatchesOs(group.context, os))
    .flatMap((group) => Object.entries(group.bindings))
    .filter(([, bound]) => bound === action)
    .map(([shortcut]) => shortcut)
    .sort();
}

export function formatShortcut(shortcut: string): string {
  return shortcut
    .split("+")
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join("+");
}
