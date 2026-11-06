import test from "node:test";
import assert from "node:assert";
import { hasToolCallMarkers, parseContentSegments, toRenderBlocks } from "../src/lib/utils/toolCallMarkers.ts";

function blocksOf(content) {
  return toRenderBlocks(parseContentSegments(content));
}

test("text only is one text segment and one text block", () => {
  assert.deepStrictEqual(parseContentSegments("hello"), [{ type: "text", text: "hello" }]);
  assert.deepStrictEqual(blocksOf("hello"), [{ kind: "text", text: "hello" }]);
  assert.strictEqual(hasToolCallMarkers("hello"), false);
});

test("markers are detected", () => {
  assert.strictEqual(hasToolCallMarkers("a {{tool_call:x}} b"), true);
});

test("adjacent markers form one tool group", () => {
  assert.deepStrictEqual(blocksOf("{{tool_call:a}}{{tool_call:b}}"), [
    { kind: "tool_group", toolCallIds: ["a", "b"] },
  ]);
});

test("markers separated by whitespace only form one tool group and no text block", () => {
  assert.deepStrictEqual(blocksOf("{{tool_call:a}}\n\n{{tool_call:b}}"), [
    { kind: "tool_group", toolCallIds: ["a", "b"] },
  ]);
});

test("text glued to markers follows the tool group", () => {
  assert.deepStrictEqual(blocksOf("{{tool_call:a}}{{tool_call:b}}W aktualnej"), [
    { kind: "tool_group", toolCallIds: ["a", "b"] },
    { kind: "text", text: "W aktualnej" },
  ]);
});

test("text, marker, text keep their order", () => {
  assert.deepStrictEqual(blocksOf("before {{tool_call:a}} after"), [
    { kind: "text", text: "before " },
    { kind: "tool_group", toolCallIds: ["a"] },
    { kind: "text", text: " after" },
  ]);
});
