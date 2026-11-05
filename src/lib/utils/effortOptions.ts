import { REASONING_LEVELS, REASONING_LEVEL_LABELS, type ReasoningLevel } from "$lib/constants/models";

export interface EffortCapability {
  kind: string;
  allowed?: string[];
}

function effortLabel(v: string): string {
  return REASONING_LEVEL_LABELS[v as ReasoningLevel] ?? v;
}

export function effortOptions(
  reasoning: EffortCapability | null | undefined,
  saved: string | null | undefined,
): { value: string; label: string }[] {
  const allowed: string[] =
    reasoning?.kind === "effort" && reasoning.allowed ? reasoning.allowed : [...REASONING_LEVELS];
  const levels = allowed.includes("none") ? allowed : ["none", ...allowed];
  const options = [
    { value: "", label: "Default" },
    ...levels.map((v) => ({ value: v, label: effortLabel(v) })),
  ];
  if (saved && !levels.includes(saved)) {
    options.push({ value: saved, label: `${effortLabel(saved)} — not accepted by this model` });
  }
  return options;
}
