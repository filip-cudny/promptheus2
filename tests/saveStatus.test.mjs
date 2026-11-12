import test from "node:test";
import assert from "node:assert";
import {
  editorDirtyChanged,
  editorRegistered,
  editorUnregistered,
  emptySaveStatusModel,
  saveStatusState,
  saveStatusTooltip,
  writeFailed,
  writeStarted,
  writeSucceeded,
} from "../src/lib/utils/saveStatus.ts";

const TTL = 2500;
const T0 = 10_000;

const state = (model, now = T0) => saveStatusState(model, now, TTL);
const tooltip = (model, now = T0) => saveStatusTooltip(model, now, TTL);

test("a model without writes or editors is idle", () => {
  assert.strictEqual(state(emptySaveStatusModel()), "idle");
});

test("write start gives saving", () => {
  assert.strictEqual(state(writeStarted(emptySaveStatusModel())), "saving");
});

test("write success gives saved, then idle after the TTL", () => {
  const model = writeSucceeded(writeStarted(emptySaveStatusModel()), T0);
  assert.strictEqual(state(model, T0), "saved");
  assert.strictEqual(state(model, T0 + TTL - 1), "saved");
  assert.strictEqual(state(model, T0 + TTL), "idle");
});

test("write failure gives error until the next success", () => {
  const failed = writeFailed(writeStarted(emptySaveStatusModel()), "boom");
  assert.strictEqual(state(failed), "error");
  assert.strictEqual(state(writeStarted(failed)), "error");
  const recovered = writeSucceeded(writeStarted(failed), T0);
  assert.strictEqual(state(recovered), "saved");
});

test("a registered dirty editor gives dirty", () => {
  const registered = editorRegistered(emptySaveStatusModel(), "e1", 200);
  assert.strictEqual(state(registered), "idle");
  assert.strictEqual(state(editorDirtyChanged(registered, "e1", true)), "dirty");
});

test("an unregistered editor clears its dirty entry", () => {
  const dirty = editorDirtyChanged(
    editorRegistered(emptySaveStatusModel(), "e1", 200),
    "e1",
    true,
  );
  assert.strictEqual(state(editorUnregistered(dirty, "e1")), "idle");
});

test("error wins over saving", () => {
  const model = writeStarted(writeFailed(writeStarted(emptySaveStatusModel()), "boom"));
  assert.strictEqual(model.writesInFlight, 1);
  assert.strictEqual(state(model), "error");
});

test("saving wins over dirty", () => {
  const dirty = editorDirtyChanged(
    editorRegistered(emptySaveStatusModel(), "e1", 200),
    "e1",
    true,
  );
  assert.strictEqual(state(writeStarted(dirty)), "saving");
});

test("dirty wins over saved", () => {
  const saved = writeSucceeded(writeStarted(emptySaveStatusModel()), T0);
  const dirty = editorDirtyChanged(editorRegistered(saved, "e1", 200), "e1", true);
  assert.strictEqual(state(dirty, T0), "dirty");
});

test("an editor going clean without a write gives idle, not saved", () => {
  const dirty = editorDirtyChanged(
    editorRegistered(emptySaveStatusModel(), "e1", 200),
    "e1",
    true,
  );
  assert.strictEqual(state(editorDirtyChanged(dirty, "e1", false)), "idle");
});

test("two writes in flight, one finished, is still saving", () => {
  const two = writeStarted(writeStarted(emptySaveStatusModel()));
  assert.strictEqual(state(writeSucceeded(two, T0)), "saving");
});

test("idle tooltip without a registered editor", () => {
  assert.strictEqual(tooltip(emptySaveStatusModel()), "All changes saved");
});

test("idle tooltip with a registered editor", () => {
  const model = editorRegistered(emptySaveStatusModel(), "e1", 200);
  assert.strictEqual(tooltip(model), "All changes saved (⌘S to flush)");
});

test("tooltips of saving, dirty, saved and error", () => {
  const dirty = editorDirtyChanged(
    editorRegistered(emptySaveStatusModel(), "e1", 350),
    "e1",
    true,
  );
  assert.strictEqual(tooltip(writeStarted(emptySaveStatusModel())), "Saving…");
  assert.strictEqual(tooltip(dirty), "Unsaved — autosaving in 350 ms");
  const saved = writeSucceeded(writeStarted(emptySaveStatusModel()), T0);
  assert.strictEqual(tooltip(saved, T0 + 2000), "Saved 2s ago — autosaves on edit");
  const failed = writeFailed(writeStarted(emptySaveStatusModel()), "boom");
  assert.strictEqual(tooltip(failed), "Failed to save: boom");
});
