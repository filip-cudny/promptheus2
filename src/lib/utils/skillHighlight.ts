const SKILL_TOKEN_RE = /(^|(?<=\s))(\/[a-z0-9-]+)(\s|$)?/g;

export function escapeHtml(t: string): string {
  return t
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function highlightSkills(
  text: string,
  classify: (token: string, finished: boolean) => string | null,
  joinWith: string,
): string {
  if (!text) return "";
  return text
    .split("\n")
    .map((line) =>
      escapeHtml(line).replace(SKILL_TOKEN_RE, (_, pre, token, after) => {
        const finished = !!after;
        const cls = classify(token, finished);
        return cls
          ? `${pre}<span class="${cls}">${token}</span>${after ?? ""}`
          : `${pre}${token}${after ?? ""}`;
      }),
    )
    .join(joinWith);
}

export function fuzzyMatch(query: string, target: string): number | null {
  if (!query) return 0;
  let qi = 0;
  let score = 0;
  let lastMatchIndex = -1;

  for (let ti = 0; ti < target.length && qi < query.length; ti++) {
    if (query[qi] === target[ti]) {
      score += 1;
      if (ti === lastMatchIndex + 1) score += 2;
      if (ti === 0) score += 3;
      if (ti > 0 && target[ti - 1] === "-") score += 2;
      lastMatchIndex = ti;
      qi++;
    }
  }

  return qi === query.length ? score : null;
}
