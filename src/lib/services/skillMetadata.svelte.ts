import { invoke } from "@tauri-apps/api/core";
import { debug as logDebug } from "@tauri-apps/plugin-log";

export type SkillMeta = {
  model: string | null;
  parameters: Record<string, unknown> | null;
};

export type MetaEntry = { key: string; value: string };

let cache = $state<Record<string, SkillMeta>>({});

export function getSkillMeta(skillId: string): SkillMeta | undefined {
  return cache[skillId];
}

export async function fetchSkillMetadata(skillId: string): Promise<void> {
  if (skillId in cache) return;
  cache = { ...cache, [skillId]: { model: null, parameters: null } };
  try {
    const skill = await invoke<{
      model?: string | null;
      parameters?: Record<string, unknown> | null;
    }>("get_skill", { name: skillId });
    cache = {
      ...cache,
      [skillId]: {
        model: skill?.model ?? null,
        parameters: skill?.parameters ?? null,
      },
    };
  } catch (e) {
    logDebug(`get_skill failed for ${skillId}: ${e}`);
  }
}

export function buildSkillMetaEntries(
  skillId: string,
  modelNames: Map<string, string>,
): MetaEntry[] {
  const meta = cache[skillId];
  if (!meta) return [];
  const out: MetaEntry[] = [];
  if (meta.model) {
    out.push({ key: "model", value: modelNames.get(meta.model) ?? meta.model });
  }
  if (meta.parameters) {
    for (const [k, v] of Object.entries(meta.parameters)) {
      if (v === null || v === undefined) continue;
      const value = typeof v === "object" ? JSON.stringify(v) : String(v);
      out.push({ key: k, value });
    }
  }
  return out;
}
