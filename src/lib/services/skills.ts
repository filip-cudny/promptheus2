import { invoke } from "@tauri-apps/api/core";
import type { Skill, SkillSummary } from "$lib/types";

export async function listSkills(): Promise<SkillSummary[]> {
  return invoke("list_skills");
}

export async function getSkill(name: string): Promise<Skill> {
  return invoke("get_skill", { name });
}

export async function getSkillBody(name: string): Promise<string> {
  return invoke("get_skill_body", { name });
}

export async function reloadSkills(): Promise<void> {
  return invoke("reload_skills");
}
