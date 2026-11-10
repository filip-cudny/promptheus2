import test from "node:test";
import assert from "node:assert";
import { INITIAL_SCROLL_STATE, reduceScroll } from "../src/lib/utils/scrollFollow.ts";

const step = (state, event) => reduceScroll(state, event);

test("a view starts in follow mode without a spacer", () => {
  assert.deepStrictEqual(INITIAL_SCROLL_STATE, { mode: "follow", spacer: false });
});

test("only pin scrolls to the pinned message", () => {
  const events = [
    { type: "reset" },
    { type: "contentChange" },
    { type: "userScroll", atBottom: true },
    { type: "userScroll", atBottom: false },
    { type: "programmaticScroll" },
  ];
  for (const event of events) {
    assert.notStrictEqual(step(INITIAL_SCROLL_STATE, event).action, "toPinned");
  }
  assert.deepStrictEqual(step(INITIAL_SCROLL_STATE, { type: "pin" }), {
    state: { mode: "pinned", spacer: true },
    action: "toPinned",
  });
});

test("content changes while pinned do not scroll", () => {
  const pinned = step(INITIAL_SCROLL_STATE, { type: "pin" }).state;
  assert.deepStrictEqual(step(pinned, { type: "contentChange" }), { state: pinned, action: "none" });
});

test("content changes scroll to the bottom only in follow mode", () => {
  assert.strictEqual(step(INITIAL_SCROLL_STATE, { type: "contentChange" }).action, "toBottom");
  const free = step(INITIAL_SCROLL_STATE, { type: "userScroll", atBottom: false }).state;
  assert.strictEqual(step(free, { type: "contentChange" }).action, "none");
});

test("reaching the bottom by a programmatic scroll does not resume following", () => {
  const pinned = step(INITIAL_SCROLL_STATE, { type: "pin" }).state;
  assert.deepStrictEqual(step(pinned, { type: "programmaticScroll" }), { state: pinned, action: "none" });
  const free = step(pinned, { type: "userScroll", atBottom: false }).state;
  assert.deepStrictEqual(step(free, { type: "programmaticScroll" }), { state: free, action: "none" });
});

test("a user gesture that ends at the bottom resumes following and keeps the spacer", () => {
  const pinned = step(INITIAL_SCROLL_STATE, { type: "pin" }).state;
  const resumed = step(pinned, { type: "userScroll", atBottom: true }).state;
  assert.deepStrictEqual(resumed, { mode: "follow", spacer: true });
  assert.strictEqual(step(resumed, { type: "contentChange" }).action, "toBottom");
});

test("a user gesture away from the bottom stops following", () => {
  assert.strictEqual(step(INITIAL_SCROLL_STATE, { type: "userScroll", atBottom: false }).state.mode, "free");
});

test("reset scrolls to the bottom and drops the spacer", () => {
  const pinned = step(INITIAL_SCROLL_STATE, { type: "pin" }).state;
  assert.deepStrictEqual(step(pinned, { type: "reset" }), {
    state: { mode: "follow", spacer: false },
    action: "toBottom",
  });
});
