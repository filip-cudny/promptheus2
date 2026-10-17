import claudeIcon from "./providers/claude.svg?raw";
import chatgptIcon from "./providers/chatgpt.svg?raw";
import logoUrl from "../../../src-tauri/icons/logo.svg?url";
import logoMonoUrl from "../../../src-tauri/icons/logo-mono.svg?url";
import { getLogo } from "$lib/stores/logo.svelte";
import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";

const HOST_ICONS: Record<string, string> = {
  "claude.ai": claudeIcon,
  "chatgpt.com": chatgptIcon,
  "chat.openai.com": chatgptIcon,
};

export function providerIconSvg(
  provider: { id?: string | null; url?: string | null } | null | undefined,
): string | null {
  if (!provider) return null;
  if (provider.id === PROMPTHEUS_PROVIDER_ID) {
    const src = getLogo() === "mono" ? logoMonoUrl : logoUrl;
    return `<img src="${src}" alt="" aria-hidden="true" />`;
  }
  if (!provider.url) return null;
  try {
    const host = new URL(provider.url).hostname.toLowerCase();
    return HOST_ICONS[host] ?? null;
  } catch {
    return null;
  }
}
