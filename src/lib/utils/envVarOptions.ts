import type { EnvVarStatus } from "$lib/types";

const ENV_REF_RE = /^\$\{([A-Z_][A-Z0-9_]*)\}$/;

export const LITERAL_LABEL = "Literal key";

export type ReferenceState =
  | { kind: "empty" }
  | { kind: "listed"; name: string }
  | { kind: "unlisted"; name: string }
  | { kind: "literal" };

export interface PickerOption {
  value: string;
  label: string;
}

export function parseEnvRef(value: string | null | undefined): string | null {
  if (!value) return null;
  const m = value.match(ENV_REF_RE);
  return m ? m[1] : null;
}

export function classifyReference(
  value: string | null | undefined,
  listed: string[],
): ReferenceState {
  if (!value) return { kind: "empty" };
  const name = parseEnvRef(value);
  if (!name) return { kind: "literal" };
  return listed.includes(name) ? { kind: "listed", name } : { kind: "unlisted", name };
}

export function pickerOptions(
  listed: Pick<EnvVarStatus, "name" | "set">[],
  value: string | null | undefined,
  extras: PickerOption[],
): PickerOption[] {
  const options = listed.map((v) => ({
    value: `\${${v.name}}`,
    label: v.set ? v.name : `${v.name} (missing)`,
  }));
  const state = classifyReference(
    value,
    listed.map((v) => v.name),
  );
  if (state.kind === "unlisted" && value) {
    options.push({ value, label: `${state.name} (not listed)` });
  } else if (state.kind === "literal" && value) {
    options.push({ value, label: LITERAL_LABEL });
  }
  return [...options, ...extras];
}
