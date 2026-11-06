import test from "node:test";
import assert from "node:assert";
import { agentModelId, agentModelName } from "../src/lib/utils/agentModel.ts";

const models = [
  { id: "a", model: "gpt-a", display_name: "Model A" },
  { id: "b", model: "gpt-b", display_name: "Model B" },
  { id: "c", model: "gpt-c", display_name: "" },
];

function surfaces(agentId, chatId) {
  return {
    chat: { generation: { model_id: chatId } },
    settings_agent: { generation: { model_id: agentId } },
  };
}

test("agent model id set resolves to its name", () => {
  assert.strictEqual(agentModelId(surfaces("a", "b")), "a");
  assert.strictEqual(agentModelName(surfaces("a", "b"), models), "Model A");
});

test("agent model id null falls back to the chat model", () => {
  assert.strictEqual(agentModelId(surfaces(null, "b")), "b");
  assert.strictEqual(agentModelName(surfaces(null, "b"), models), "Model B");
});

test("both ids null gives No model", () => {
  assert.strictEqual(agentModelId(surfaces(null, null)), null);
  assert.strictEqual(agentModelName(surfaces(null, null), models), "No model");
});

test("agent id missing from models gives No model", () => {
  assert.strictEqual(agentModelName(surfaces("zzz", "b"), models), "No model");
});

test("empty display_name falls back to the model field", () => {
  assert.strictEqual(agentModelName(surfaces("c", null), models), "gpt-c");
});
