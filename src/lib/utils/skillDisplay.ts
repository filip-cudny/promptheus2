const HAS_SKILL_RE = /^\/[a-z0-9-]+(\s|$)/m;

export function hasSkillReferences(text: string): boolean {
  return HAS_SKILL_RE.test(text);
}

const SKILL_OPEN_RE = /<skill name="([^"]+)">/;
const SKILL_OPEN_RE_GLOBAL = /<skill name="([^"]+)">/g;
const INPUT_BLOCK_RE = /<\/skill>\s*<input>\n?([\s\S]*?)(?:\n?<\/input>|$)/;

export function isSkillXml(content: string): boolean {
  return SKILL_OPEN_RE.test(content);
}

export function extractSkillDisplayText(content: string): string {
  if (!content || !SKILL_OPEN_RE.test(content)) return content;

  const parts: string[] = [];
  let cursor = 0;

  SKILL_OPEN_RE_GLOBAL.lastIndex = 0;
  const opens: { name: string; tagStart: number; tagEnd: number }[] = [];
  let m: RegExpExecArray | null;
  while ((m = SKILL_OPEN_RE_GLOBAL.exec(content)) !== null) {
    opens.push({
      name: m[1],
      tagStart: m.index,
      tagEnd: m.index + m[0].length,
    });
  }

  for (let i = 0; i < opens.length; i++) {
    const open = opens[i];
    const blockEnd = i + 1 < opens.length ? opens[i + 1].tagStart : content.length;

    if (open.tagStart > cursor) {
      const lead = content.slice(cursor, open.tagStart).trim();
      if (lead) parts.push(lead);
    }

    const blockBody = content.slice(open.tagEnd, blockEnd);
    const inputMatch = INPUT_BLOCK_RE.exec(blockBody);
    const input = inputMatch ? inputMatch[1].trim() : "";

    parts.push(input ? `/${open.name} ${input}` : `/${open.name}`);
    cursor = blockEnd;
  }

  if (cursor < content.length) {
    const tail = content.slice(cursor).trim();
    if (tail) parts.push(tail);
  }

  return parts.length > 0 ? parts.join("\n") : content;
}

export function buildUserNodeDisplay(
  content: string,
  textAttachments: readonly string[] | null | undefined,
): string {
  const skillText = extractSkillDisplayText(content);
  const attachments = (textAttachments ?? [])
    .map((a) => a.trim())
    .filter(Boolean);
  if (attachments.length === 0) return skillText;
  const joined = attachments.join("\n");
  if (!skillText) return joined;
  return `${skillText}\n${joined}`;
}
