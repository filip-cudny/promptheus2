export * from "./ai";
export * from "./menu";
export * from "./execution";
export * from "./context";
export * from "./history";
export * from "./conversation";

export interface Settings {
  show_tray_icon: boolean;
  debug_mode: boolean;
  code_theme: string;
  menu_section_order: string[];
  description_generator: DescriptionGenerator;
  notifications: NotificationSettings;
  speech_to_text_model: SpeechToTextModel | null;
  default_model: string | null;
  number_input_debounce_ms: number;
  models: ModelConfig[];
  keymaps: KeymapGroup[];
  prompts: PromptData[];
  system_prompt: string;
  about_me: string | null;
  context_section: string | null;
  recent_apps_count: number;
  skills_order: string[];
  conversation_title_model: string;
  conversation_title_prompt: string;
}

export interface ModelConfig {
  id: string;
  model: string;
  display_name: string;
  api_key_source: ApiKeySource;
  provider: Provider;
  api_key_env: string | null;
  api_key: string | null;
  base_url: string | null;
  parameters: ModelParameters | null;
}

export type ApiKeySource = "env" | "direct";

export type Provider = "openai" | "anthropic" | "gemini";

export interface ModelParameters {
  temperature: number | null;
  max_tokens: number | null;
  top_p: number | null;
  frequency_penalty: number | null;
  presence_penalty: number | null;
  reasoning_effort: string | null;
}

export interface SpeechToTextModel {
  model: string;
  display_name: string;
  api_key_env: string;
  base_url: string | null;
  api_key: string | null;
}

export interface PromptData {
  id: string;
  name: string;
  description: string | null;
  messages: PromptMessage[];
}

export interface PromptMessage {
  role: string;
  content: string;
}

export interface KeymapGroup {
  context: string;
  bindings: Record<string, string>;
}

export interface NotificationSettings {
  events: NotificationEvents;
  background_colors: NotificationColors;
  monochromatic_notification_icons: boolean;
  opacity: number | null;
}

export interface NotificationEvents {
  prompt_execution_success: boolean;
  prompt_execution_cancel: boolean;
  prompt_execution_in_progress: boolean;
  speech_recording_start: boolean;
  speech_recording_stop: boolean;
  speech_transcription_success: boolean;
  context_saved: boolean;
  context_set: boolean;
  context_append: boolean;
  context_cleared: boolean;
  clipboard_copy: boolean;
  image_added: boolean;
}

export interface NotificationColors {
  success: string;
  error: string;
  info: string;
  warning: string;
}

export interface SkillSummary {
  name: string;
  display_name: string;
  description: string | null;
}

export interface Skill extends SkillSummary {
  body: string;
}

export interface DescriptionGenerator {
  model: string;
  system_prompt: string | null;
  prompt: string | null;
}

