import { invoke } from "@tauri-apps/api/core";
import type {
  ExportedSkill,
  ImportConflictMode,
  Skill,
  SkillFrontmatter,
  SkillFull,
  SkillSummary,
  SlugValidation,
} from "$lib/types";

export async function listSkills(): Promise<SkillSummary[]> {
  return invoke("list_skills");
}

export async function listSkillsFull(): Promise<SkillFull[]> {
  return invoke("list_skills_full");
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

export async function validateSkillSlug(slug: string): Promise<SlugValidation> {
  return invoke("validate_skill_slug", { slug });
}

export async function createSkill(
  slug: string,
  frontmatter: SkillFrontmatter,
  body: string,
): Promise<SkillFull> {
  return invoke("create_skill", { slug, frontmatter, body });
}

export async function updateSkill(
  slug: string,
  frontmatter: SkillFrontmatter,
  body: string,
): Promise<SkillFull> {
  return invoke("update_skill", { slug, frontmatter, body });
}

export async function deleteSkill(slug: string): Promise<void> {
  return invoke("delete_skill", { slug });
}

export async function duplicateSkill(slug: string, newSlug: string): Promise<SkillFull> {
  return invoke("duplicate_skill", { slug, newSlug });
}

export async function reorderSkills(order: string[]): Promise<void> {
  return invoke("reorder_skills", { order });
}

export async function importSkillFile(
  content: string,
  onConflict: ImportConflictMode = "reject",
): Promise<SkillFull> {
  return invoke("import_skill_file", { content, onConflict });
}

export async function exportSkill(slug: string): Promise<ExportedSkill> {
  return invoke("export_skill", { slug });
}

export async function previewSkillMessage(
  body: string,
  sampleInput: string,
  skillName?: string | null,
): Promise<string> {
  return invoke("preview_skill_message", {
    body,
    sampleInput,
    skillName: skillName ?? null,
  });
}
