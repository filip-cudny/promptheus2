import test from "node:test";
import assert from "node:assert";
import { actionShortcuts, formatShortcut } from "../src/lib/utils/keymapShortcuts.ts";

test("every shortcut of an action is listed, sorted", () => {
  const keymaps = [
    {
      context: "os == linux",
      bindings: {
        "ctrl+f1": "open_context_menu",
        "ctrl+f2": "speech_to_text_toggle",
        "shift+f1": "speech_to_text_toggle",
      },
    },
  ];
  assert.deepStrictEqual(actionShortcuts(keymaps, "speech_to_text_toggle", "linux"), [
    "ctrl+f2",
    "shift+f1",
  ]);
});

test("groups of the current OS are merged, other OS groups ignored", () => {
  const keymaps = [
    { context: "os == linux", bindings: { "shift+f1": "speech_to_text_toggle" } },
    { context: "os==linux", bindings: { "ctrl+f2": "speech_to_text_toggle" } },
    { context: "os == macos", bindings: { "cmd+f2": "speech_to_text_toggle" } },
  ];
  assert.deepStrictEqual(actionShortcuts(keymaps, "speech_to_text_toggle", "linux"), [
    "ctrl+f2",
    "shift+f1",
  ]);
  assert.deepStrictEqual(actionShortcuts(keymaps, "speech_to_text_toggle", "macos"), ["cmd+f2"]);
});

test("an action without a binding has no shortcuts", () => {
  const keymaps = [{ context: "os == linux", bindings: { "ctrl+f1": "open_context_menu" } }];
  assert.deepStrictEqual(actionShortcuts(keymaps, "clear_context", "linux"), []);
});

test("shortcut segments are capitalised for display", () => {
  assert.strictEqual(formatShortcut("ctrl+f2"), "Ctrl+F2");
  assert.strictEqual(formatShortcut("shift+f1"), "Shift+F1");
  assert.strictEqual(formatShortcut("cmd+shift+a"), "Cmd+Shift+A");
});
