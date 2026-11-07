import type { ChangeLogEntry, UndoConflict } from "$lib/services/changeLog";
import type { EffectTarget } from "$lib/services/settingsAgent";

export function targetLabel(target: EffectTarget): string {
  return target.type === "secret" ? target.name : target.path;
}

export function formatValue(value: unknown): string {
  if (value === null || value === undefined) return "(none)";
  return typeof value === "string" ? value : JSON.stringify(value);
}

export function formatEntryTime(createdAt: string): string {
  return new Date(createdAt).toLocaleString();
}

export function entryTitle(
  entry: ChangeLogEntry,
  entries: ChangeLogEntry[],
  formatTime: (createdAt: string) => string = formatEntryTime,
): string {
  if (entry.kind === "secret") return `Secret ${entry.secret?.name ?? ""}`.trim();
  if (entry.kind === "undo") {
    const undone = entries.find((e) => e.id === entry.undoes);
    return undone ? `Undo of ${formatTime(undone.created_at)}` : "Undo of an earlier entry";
  }
  return entry.operations.map((o) => o.label).join(", ") || "Change";
}

export function conflictLines(conflicts: UndoConflict[]): string[] {
  return conflicts.map((c) =>
    c.target.type === "secret"
      ? `${c.target.name}: changed since this entry`
      : `${c.target.path}: current ${formatValue(c.current)}, expected ${formatValue(c.expected_after)}`,
  );
}
