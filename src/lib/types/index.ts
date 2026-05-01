export * from "./ai";
export * from "./menu";
export * from "./execution";
export * from "./context";
export * from "./history";
export * from "./conversation";

export type SurfaceKind = "chat" | "quick_actions" | "title_generation" | "speech_to_text";

export interface Settings {
  show_tray_icon: boolean;
  debug_mode: boolean;
  code_theme: string;
  theme: string;
  menu_section_order: string[];
  description_generator: DescriptionGenerator;
  notifications: NotificationSettings;
  number_input_debounce_ms: number;
  models: ModelConfig[];
  prompt_base: PromptBase;
  surfaces: Surfaces;
  keymaps: KeymapGroup[];
  recent_apps_count: number;
  skills_order: string[];
  webview_providers: WebviewProvider[];
}

export interface WebviewProvider {
  id: string;
  name: string;
  url: string;
}

export interface PromptBase {
  system_prompt: string;
  about_me: string | null;
  environment_section: string | null;
}

export interface Surfaces {
  chat: ChatConfig;
  quick_actions: QuickActionsConfig;
  title_generation: TitleGenConfig;
  speech_to_text: SpeechToTextConfig;
}

export interface GenerationConfig {
  model_id: string | null;
  parameters: ModelParameters;
  enabled_tools: string[];
}

export interface ChatConfig {
  generation: GenerationConfig;
}

export interface QuickActionsConfig {
  generation: GenerationConfig;
}

export interface TitleGenConfig {
  generation: GenerationConfig;
  prompt: string;
}

export interface SpeechToTextConfig {
  model_id: string | null;
  language: string | null;
  keyterms_file: string | null;
  no_verbatim: boolean | null;
  prompt: string | null;
}

export type ModelType = "text" | "stt";
export type ApiMode = "responses" | "completions";

export interface ModelConfig {
  id: string;
  model: string;
  display_name: string;
  type: ModelType;
  provider: Provider | null;
  group: string | null;
  api_key: string | null;
  base_url: string | null;
  parameters: ModelParameters | null;
  context_window_size: number | null;
  api_mode: ApiMode | null;
  store: boolean;
}

export type Provider = "openai" | "anthropic" | "gemini" | "elevenlabs";

export interface ModelParameters {
  temperature: number | null;
  max_tokens: number | null;
  top_p: number | null;
  frequency_penalty: number | null;
  presence_penalty: number | null;
  reasoning_effort: string | null;
  [key: string]: unknown;
}

export const KNOWN_MODEL_PARAMETER_KEYS = [
  "temperature",
  "max_tokens",
  "top_p",
  "frequency_penalty",
  "presence_penalty",
  "reasoning_effort",
] as const;

export type KnownModelParameterKey = (typeof KNOWN_MODEL_PARAMETER_KEYS)[number];

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
