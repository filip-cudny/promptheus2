const HAS_SKILL_RE = /^\/[a-z0-9-]+(\s|$)/m;

export function hasSkillReferences(text: string): boolean {
  return HAS_SKILL_RE.test(text);
}
