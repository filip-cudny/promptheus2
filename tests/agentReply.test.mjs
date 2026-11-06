import test from "node:test";
import assert from "node:assert";
import { applyStreamEvent, emptyReply, thinkingText } from "../src/lib/utils/agentReply.ts";

function chunk(accumulated, accumulatedThinking) {
  return {
    event: "chunk",
    data: { delta: "", accumulated, thinking_delta: null, accumulated_thinking: accumulatedThinking },
  };
}

function toolCall(id) {
  return {
    tool_call_id: id,
    tool_name: "t",
    tool_display_name: "t",
    tool_type: "custom",
    arguments: {},
    result: null,
    error: null,
    status: "in_progress",
    started_at: null,
    completed_at: null,
  };
}

test("thinking is active until text arrives and stays in the thinking text", () => {
  const reply = emptyReply();
  applyStreamEvent(reply, chunk("", "A"));
  assert.strictEqual(thinkingText(reply), "A");
  assert.strictEqual(reply.isThinking, true);
  applyStreamEvent(reply, chunk("x", "A"));
  assert.strictEqual(reply.isThinking, false);
  assert.strictEqual(thinkingText(reply), "A");
});

test("a tool call closes the iteration and the next thinking is appended", () => {
  const reply = emptyReply();
  applyStreamEvent(reply, chunk("", "A"));
  applyStreamEvent(reply, chunk("x", "A"));
  applyStreamEvent(reply, { event: "tool_call_start", data: { tool_call: toolCall("c1") } });
  applyStreamEvent(reply, chunk("x", "B"));
  assert.strictEqual(thinkingText(reply), "A\n\nB");
  assert.strictEqual(reply.isThinking, true);
  assert.strictEqual(reply.toolCalls.length, 1);
});

test("done does not duplicate the last iteration's thinking", () => {
  const reply = emptyReply();
  applyStreamEvent(reply, chunk("", "A"));
  applyStreamEvent(reply, chunk("x", "A"));
  applyStreamEvent(reply, { event: "tool_call_start", data: { tool_call: toolCall("c1") } });
  applyStreamEvent(reply, chunk("x", "B"));
  applyStreamEvent(reply, {
    event: "done",
    data: { full_text: "xy", full_thinking: "B", prompt_tokens: null, completion_tokens: null },
  });
  assert.strictEqual(thinkingText(reply), "A\n\nB");
  assert.strictEqual(reply.isThinking, false);
  assert.strictEqual(reply.text, "xy");
});

test("tool_call_done with an error marks the call failed", () => {
  const reply = emptyReply();
  applyStreamEvent(reply, { event: "tool_call_start", data: { tool_call: toolCall("c1") } });
  applyStreamEvent(reply, { event: "tool_call_done", data: { tool_call_id: "c1", result: null, error: "boom" } });
  assert.strictEqual(reply.toolCalls[0].status, "failed");
  assert.strictEqual(reply.toolCalls[0].error, "boom");
});
