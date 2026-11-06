import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { StreamEvent } from "$lib/types/ai";

export type PanelMode = "widened" | "overlay";

export function settingsAgentSend(
  message: string,
  onEvent: (event: StreamEvent) => void,
): Promise<void> {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return invoke("settings_agent_send", { message, onEvent: channel });
}

export function settingsAgentCancel(): Promise<void> {
  return invoke("settings_agent_cancel");
}

export function settingsAgentReset(): Promise<void> {
  return invoke("settings_agent_reset");
}

export function settingsAgentPanel(open: boolean): Promise<PanelMode> {
  return invoke("settings_agent_panel", { open });
}

export type EffectTarget = { type: "setting"; path: string } | { type: "file"; path: string };

export interface ProposalEffect {
  target: EffectTarget;
  before: unknown;
  after: unknown;
}

export interface RowTest {
  ok: boolean;
  millis: number | null;
  error: string | null;
  output: string | null;
}

export interface ProposalRowData {
  row: number;
  label: string;
  op: string;
  effects: ProposalEffect[];
  test: RowTest | null;
  sample_input: string | null;
  stale: boolean;
}

export interface ProposalData {
  proposal_id: string;
  rows: ProposalRowData[];
}

export interface SecretRequest {
  request_id: string;
  name: string;
  purpose: string;
  exists: boolean;
}

export type ApplyResult =
  | { entry_id: string }
  | { stale_rows: number[] }
  | { error: string };

export type SecretOutcome = "saved" | "cancelled";

export function onSettingsAgentProposal(cb: (proposal: ProposalData) => void): Promise<UnlistenFn> {
  return listen<ProposalData>("settings-agent-proposal", (e) => cb(e.payload));
}

export function onSettingsAgentSecretRequest(cb: (request: SecretRequest) => void): Promise<UnlistenFn> {
  return listen<SecretRequest>("settings-agent-secret-request", (e) => cb(e.payload));
}

export function settingsAgentApply(proposalId: string, selected: boolean[]): Promise<ApplyResult> {
  return invoke("settings_agent_apply", { proposalId, selected });
}

export function settingsAgentReject(proposalId: string): Promise<void> {
  return invoke("settings_agent_reject", { proposalId });
}

export function settingsAgentRefresh(proposalId: string): Promise<void> {
  return invoke("settings_agent_refresh", { proposalId });
}

export function settingsAgentTestRow(proposalId: string, row: number): Promise<RowTest> {
  return invoke("settings_agent_test_row", { proposalId, row });
}

export function settingsAgentSecretResult(requestId: string, outcome: SecretOutcome): Promise<void> {
  return invoke("settings_agent_secret_result", { requestId, outcome });
}
