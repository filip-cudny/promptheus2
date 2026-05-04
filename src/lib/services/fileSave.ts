import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";

export async function saveSvg(svg: string, defaultPath = "mermaid-diagram.svg"): Promise<void> {
  const path = await save({
    defaultPath,
    filters: [{ name: "SVG", extensions: ["svg"] }],
  });
  if (path) await writeTextFile(path, svg);
}
