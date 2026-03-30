export interface SkillSegment {
  skillName: string | null;
  input: string;
}

export interface ResolvedSkillSegment {
  skillName: string;
  skillBody: string;
  input: string;
}

export function parseInputForSkills(text: string): SkillSegment[] {
  const segments: SkillSegment[] = [];
  const lines = text.split("\n");
  let currentSkill: string | null = null;
  let currentLines: string[] = [];

  for (const line of lines) {
    const match = line.match(/^\/([a-z0-9-]+)(?:\s+(.*))?$/);
    if (match) {
      if (currentLines.length > 0 || currentSkill !== null) {
        segments.push({
          skillName: currentSkill,
          input: currentLines.join("\n").trim(),
        });
      }
      currentSkill = match[1];
      currentLines = [];
      if (match[2]) {
        currentLines.push(match[2]);
      }
    } else {
      currentLines.push(line);
    }
  }

  if (currentLines.length > 0 || currentSkill !== null) {
    segments.push({
      skillName: currentSkill,
      input: currentLines.join("\n").trim(),
    });
  }

  return segments.filter((s) => s.input.length > 0 || s.skillName !== null);
}

export function composeSkillMessage(segments: ResolvedSkillSegment[]): string {
  const parts: string[] = [];

  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    parts.push(
      `<skill name="${seg.skillName}">\n${seg.skillBody}\n</skill>\n\n<input>\n${seg.input}\n</input>`,
    );

    if (i < segments.length - 1) {
      parts.push(`\n\n<skill-end name="${seg.skillName}" />\n`);
    }
  }

  return parts.join("");
}

export function hasSkillReferences(text: string): boolean {
  return /^\/[a-z0-9-]+(\s|$)/m.test(text);
}
