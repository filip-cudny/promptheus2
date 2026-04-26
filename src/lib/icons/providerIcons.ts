import claudeIcon from "./providers/claude.svg?raw";
import chatgptIcon from "./providers/chatgpt.svg?raw";
import promptheusIconUrl from "../../../src-tauri/icons/tray_icon.png?url";
import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";

const HOST_ICONS: Record<string, string> = {
  "claude.ai": claudeIcon,
  "chatgpt.com": chatgptIcon,
  "chat.openai.com": chatgptIcon,
};

const ID_ICONS: Record<string, string> = {
  [PROMPTHEUS_PROVIDER_ID]: `<img src="${promptheusIconUrl}" alt="" aria-hidden="true" />`,
};

export function providerIconSvg(
  provider: { id?: string | null; url?: string | null } | null | undefined,
): string | null {
  if (!provider) return null;
  if (provider.id && ID_ICONS[provider.id]) {
    return ID_ICONS[provider.id];
  }
  if (!provider.url) return null;
  try {
    const host = new URL(provider.url).hostname.toLowerCase();
    return HOST_ICONS[host] ?? null;
  } catch {
    return null;
  }
}
