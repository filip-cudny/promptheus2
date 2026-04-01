import { listSkills } from "$lib/services/skills";
import type { SkillSummary } from "$lib/types";

let items = $state.raw<SkillSummary[]>([]);
let initialized = $state(false);

const nameSet = $derived(new Set(items.map((s) => s.name)));

async function init() {
  if (initialized) return;
  initialized = true;
  try {
    items = await listSkills();
  } catch {
    items = [];
  }
}

export function getSkillsStore() {
  return {
    get items() {
      return items;
    },
    get nameSet() {
      return nameSet;
    },
    get initialized() {
      return initialized;
    },
    init,
  };
}
