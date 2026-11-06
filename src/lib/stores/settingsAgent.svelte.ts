import type { StreamEvent, ToolCallStatus } from "$lib/types/ai";
import {
  onSettingsAgentProposal,
  onSettingsAgentSecretRequest,
  settingsAgentApply,
  settingsAgentCancel,
  settingsAgentPanel,
  settingsAgentRefresh,
  settingsAgentReject,
  settingsAgentReset,
  settingsAgentSecretResult,
  settingsAgentSend,
  settingsAgentTestRow,
  type PanelMode,
  type ProposalData,
  type SecretOutcome,
  type SecretRequest,
} from "$lib/services/settingsAgent";

export interface AgentToolCall {
  id: string;
  name: string;
  status: ToolCallStatus;
}

export type AgentMessage =
  | { id: number; role: "user"; text: string }
  | { id: number; role: "assistant"; text: string; toolCalls: AgentToolCall[] };

let nextId = 0;
let messages = $state<AgentMessage[]>([]);
let running = $state(false);
let error = $state<string | null>(null);
let panelOpen = $state(false);
let panelMode = $state<PanelMode>("overlay");

export type ProposalOutcome =
  | { kind: "applied"; entryId: string }
  | { kind: "blocked"; message: string };

let proposal = $state<ProposalData | null>(null);
let proposalOutcome = $state<ProposalOutcome | null>(null);
let applying = $state(false);
let secretRequests = $state<SecretRequest[]>([]);

settingsAgentReset().catch((e) => (error = String(e)));

onSettingsAgentProposal((incoming) => {
  if (proposal?.proposal_id !== incoming.proposal_id) proposalOutcome = null;
  proposal = incoming;
}).catch((e) => (error = String(e)));

onSettingsAgentSecretRequest((request) => {
  secretRequests.push(request);
}).catch((e) => (error = String(e)));

function lastAssistant() {
  const last = messages[messages.length - 1];
  return last?.role === "assistant" ? last : null;
}

function onEvent(event: StreamEvent) {
  const reply = lastAssistant();
  if (!reply) return;
  switch (event.event) {
    case "chunk":
      reply.text = event.data.accumulated;
      break;
    case "done":
      reply.text = event.data.full_text;
      break;
    case "error":
      error = event.data.message;
      break;
    case "tool_call_start":
      reply.toolCalls.push({
        id: event.data.tool_call.tool_call_id,
        name: event.data.tool_call.tool_display_name || event.data.tool_call.tool_name,
        status: "in_progress",
      });
      break;
    case "tool_call_done": {
      const call = reply.toolCalls.find((c) => c.id === event.data.tool_call_id);
      if (call) call.status = event.data.error ? "failed" : "completed";
      break;
    }
  }
}

async function send(text: string) {
  const message = text.trim();
  if (!message || running) return;
  error = null;
  messages.push({ id: nextId++, role: "user", text: message });
  messages.push({ id: nextId++, role: "assistant", text: "", toolCalls: [] });
  running = true;
  try {
    await settingsAgentSend(message, onEvent);
  } catch (e) {
    error = String(e);
  } finally {
    running = false;
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
  secretRequests = [];
}

function blockedMessage(rows: number[]): string {
  return `Nothing was written. Rows changed since proposed: ${rows.map((r) => r + 1).join(", ")}. Refresh first.`;
}

async function applyProposal(selected: boolean[]) {
  if (!proposal || applying) return;
  applying = true;
  try {
    const result = await settingsAgentApply(proposal.proposal_id, selected);
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

async function testRow(row: number) {
  if (!proposal) return;
  try {
    const result = await settingsAgentTestRow(proposal.proposal_id, row);
    const target = proposal.rows.find((r) => r.row === row);
    if (target) target.test = result;
  } catch (e) {
    proposalOutcome = { kind: "blocked", message: String(e) };
  }
}

async function resolveSecret(requestId: string, outcome: SecretOutcome) {
  secretRequests = secretRequests.filter((r) => r.request_id !== requestId);
  try {
    await settingsAgentSecretResult(requestId, outcome);
  } catch (e) {
    error = String(e);
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
    get secretRequests() { return secretRequests; },
    applyProposal,
    rejectProposal,
    refreshProposal,
    testRow,
    resolveSecret,
    get panelOpen() { return panelOpen; },
    get panelMode() { return panelMode; },
    send,
    cancel,
    reset,
    setPanelOpen,
  };
}
