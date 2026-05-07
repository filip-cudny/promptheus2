import { invoke } from "@tauri-apps/api/core";
import type { ModelCapabilities } from "$lib/types";

export async function getModelCapabilities(
  modelId: string,
): Promise<ModelCapabilities> {
  return invoke("get_model_capabilities", { modelId });
}
