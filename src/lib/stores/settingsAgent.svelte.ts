import type { StreamEvent } from "$lib/types/ai";
import { applyStreamEvent, emptyReply, type AgentReply } from "$lib/utils/agentReply";
import { undoChange, type UndoConflict } from "$lib/services/changeLog";
import {
  onSettingsAgentProposal,
  onSettingsAgentSecretView,
  settingsAgentApply,
  settingsAgentCancel,
  settingsAgentPanel,
  settingsAgentRefresh,
  settingsAgentReject,
  settingsAgentReset,
  settingsAgentSend,
  settingsAgentStartDictation,
  settingsAgentTestRow,
  type PanelMode,
  type ProposalData,
} from "$lib/services/settingsAgent";

export type AgentMessage =
  | { id: number; role: "user"; text: string }
  | ({ id: number; role: "assistant" } & AgentReply);

let nextId = 0;
let messages = $state<AgentMessage[]>([]);
let running = $state(false);
let error = $state<string | null>(null);
let panelOpen = $state(false);
let panelMode = $state<PanelMode>("overlay");

export type ProposalOutcome =
  | { kind: "applied"; entryId: string; conflicts?: UndoConflict[]; message?: string }
  | { kind: "blocked"; message: string }
  | { kind: "undone" };

let proposal = $state<ProposalData | null>(null);
let proposalOutcome = $state<ProposalOutcome | null>(null);
let applying = $state(false);
let undoing = $state(false);

settingsAgentReset().catch((e) => (error = String(e)));

onSettingsAgentProposal((incoming) => {
  if (proposal?.proposal_id !== incoming.proposal_id) proposalOutcome = null;
  proposal = incoming;
}).catch((e) => (error = String(e)));

function lastAssistant() {
  const last = messages[messages.length - 1];
  return last?.role === "assistant" ? last : null;
}

onSettingsAgentSecretView((name) => {
  lastAssistant()?.secretViews.push(name);
}).catch((e) => (error = String(e)));

function onEvent(event: StreamEvent) {
  if (event.event === "error") error = event.data.message;
  else {
    const reply = lastAssistant();
    if (reply) applyStreamEvent(reply, event);
  }
}

function cancelUnfinishedToolCalls() {
  const now = new Date().toISOString();
  for (const call of lastAssistant()?.toolCalls ?? []) {
    if (call.status === "in_progress" || call.status === "pending") {
      call.status = "cancelled";
      call.completed_at = now;
    }
  }
}

async function send(text: string) {
  const message = text.trim();
  if (!message || running) return;
  error = null;
  messages.push({ id: nextId++, role: "user", text: message });
  messages.push({ id: nextId++, role: "assistant", ...emptyReply() });
  running = true;
  try {
    await settingsAgentSend(message, onEvent);
  } catch (e) {
    error = String(e);
  } finally {
    cancelUnfinishedToolCalls();
    const last = lastAssistant();
    if (last) last.isThinking = false;
    running = false;
  }
}

async function dictate() {
  error = null;
  try {
    await settingsAgentStartDictation();
  } catch (e) {
    error = String(e);
  }
}

async function cancel() {
  await settingsAgentCancel();
}

async function reset() {
  await settingsAgentReset();
  messages = [];
  error = null;
  proposal = null;
  proposalOutcome = null;
}

function blockedMessage(rows: number[]): string {
  return `Nothing was written. Rows changed since proposed: ${rows.map((r) => r + 1).join(", ")}. Refresh first.`;
}

async function applyProposal(selected: boolean[], values: Record<string, string>) {
  if (!proposal || applying) return;
  applying = true;
  try {
    const result = await settingsAgentApply(proposal.proposal_id, selected, values);
    if ("entry_id" in result) proposalOutcome = { kind: "applied", entryId: result.entry_id };
    else if ("stale_rows" in result) {
      for (const row of proposal.rows) if (result.stale_rows.includes(row.row)) row.stale = true;
      proposalOutcome = { kind: "blocked", message: blockedMessage(result.stale_rows) };
    }
    else proposalOutcome = { kind: "blocked", message: `Nothing was written. ${result.error}` };
  } catch (e) {
    proposalOutcome = { kind: "blocked", message: `Nothing was written. ${e}` };
  } finally {
    applying = false;
  }
}

async function undoProposal(force: boolean) {
  const current = proposalOutcome;
  if (current?.kind !== "applied" || undoing) return;
  const { entryId } = current;
  undoing = true;
  let next: ProposalOutcome;
  try {
    const result = await undoChange(entryId, force);
    if (result.status === "undone") next = { kind: "undone" };
    else if (result.status === "conflicts") next = { kind: "applied", entryId, conflicts: result.conflicts };
    else next = { kind: "applied", entryId, message: `Secret backup is on ${result.held_by}` };
  } catch (e) {
    next = { kind: "applied", entryId, message: `Undo failed: ${e}` };
  } finally {
    undoing = false;
  }
  if (proposalOutcome === current) proposalOutcome = next;
}

function dismissUndoConflicts() {
  if (proposalOutcome?.kind === "applied") proposalOutcome = { kind: "applied", entryId: proposalOutcome.entryId };
}

async function rejectProposal() {
  if (!proposal) return;
  const id = proposal.proposal_id;
  proposal = null;
  proposalOutcome = null;
  try {
    await settingsAgentReject(id);
  } catch (e) {
    error = String(e);
  }
}

async function refreshProposal() {
  if (!proposal) return;
  try {
    await settingsAgentRefresh(proposal.proposal_id);
    proposalOutcome = null;
  } catch (e) {
    proposalOutcome = { kind: "blocked", message: String(e) };
  }
}

async function testRow(row: number, values: Record<string, string>) {
  if (!proposal) return;
  try {
    const result = await settingsAgentTestRow(proposal.proposal_id, row, values);
    const target = proposal.rows.find((r) => r.row === row);
    if (target) target.test = result;
  } catch (e) {
    proposalOutcome = { kind: "blocked", message: String(e) };
  }
}

async function setPanelOpen(open: boolean) {
  panelOpen = open;
  panelMode = open ? await settingsAgentPanel(true) : "overlay";
  if (!open) await settingsAgentPanel(false);
}

export function getSettingsAgent() {
  return {
    get messages() { return messages; },
    get running() { return running; },
    get error() { return error; },
    get proposal() { return proposal; },
    get proposalOutcome() { return proposalOutcome; },
    get applying() { return applying; },
    get undoing() { return undoing; },
    applyProposal,
    undoProposal,
    dismissUndoConflicts,
    rejectProposal,
    refreshProposal,
    testRow,
    get panelOpen() { return panelOpen; },
    get panelMode() { return panelMode; },
    send,
    dictate,
    cancel,
    reset,
    setPanelOpen,
  };
}
