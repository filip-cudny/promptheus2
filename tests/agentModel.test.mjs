import test from "node:test";
import assert from "node:assert";
import {
  agentModelId,
  agentReasoningEffort,
  agentModelName,
  agentModelHasBuiltinSearch,
  agentHasUsableModel,
  agentModelState,
  useModelWrites,
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

function effortSurfaces(agentEffort, chatEffort) {
  return {
    chat: { generation: { parameters: { reasoning_effort: chatEffort } } },
    settings_agent: { generation: { parameters: { reasoning_effort: agentEffort } } },
  };
}

test("agent effort set overrides the chat effort", () => {
  assert.strictEqual(agentReasoningEffort(effortSurfaces("low", "high")), "low");
});

test("agent effort null falls back to the chat effort", () => {
  assert.strictEqual(agentReasoningEffort(effortSurfaces(null, "high")), "high");
});

test("both efforts null gives null", () => {
  assert.strictEqual(agentReasoningEffort(effortSurfaces(null, null)), null);
});

test("agent effort none overrides the chat effort", () => {
  assert.strictEqual(agentReasoningEffort(effortSurfaces("none", "high")), "none");
});

test("agent effort empty string overrides the chat effort", () => {
  assert.strictEqual(agentReasoningEffort(effortSurfaces("", "high")), "");
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

const usableModels = [
  { id: "t1", type: "text" },
  { id: "t2", type: "text" },
  { id: "stt", type: "stt" },
];

test("usable model: settings agent id is a text model", () => {
  assert.strictEqual(agentHasUsableModel(surfaces("t1", "t2"), usableModels), true);
});

test("usable model: chat id only is a text model", () => {
  assert.strictEqual(agentHasUsableModel(surfaces(null, "t2"), usableModels), true);
});

test("usable model: settings agent id missing from models is not usable", () => {
  assert.strictEqual(agentHasUsableModel(surfaces("zzz", "t1"), usableModels), false);
});

test("usable model: non-text model is not usable", () => {
  assert.strictEqual(agentHasUsableModel(surfaces("stt", "t1"), usableModels), false);
});

test("usable model: no ids is not usable", () => {
  assert.strictEqual(agentHasUsableModel(surfaces(null, null), usableModels), false);
});

test("model state: no text model", () => {
  assert.strictEqual(agentModelState(surfaces(null, null), []), "no_text_model");
  assert.strictEqual(
    agentModelState(surfaces(null, null), [{ id: "stt", type: "stt" }]),
    "no_text_model",
  );
});

test("model state: effective id is a text model", () => {
  assert.strictEqual(agentModelState(surfaces("t1", null), usableModels), "ok");
  assert.strictEqual(agentModelState(surfaces(null, "t2"), usableModels), "ok");
});

test("model state: no ids gives no usable model", () => {
  assert.strictEqual(agentModelState(surfaces(null, null), usableModels), "no_usable_model");
});

test("model state: stale chat id only gives no usable model", () => {
  assert.strictEqual(agentModelState(surfaces(null, "zzz"), usableModels), "no_usable_model");
});

test("model state: agent id missing from models is stale", () => {
  assert.strictEqual(agentModelState(surfaces("zzz", "t1"), usableModels), "stale_agent_model");
});

test("model state: agent id of a non-text model gives no usable model", () => {
  assert.strictEqual(agentModelState(surfaces("stt", "t1"), usableModels), "no_usable_model");
});

function applyWrites(current, writes) {
  const next = structuredClone(current);
  for (const w of writes) next[w.surface].generation.model_id = w.modelId;
  return next;
}

test("use writes: no agent id writes only the chat model", () => {
  assert.deepStrictEqual(useModelWrites(surfaces(null, null), usableModels, "t1"), [
    { surface: "chat", modelId: "t1" },
  ]);
});

test("use writes: stale agent id is cleared before the chat write", () => {
  assert.deepStrictEqual(useModelWrites(surfaces("zzz", null), usableModels, "t1"), [
    { surface: "settings_agent", modelId: null },
    { surface: "chat", modelId: "t1" },
  ]);
});

test("use writes: applied to a stale agent id the state is ok", () => {
  const stale = surfaces("zzz", "yyy");
  const next = applyWrites(stale, useModelWrites(stale, usableModels, "t2"));
  assert.strictEqual(agentModelState(next, usableModels), "ok");
  assert.strictEqual(agentModelId(next), "t2");
});

test("use writes: applied to a non-text agent id the state is ok", () => {
  const nonText = surfaces("stt", null);
  const next = applyWrites(nonText, useModelWrites(nonText, usableModels, "t1"));
  assert.strictEqual(agentModelState(next, usableModels), "ok");
});

test("use writes: applied with no agent id the state is ok", () => {
  const none = surfaces(null, null);
  const next = applyWrites(none, useModelWrites(none, usableModels, "t1"));
  assert.strictEqual(agentModelState(next, usableModels), "ok");
});
