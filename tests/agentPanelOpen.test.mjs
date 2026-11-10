import test from "node:test";
import assert from "node:assert";
import { initialPanelOpen, panelOpenWrite } from "../src/lib/utils/agentPanelOpen.ts";

test("widened and overlay open the panel initially", () => {
  assert.strictEqual(initialPanelOpen("widened"), true);
  assert.strictEqual(initialPanelOpen("overlay"), true);
});

test("hidden and anything else keep the panel closed initially", () => {
  assert.strictEqual(initialPanelOpen("hidden"), false);
  assert.strictEqual(initialPanelOpen(undefined), false);
  assert.strictEqual(initialPanelOpen(null), false);
  assert.strictEqual(initialPanelOpen(true), false);
  assert.strictEqual(initialPanelOpen("other"), false);
});

test("hide writes false", () => {
  assert.strictEqual(panelOpenWrite("hide"), false);
});

test("reopen and search-row write true", () => {
  assert.strictEqual(panelOpenWrite("reopen"), true);
  assert.strictEqual(panelOpenWrite("search-row"), true);
});

test("show-link writes nothing", () => {
  assert.strictEqual(panelOpenWrite("show-link"), null);
});
