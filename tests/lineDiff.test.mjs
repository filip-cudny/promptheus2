import test from "node:test";
import assert from "node:assert";
import { lineDiff } from "../src/lib/utils/lineDiff.ts";

test("identical texts are all unchanged", () => {
  const diff = lineDiff("a\nb\nc", "a\nb\nc");
  assert.deepStrictEqual(diff.map((l) => l.kind), ["unchanged", "unchanged", "unchanged"]);
});

test("a changed middle line is one removed and one added line at that position", () => {
  const diff = lineDiff("a\nb\nc", "a\nx\nc");
  assert.deepStrictEqual(diff, [
    { kind: "unchanged", text: "a" },
    { kind: "removed", text: "b" },
    { kind: "added", text: "x" },
    { kind: "unchanged", text: "c" },
  ]);
});

test("an empty before adds every line", () => {
  const diff = lineDiff("", "a\nb");
  assert.deepStrictEqual(diff, [
    { kind: "added", text: "a" },
    { kind: "added", text: "b" },
  ]);
});
