import type { CatalogEntry } from "../types/settingsCatalog";

const MAX_RESULTS = 20;

function words(text: string): string[] {
  return text.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
}

function typoBudget(length: number): number {
  if (length < 4) return 0;
  return length <= 6 ? 1 : 2;
}

function editDistance(a: string[], b: string[]): number {
  const rows = a.length + 1;
  const cols = b.length + 1;
  const d: number[][] = Array.from({ length: rows }, (_, i) => {
    const row = new Array<number>(cols).fill(0);
    row[0] = i;
    return row;
  });
  for (let j = 0; j < cols; j++) d[0][j] = j;
  for (let i = 1; i < rows; i++) {
    for (let j = 1; j < cols; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost);
      if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) {
        d[i][j] = Math.min(d[i][j], d[i - 2][j - 2] + 1);
      }
    }
  }
  return d[rows - 1][cols - 1];
}

function matchesByTypo(query: string[], word: string[], k: number): boolean {
  for (let length = Math.max(1, query.length - k); length <= query.length + k; length++) {
    if (editDistance(query, word.slice(0, length)) <= k) return true;
  }
  return false;
}

function matchesWord(q: string, w: string): boolean {
  if (w.startsWith(q)) return true;
  const query = [...q];
  const k = typoBudget(query.length);
  return k > 0 && matchesByTypo(query, [...w], k);
}

function matchesAllExact(queryWords: string[], haystack: string[]): boolean {
  return queryWords.every((q) => haystack.some((w) => w.startsWith(q)));
}

function matchesAllFuzzy(queryWords: string[], haystack: string[]): boolean {
  return queryWords.every((q) => haystack.some((w) => matchesWord(q, w)));
}

export function searchCatalog(entries: CatalogEntry[], query: string): CatalogEntry[] {
  const queryWords = words(query);
  if (queryWords.length === 0) return [];
  const tiers: CatalogEntry[][] = [[], [], [], []];
  for (const entry of entries) {
    const label = words(entry.label);
    const labelAndDescription = [...label, ...words(entry.description)];
    if (matchesAllExact(queryWords, label)) tiers[0].push(entry);
    else if (matchesAllExact(queryWords, labelAndDescription)) tiers[1].push(entry);
    else if (matchesAllFuzzy(queryWords, label)) tiers[2].push(entry);
    else if (matchesAllFuzzy(queryWords, labelAndDescription)) tiers[3].push(entry);
  }
  return tiers.flat().slice(0, MAX_RESULTS);
}

export function catalogPath(concrete: string): string {
  const path = concrete.replace(/\[[^\]]*\]/g, "[]");
  const tail = path.startsWith("mcp_servers.") ? path.slice("mcp_servers.".length) : null;
  if (tail === null) return path;
  const dot = tail.indexOf(".");
  return dot < 0 ? "mcp_servers.*" : `mcp_servers.*.${tail.slice(dot + 1)}`;
}

export interface ListItemRef {
  list: "models" | "mcp_servers" | "skills";
  id: string;
}

export function listItemOf(concrete: string): ListItemRef | null {
  const model = /^models\[([^\]]*)\]/.exec(concrete);
  if (model) return { list: "models", id: model[1] };
  const skill = /^skills\[([^\]]*)\]/.exec(concrete);
  if (skill) return { list: "skills", id: skill[1] };
  const server = /^mcp_servers\.([^.]+)/.exec(concrete);
  if (server) return { list: "mcp_servers", id: server[1] };
  return null;
}
