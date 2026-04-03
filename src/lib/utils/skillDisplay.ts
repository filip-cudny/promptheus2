const HAS_SKILL_RE = /^\/[a-z0-9-]+(\s|$)/m;

export function hasSkillReferences(text: string): boolean {
  return HAS_SKILL_RE.test(text);
}

const SKILL_TAG_RE = /<skill name="[^"]+">/;
const SKILL_BLOCK_RE =
  /<skill name="([^"]+)">[^]*?<\/skill>\s*<input>\n?([\s\S]*?)\n?<\/input>/g;

export function isSkillXml(content: string): boolean {
  return SKILL_TAG_RE.test(content);
}

export function extractSkillDisplayText(content: string): string {
  const parts: string[] = [];
  let match: RegExpExecArray | null;

  SKILL_BLOCK_RE.lastIndex = 0;
  while ((match = SKILL_BLOCK_RE.exec(content)) !== null) {
    const name = match[1];
    const input = match[2].trim();
    parts.push(input ? `/${name} ${input}` : `/${name}`);
  }

  return parts.length > 0 ? parts.join("\n") : content;
}
