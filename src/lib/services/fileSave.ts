import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export async function saveSvg(svg: string, defaultPath = "mermaid-diagram.svg"): Promise<void> {
  const path = await save({
    defaultPath,
    filters: [{ name: "SVG", extensions: ["svg"] }],
  });
  if (path) await invoke("write_text_file", { path, content: svg });
}
