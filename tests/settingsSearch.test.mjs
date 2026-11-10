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

const typoEntries = [
  { key: "theme", section: "general", label: "Theme", description: "Appearance of the app." },
  { key: "model", section: "models", label: "Model", description: "Used for answers." },
  { key: "context_menu.shortcut", section: "context_menu", label: "Context menu shortcut", description: "Opens the menu." },
  { key: "speech.language", section: "speech", label: "Speech language", description: "Spoken tongue." },
];

test("a typo in a query word still finds the entry", () => {
  assert.equal(searchCatalog(typoEntries, "theem")[0].key, "theme");
  assert.equal(searchCatalog(typoEntries, "modeel")[0].key, "model");
  assert.equal(searchCatalog(typoEntries, "mdel")[0].key, "model");
  assert.equal(searchCatalog(typoEntries, "shrotcut")[0].key, "context_menu.shortcut");
});

test("a query word shorter than 4 characters gets no typo match", () => {
  assert.deepEqual(searchCatalog(typoEntries, "thm"), []);
  assert.deepEqual(searchCatalog(typoEntries, "mdl"), []);
});

test("exact label, exact description, typo label and typo description hits rank in that order", () => {
  const ranked = [
    { key: "typo_description", section: "general", label: "Window", description: "Colur of the title." },
    { key: "typo_label", section: "general", label: "Colur picker", description: "Pick one." },
    { key: "exact_description", section: "general", label: "Theme", description: "Color of the windows." },
    { key: "exact_label", section: "general", label: "Color scheme", description: "Scheme." },
  ];
  assert.deepEqual(
    searchCatalog(ranked, "color").map((e) => e.key),
    ["exact_label", "exact_description", "typo_label", "typo_description"],
  );
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
