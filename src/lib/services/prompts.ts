import { invoke } from "@tauri-apps/api/core";

export type PromptKind =
  | "system"
  | "about_you"
  | "environment"
  | "input_format"
  | "title_generation"
  | "speech_to_text";

export interface PromptDoc {
  kind: PromptKind;
  label: string;
  path: string;
  content: string;
  supports_env_placeholders: boolean;
}

export interface EnvPlaceholder {
  token: string;
  name: string;
  label: string;
  description: string;
  example: string;
}

export async function listPrompts(): Promise<PromptDoc[]> {
  return invoke("list_prompts");
}

export async function getPrompt(kind: PromptKind): Promise<PromptDoc> {
  return invoke("get_prompt", { kind });
}

export async function savePrompt(kind: PromptKind, content: string): Promise<void> {
  return invoke("save_prompt", { kind, content });
}

export async function getEnvironmentPlaceholders(): Promise<EnvPlaceholder[]> {
  return invoke("get_environment_placeholders");
}
