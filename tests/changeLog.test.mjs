import test from "node:test";
import assert from "node:assert";
import { entryTitle, conflictLines } from "../src/lib/utils/changeLog.ts";

const card = { id: "a", created_at: "T1", host: "h", kind: "card", operations: [{ label: "Rename model" }], undoes: null, secret: null };
const undo = { id: "b", created_at: "T2", host: "h", kind: "undo", operations: [], undoes: "a", secret: null };

test("an undo entry's title names the undone entry's time", () => {
  assert.strictEqual(entryTitle(undo, [card, undo], (t) => `at ${t}`), "Undo of at T1");
});

test("a card title lists its row labels", () => {
  assert.strictEqual(entryTitle(card, [card]), "Rename model");
});

test("a conflict summary lists every target", () => {
  const lines = conflictLines([
    { row: 0, target: { type: "setting", path: "a.b" }, expected_after: "x", current: "y" },
    { row: 1, target: { type: "file", path: "skills/s/SKILL.md" }, expected_after: null, current: "z" },
  ]);
  assert.strictEqual(lines.length, 2);
  assert.ok(lines[0].includes("a.b") && lines[1].includes("skills/s/SKILL.md"));
});
