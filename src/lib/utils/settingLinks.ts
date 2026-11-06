const SETTING_SCHEME = "setting:";
const SETTING_LINK = /\[[^\]]*\]\((setting:[^)\s]+)\)/g;

export function parseSettingLink(href: string): string | null {
  if (!href.startsWith(SETTING_SCHEME)) return null;
  const key = decodeURIComponent(href.slice(SETTING_SCHEME.length));
  return key || null;
}

export function showSettingLinks(markdown: string): string {
  return markdown.replace(SETTING_LINK, "[show]($1)");
}
