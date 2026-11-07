import test from "node:test";
import assert from "node:assert";
import { modelUses, quickActionsModelId, sameAsChatLabel } from "../src/lib/utils/modelUses.ts";

function surfaces({ chat = null, quick = null, title = null, agent = null, speech = null } = {}) {
  return {
    chat: { generation: { model_id: chat } },
    quick_actions: { generation: { model_id: quick } },
    title_generation: { generation: { model_id: title } },
    settings_agent: { generation: { model_id: agent } },
    speech_to_text: { model_id: speech },
  };
}

const surfaceUse = (surface, inherited) => ({ kind: "surface", surface, inherited });

test("explicit ids give non-inherited uses", () => {
  const uses = modelUses(surfaces({ chat: "a", quick: "b", speech: "s" }), []);
  assert.deepStrictEqual(uses.get("a")?.slice(0, 1), [surfaceUse("chat", false)]);
  assert.deepStrictEqual(uses.get("b"), [surfaceUse("quick_actions", false)]);
  assert.deepStrictEqual(uses.get("s"), [surfaceUse("speech_to_text", false)]);
});

test("empty surfaces inherit the chat model in order", () => {
  const expected = [
    surfaceUse("chat", false),
    surfaceUse("quick_actions", true),
    surfaceUse("title_generation", true),
    surfaceUse("settings_agent", true),
  ];
  assert.deepStrictEqual(modelUses(surfaces({ chat: "a" }), []).get("a"), expected);
  assert.deepStrictEqual(
    modelUses(surfaces({ chat: "a", quick: "", title: "", agent: "" }), []).get("a"),
    expected,
  );
});

test("empty chat gives no inherited uses", () => {
  assert.strictEqual(modelUses(surfaces(), []).size, 0);
  assert.strictEqual(modelUses(surfaces({ chat: "" }), []).size, 0);
});

test("skill pin follows the surface uses", () => {
  const skills = [
    { name: "translate", model: "b" },
    { name: "plain", model: null },
  ];
  const uses = modelUses(surfaces({ chat: "a", quick: "b" }), skills);
  assert.deepStrictEqual(uses.get("b"), [
    surfaceUse("quick_actions", false),
    { kind: "skill", skill: "translate" },
  ]);
  assert.strictEqual(
    [...uses.values()].flat().some((u) => u.kind === "skill" && u.skill === "plain"),
    false,
  );
});

test("unused model has no entry", () => {
  assert.strictEqual(modelUses(surfaces({ chat: "a" }), []).get("z"), undefined);
});

test("quickActionsModelId prefers the stored id", () => {
  assert.strictEqual(quickActionsModelId(surfaces({ chat: "a", quick: "b" })), "b");
});

test("quickActionsModelId falls back to the chat id", () => {
  assert.strictEqual(quickActionsModelId(surfaces({ chat: "a" })), "a");
  assert.strictEqual(quickActionsModelId(surfaces({ chat: "a", quick: "" })), "a");
});

test("quickActionsModelId is null when both are empty", () => {
  assert.strictEqual(quickActionsModelId(surfaces()), null);
});

const models = [
  { id: "a", model: "gpt-a", display_name: "Model A" },
  { id: "c", model: "gpt-c", display_name: "" },
];

test("sameAsChatLabel names the chat model", () => {
  assert.strictEqual(sameAsChatLabel(surfaces({ chat: "a" }), models), "Same as Chat (Model A)");
});

test("sameAsChatLabel falls back to the model field", () => {
  assert.strictEqual(sameAsChatLabel(surfaces({ chat: "c" }), models), "Same as Chat (gpt-c)");
});

test("sameAsChatLabel without a chat model", () => {
  assert.strictEqual(sameAsChatLabel(surfaces(), models), "Same as Chat");
  assert.strictEqual(sameAsChatLabel(surfaces({ chat: "zzz" }), models), "Same as Chat");
});
