import test from "node:test";
import assert from "node:assert/strict";
import { searchCatalog, catalogPath, listItemOf } from "../src/lib/utils/settingsSearch.ts";

const entries = [
  { key: "theme", section: "general", label: "Theme", description: "Color scheme of the app windows." },
  { key: "history_limit", section: "history", label: "History limit", description: "Maximum stored conversations." },
  { key: "autosave_debounce_ms", section: "general", label: "Autosave delay", description: "Wait before a theme edit is saved." },
];

test("a word of a description finds its entry", () => {
  assert.deepEqual(searchCatalog(entries, "conversations").map((e) => e.key), ["history_limit"]);
});

test("two words must both match", () => {
  assert.deepEqual(searchCatalog(entries, "maximum conv").map((e) => e.key), ["history_limit"]);
  assert.deepEqual(searchCatalog(entries, "maximum theme"), []);
});

test("label match ranks before description-only match", () => {
  assert.deepEqual(searchCatalog(entries, "theme").map((e) => e.key), ["theme", "autosave_debounce_ms"]);
});

test("catalogPath normalizes concrete paths", () => {
  assert.equal(catalogPath("models[abc].model"), "models[].model");
  assert.equal(catalogPath("mcp_servers.x.env"), "mcp_servers.*.env");
});

test("listItemOf names the list item", () => {
  assert.deepEqual(listItemOf("models[abc].api_key"), { list: "models", id: "abc" });
  assert.deepEqual(listItemOf("mcp_servers.x.env"), { list: "mcp_servers", id: "x" });
  assert.equal(listItemOf("theme"), null);
});
