import type {
  HistoryStatusFilter,
  HistoryTypeFilter,
  SkillCount,
  TimeRangePreset,
} from "$lib/types/historySearch";

export function formatActiveFilters(s: {
  query: string;
  typeFilter: HistoryTypeFilter;
  statusFilter: HistoryStatusFilter;
  skillFilter: Set<string>;
  availableSkills: SkillCount[];
  timeRange: TimeRangePreset;
}): string {
  const parts: string[] = [];
  if (s.query.trim()) parts.push(`Query: "${s.query.trim()}"`);
  if (s.typeFilter !== "all") parts.push(`Type: ${typeLabel(s.typeFilter)}`);
  if (s.statusFilter !== "all") parts.push(`Status: ${capitalize(s.statusFilter)}`);
  if (s.skillFilter.size === 1) {
    const [id] = Array.from(s.skillFilter);
    const name = s.availableSkills.find((x) => x.skill_id === id)?.skill_name ?? id;
    parts.push(`Skill: ${name}`);
  } else if (s.skillFilter.size > 1) {
    parts.push(`Skills: ${s.skillFilter.size}`);
  }
  if (s.timeRange !== "all") parts.push(`Time: ${timeRangeLabel(s.timeRange)}`);
  return parts.join(" · ");
}

export function timeRangeLabel(t: TimeRangePreset): string {
  switch (t) {
    case "today":
      return "Today";
    case "7d":
      return "Last 7 days";
    case "30d":
      return "Last 30 days";
    default:
      return "All time";
  }
}

function typeLabel(t: HistoryTypeFilter): string {
  return t === "quick_action" ? "Quick Action" : capitalize(t);
}

function capitalize(s: string): string {
  return s ? s[0].toUpperCase() + s.slice(1) : s;
}
