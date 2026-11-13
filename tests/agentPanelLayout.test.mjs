import test from "node:test";
import assert from "node:assert";
import {
  CONTENT_MIN_WIDTH,
  PANEL_MIN_WIDTH,
  agentPanelLayout,
  clampPanelWidth,
  sidebarLead,
} from "../src/lib/utils/agentPanelLayout.ts";

const DOCK_AT = CONTENT_MIN_WIDTH + 300;

test("minimum panel width is 300", () => {
  assert.strictEqual(PANEL_MIN_WIDTH, 300);
});

test("below the dock threshold the panel overlays", () => {
  const container = DOCK_AT - 1;
  assert.deepStrictEqual(agentPanelLayout(container, 200), { docked: false, width: 300 });
  assert.deepStrictEqual(agentPanelLayout(container, 400), { docked: false, width: 400 });
  assert.deepStrictEqual(agentPanelLayout(container, container + 500), {
    docked: false,
    width: container,
  });
});

test("exactly at the dock threshold the panel docks at its minimum", () => {
  assert.deepStrictEqual(agentPanelLayout(DOCK_AT, 200), { docked: true, width: 300 });
  assert.deepStrictEqual(agentPanelLayout(DOCK_AT, 400), { docked: true, width: 300 });
  assert.deepStrictEqual(agentPanelLayout(DOCK_AT, DOCK_AT + 500), { docked: true, width: 300 });
});

test("above the dock threshold the panel docks and leaves the content its minimum", () => {
  const container = DOCK_AT + 200;
  assert.deepStrictEqual(agentPanelLayout(container, 200), { docked: true, width: 300 });
  assert.deepStrictEqual(agentPanelLayout(container, 400), { docked: true, width: 400 });
  assert.deepStrictEqual(agentPanelLayout(container, container + 500), {
    docked: true,
    width: container - CONTENT_MIN_WIDTH,
  });
});

test("drag clamp is [300, container - content minimum] docked and [300, container] overlaid", () => {
  assert.strictEqual(clampPanelWidth(DOCK_AT + 200, 100), 300);
  assert.strictEqual(clampPanelWidth(DOCK_AT + 200, 10_000), 500);
  assert.strictEqual(clampPanelWidth(DOCK_AT - 1, 10_000), DOCK_AT - 1);
  assert.strictEqual(clampPanelWidth(DOCK_AT - 1, 100), 300);
});

test("sidebar lead is half the free width, shrunk first by the docked panel", () => {
  assert.strictEqual(sidebarLead(2560, 0), 650);
  assert.strictEqual(sidebarLead(2560, 380), 650);
  assert.strictEqual(sidebarLead(1920, 0), 330);
  assert.strictEqual(sidebarLead(1920, 380), 280);
});

test("sidebar lead is never negative", () => {
  assert.strictEqual(sidebarLead(1340, 380), 0);
  assert.strictEqual(sidebarLead(1100, 0), 0);
});
