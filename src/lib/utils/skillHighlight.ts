const SKILL_TOKEN_RE = /(^|(?<=\s))(\/[a-z0-9-]+)/g;

export function escapeHtml(t: string): string {
  return t
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function highlightSkills(
  text: string,
  isKnown: (token: string) => boolean,
  spanClass: string,
  joinWith: string,
): string {
  if (!text) return "";
  return text
    .split("\n")
    .map((line) =>
      escapeHtml(line).replace(SKILL_TOKEN_RE, (_, pre, token) =>
        isKnown(token)
          ? `${pre}<span class="${spanClass}">${token}</span>`
          : `${pre}${token}`,
      ),
    )
    .join(joinWith);
}
