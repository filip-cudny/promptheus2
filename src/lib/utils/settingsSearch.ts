import type { CatalogEntry } from "../types/settingsCatalog";

const MAX_RESULTS = 20;

function words(text: string): string[] {
  return text.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
}

function matchesAll(queryWords: string[], haystack: string[]): boolean {
  return queryWords.every((q) => haystack.some((w) => w.startsWith(q)));
}

export function searchCatalog(entries: CatalogEntry[], query: string): CatalogEntry[] {
  const queryWords = words(query);
  if (queryWords.length === 0) return [];
  const byLabel: CatalogEntry[] = [];
  const byDescription: CatalogEntry[] = [];
  for (const entry of entries) {
    const label = words(entry.label);
    if (matchesAll(queryWords, label)) byLabel.push(entry);
    else if (matchesAll(queryWords, [...label, ...words(entry.description)])) {
      byDescription.push(entry);
    }
  }
  return [...byLabel, ...byDescription].slice(0, MAX_RESULTS);
}

export function catalogPath(concrete: string): string {
  const path = concrete.replace(/\[[^\]]*\]/g, "[]");
  const tail = path.startsWith("mcp_servers.") ? path.slice("mcp_servers.".length) : null;
  if (tail === null) return path;
  const dot = tail.indexOf(".");
  return dot < 0 ? "mcp_servers.*" : `mcp_servers.*.${tail.slice(dot + 1)}`;
}

export interface ListItemRef {
  list: "models" | "mcp_servers";
  id: string;
}

export function listItemOf(concrete: string): ListItemRef | null {
  const model = /^models\[([^\]]*)\]/.exec(concrete);
  if (model) return { list: "models", id: model[1] };
  const server = /^mcp_servers\.([^.]+)/.exec(concrete);
  if (server) return { list: "mcp_servers", id: server[1] };
  return null;
}
