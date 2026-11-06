import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { EffectTarget, ProposalEffect, RowTest } from "$lib/services/settingsAgent";

export type LoggedState = "applied" | "undone";

export interface LoggedOperation {
  label: string;
  op: string;
  effects: ProposalEffect[];
  test: RowTest | null;
}

export interface ChangeLogEntry {
  id: string;
  created_at: string;
  host: string;
  kind: "card" | "secret" | "undo";
  request: string | null;
  reply: string | null;
  operations: LoggedOperation[];
  undoes: string | null;
  secret: { name: string; backup: string; had_previous: boolean } | null;
}

export interface ChangeLogItem {
  entry: ChangeLogEntry;
  state: LoggedState;
}

export interface UndoConflict {
  row: number;
  target: EffectTarget;
  expected_after: unknown;
  current: unknown;
}

export type UndoOutcome =
  | { status: "undone"; entry_id: string }
  | { status: "conflicts"; conflicts: UndoConflict[] }
  | { status: "skipped"; held_by: string };

export function listChangeLog(): Promise<ChangeLogItem[]> {
  return invoke("list_change_log");
}

export function undoChange(id: string, force: boolean): Promise<UndoOutcome> {
  return invoke("undo_change", { id, force });
}

export function onChangeLogChanged(cb: () => void): Promise<UnlistenFn> {
  return listen("change-log-changed", () => cb());
}
