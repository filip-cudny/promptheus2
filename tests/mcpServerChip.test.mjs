import test from "node:test";
import assert from "node:assert";
import { serverChip } from "../src/lib/utils/mcpServerChip.ts";

const cases = [
  ["running", false, "running"],
  ["running", true, "stops after restart"],
  ["error", false, "error"],
  ["error", true, "off"],
  ["off", false, "starts after restart"],
  ["off", true, "off"],
  [null, false, "starts after restart"],
  [null, true, "off"],
];

for (const [state, disabled, chip] of cases) {
  test(`state ${state} with disabled ${disabled} gives "${chip}"`, () => {
    assert.strictEqual(serverChip(state, disabled), chip);
  });
}
