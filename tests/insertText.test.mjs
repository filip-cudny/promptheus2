import test from "node:test";
import assert from "node:assert";
import { insertText } from "../src/lib/utils/insertText.ts";

test("inserts in the middle and puts the caret after the insert", () => {
  assert.deepStrictEqual(insertText("hello world", 5, 5, ","), { text: "hello, world", caret: 6 });
});

test("inserts at the start", () => {
  assert.deepStrictEqual(insertText("world", 0, 0, "hello "), { text: "hello world", caret: 6 });
});

test("inserts at the end", () => {
  assert.deepStrictEqual(insertText("hello", 5, 5, " world"), { text: "hello world", caret: 11 });
});

test("replaces a selection", () => {
  assert.deepStrictEqual(insertText("hello big world", 6, 9, "small"), {
    text: "hello small world",
    caret: 11,
  });
});

test("clamps offsets past the end", () => {
  assert.deepStrictEqual(insertText("abc", 10, 20, "d"), { text: "abcd", caret: 4 });
  assert.deepStrictEqual(insertText("abc", 1, 20, "d"), { text: "ad", caret: 2 });
});
