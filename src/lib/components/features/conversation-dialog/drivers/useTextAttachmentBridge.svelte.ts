import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function useTextAttachmentBridge(opts: {
  getAttachments: () => string[];
  setAttachments: (next: string[]) => void;
}) {
  async function init(): Promise<() => void> {
    const win = getCurrentWebviewWindow();
    const unlisten = await win.listen<{ text: string; index: number }>(
      "text-attachment-updated",
      (event) => {
        const { text, index } = event.payload;
        const current = opts.getAttachments();
        if (index >= 0 && index < current.length) {
          opts.setAttachments(current.map((t, i) => (i === index ? text : t)));
        }
      },
    );
    return () => unlisten();
  }

  return { init };
}

export type TextAttachmentBridge = ReturnType<typeof useTextAttachmentBridge>;
