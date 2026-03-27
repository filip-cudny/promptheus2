export type ErrorCode =
  | "no_active_prompt"
  | "execution_in_progress"
  | "clipboard_error"
  | "api_error"
  | "validation_error"
  | "timeout_error"
  | "unknown_error";

export interface ExecutionResult {
  success: boolean;
  content: string | null;
  error: string | null;
  error_code: ErrorCode | null;
  execution_time: number | null;
  metadata: unknown | null;
  execution_id: string | null;
}
