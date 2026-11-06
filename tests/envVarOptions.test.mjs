import test from "node:test";
import assert from "node:assert/strict";
import {
  classifyReference,
  insertAt,
  parseEnvRef,
  pickerOptions,
} from "../src/lib/utils/envVarOptions.ts";

const listed = [
  { name: "OPENAI_API_KEY", set: true },
  { name: "ANTHROPIC_API_KEY", set: false },
];
const NEW = { value: "__new__", label: "New…" };

test("parseEnvRef accepts exactly one token", () => {
  assert.equal(parseEnvRef("${A_1}"), "A_1");
  assert.equal(parseEnvRef("${openai_key}"), "openai_key");
  assert.equal(parseEnvRef("${A}/x"), null);
  assert.equal(parseEnvRef("sk-test"), null);
  assert.equal(parseEnvRef(""), null);
});

test("listed reference", () => {
  assert.deepEqual(classifyReference("${OPENAI_API_KEY}", ["OPENAI_API_KEY"]), {
    kind: "listed",
    name: "OPENAI_API_KEY",
  });
  const options = pickerOptions(listed, "${OPENAI_API_KEY}", []);
  assert.deepEqual(options[0], { value: "${OPENAI_API_KEY}", label: "OPENAI_API_KEY" });
  assert.equal(options.length, 2);
});

test("a lowercase listed name is a listed reference, not a literal", () => {
  assert.deepEqual(classifyReference("${openai_key}", ["openai_key"]), {
    kind: "listed",
    name: "openai_key",
  });
  const options = pickerOptions([{ name: "openai_key", set: true }], "${openai_key}", []);
  assert.deepEqual(options, [{ value: "${openai_key}", label: "openai_key" }]);
});

test("listed but missing is labelled", () => {
  const options = pickerOptions(listed, "${ANTHROPIC_API_KEY}", []);
  assert.equal(options[1].label, "ANTHROPIC_API_KEY (missing)");
});

test("unlisted reference", () => {
  assert.deepEqual(classifyReference("${GEMINI_KEY}", ["OPENAI_API_KEY"]), {
    kind: "unlisted",
    name: "GEMINI_KEY",
  });
  const options = pickerOptions([], "${OPENAI_API_KEY}", []);
  assert.deepEqual(options, [
    { value: "${OPENAI_API_KEY}", label: "OPENAI_API_KEY (not listed)" },
  ]);
});

test("literal key", () => {
  assert.deepEqual(classifyReference("sk-test", []), { kind: "literal" });
  const options = pickerOptions(listed, "sk-test", []);
  assert.deepEqual(options.at(-1), { value: "sk-test", label: "Literal key" });
});

test("a token with text around it is a literal", () => {
  assert.deepEqual(classifyReference("${A}/x", ["A"]), { kind: "literal" });
  assert.equal(pickerOptions([], "${A}/x", []).at(-1).label, "Literal key");
});

test("empty value adds no option", () => {
  assert.deepEqual(classifyReference("", ["A"]), { kind: "empty" });
  assert.deepEqual(classifyReference(null, ["A"]), { kind: "empty" });
  assert.equal(pickerOptions(listed, "", []).length, 2);
});

test("extras come last", () => {
  const options = pickerOptions(listed, "sk-test", [NEW]);
  assert.deepEqual(options.at(-1), NEW);
  assert.equal(options.at(-2).label, "Literal key");
});

test("literal option can be left out", () => {
  const options = pickerOptions(listed, "sk-test", [NEW], false);
  assert.deepEqual(options.map((o) => o.label), [
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY (missing)",
    "New…",
  ]);
  assert.equal(pickerOptions(listed, "${GEMINI_KEY}", [], false).at(-1).label, "GEMINI_KEY (not listed)");
});

test("insertAt into an empty string", () => {
  assert.deepEqual(insertAt("", 0, 0, "${A}"), { text: "${A}", caret: 4 });
});

test("insertAt at the start", () => {
  assert.deepEqual(insertAt("/src/index.ts", 0, 0, "${P}"), {
    text: "${P}/src/index.ts",
    caret: 4,
  });
});

test("insertAt in the middle", () => {
  assert.deepEqual(insertAt("ab", 1, 1, "${A}"), { text: "a${A}b", caret: 5 });
});

test("insertAt replaces a selection", () => {
  assert.deepEqual(insertAt("abcd", 1, 3, "${A}"), { text: "a${A}d", caret: 5 });
});
