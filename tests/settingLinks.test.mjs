import test from "node:test";
import assert from "node:assert";
import { parseSettingLink, showSettingLinks } from "../src/lib/utils/settingLinks.ts";

test("setting link yields its key", () => {
  assert.strictEqual(parseSettingLink("setting:models[gpt].display_name"), "models[gpt].display_name");
});

test("other links are not setting links", () => {
  assert.strictEqual(parseSettingLink("https://x"), null);
  assert.strictEqual(parseSettingLink("setting:"), null);
});

test("setting links are relabelled show, other links stay", () => {
  assert.strictEqual(
    showSettingLinks("Theme [here](setting:theme) and [x](https://x)"),
    "Theme [show](setting:theme) and [x](https://x)",
  );
});
