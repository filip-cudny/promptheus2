import test from "node:test";
import assert from "node:assert/strict";
import { createDigitSelector } from "../src/lib/utils/digitSelect.ts";

function setup(count, delay = 200) {
  const s = { count, delay, runs: [], timers: [], cleared: 0 };
  s.sel = createDigitSelector({
    getItemCount: () => s.count,
    getDelayMs: () => s.delay,
    run: (n) => s.runs.push(n),
    setTimer: (fn, ms) => (s.timers.push({ fn, ms }), s.timers.length),
    clearTimer: () => s.cleared++,
  });
  return s;
}

test("no longer number possible runs at once", () => {
  const s = setup(8);
  s.sel.push("3");
  assert.deepEqual(s.runs, [3]);
  assert.equal(s.timers.length, 0);
});

test("second digit before the timer runs the two-digit number", () => {
  const s = setup(12);
  s.sel.push("1");
  assert.deepEqual(s.runs, []);
  assert.equal(s.timers[0].ms, 200);
  s.sel.push("2");
  assert.deepEqual(s.runs, [12]);
});

test("timer firing runs the single digit", () => {
  const s = setup(12);
  s.sel.push("1");
  s.timers[0].fn();
  assert.deepEqual(s.runs, [1]);
});

test("digit with no possible second digit runs at once", () => {
  const s = setup(12);
  s.sel.push("3");
  assert.deepEqual(s.runs, [3]);
});

test("delay change applies to the next timer", () => {
  const s = setup(1000);
  s.sel.push("1");
  s.delay = 500;
  s.sel.push("2");
  assert.equal(s.timers[0].ms, 200);
  assert.equal(s.timers[1].ms, 500);
});
