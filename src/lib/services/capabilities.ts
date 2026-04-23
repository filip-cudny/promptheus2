import { invoke } from "@tauri-apps/api/core";
import type { ModelCapabilities, Provider } from "$lib/types";

export async function getModelCapabilities(
  provider: Provider,
  model: string,
): Promise<ModelCapabilities> {
  return invoke("get_model_capabilities", { provider, model });
}
