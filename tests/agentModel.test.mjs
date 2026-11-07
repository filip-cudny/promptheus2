import test from "node:test";
import assert from "node:assert";
import {
  agentModelId,
  agentModelName,
  agentModelHasBuiltinSearch,
} from "../src/lib/utils/agentModel.ts";

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

const searchModels = [
  { id: "oai", provider: "openai", api_mode: "responses" },
  { id: "oai-default-mode", provider: "openai", api_mode: null },
  { id: "unset-provider", provider: null, api_mode: "responses" },
  { id: "oai-completions", provider: "openai", api_mode: "completions" },
  { id: "claude", provider: "anthropic", api_mode: null },
];

test("built-in search: openai provider in responses mode is eligible", () => {
  assert.strictEqual(agentModelHasBuiltinSearch(surfaces("oai", null), searchModels), true);
});

test("built-in search: unset api_mode is eligible", () => {
  assert.strictEqual(
    agentModelHasBuiltinSearch(surfaces("oai-default-mode", null), searchModels),
    true,
  );
});

test("built-in search: unset provider is eligible", () => {
  assert.strictEqual(
    agentModelHasBuiltinSearch(surfaces("unset-provider", null), searchModels),
    true,
  );
});

test("built-in search: completions mode is not eligible", () => {
  assert.strictEqual(
    agentModelHasBuiltinSearch(surfaces("oai-completions", null), searchModels),
    false,
  );
});

test("built-in search: anthropic provider is not eligible", () => {
  assert.strictEqual(agentModelHasBuiltinSearch(surfaces("claude", null), searchModels), false);
});

test("built-in search: agent id missing from models is not eligible", () => {
  assert.strictEqual(agentModelHasBuiltinSearch(surfaces("zzz", "oai"), searchModels), false);
});

test("built-in search: no agent and no chat model is not eligible", () => {
  assert.strictEqual(agentModelHasBuiltinSearch(surfaces(null, null), searchModels), false);
});

test("built-in search: agent model null follows the chat model", () => {
  assert.strictEqual(agentModelHasBuiltinSearch(surfaces(null, "oai"), searchModels), true);
  assert.strictEqual(agentModelHasBuiltinSearch(surfaces(null, "claude"), searchModels), false);
});
