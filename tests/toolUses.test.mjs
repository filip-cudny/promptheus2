import test from "node:test";
import assert from "node:assert";
import { toolUses } from "../src/lib/utils/toolUses.ts";

function surfaces({ chat = [], agent = [] } = {}) {
  return {
    chat: { generation: { enabled_tools: chat } },
    settings_agent: { generation: { enabled_tools: agent } },
  };
}

test("exact match tags both surfaces in order", () => {
  assert.deepStrictEqual(
    toolUses("srv", "echo", surfaces({ chat: ["srv.echo"], agent: ["srv.echo"] })),
    ["chat", "settings_agent"],
  );
});

test("exact match tags only the surface that holds it", () => {
  assert.deepStrictEqual(toolUses("srv", "echo", surfaces({ agent: ["srv.echo"] })), [
    "settings_agent",
  ]);
  assert.deepStrictEqual(toolUses("srv", "echo", surfaces({ chat: ["srv.echo"] })), ["chat"]);
});

test("bare web_search entry does not tag the MCP tool web_search", () => {
  assert.deepStrictEqual(toolUses("srv", "web_search", surfaces({ agent: ["web_search"] })), []);
});

test("server srv is not tagged by srv2.echo nor srv.echo2", () => {
  const s = surfaces({ chat: ["srv2.echo", "srv.echo2"], agent: ["srv2.echo", "srv.echo2"] });
  assert.deepStrictEqual(toolUses("srv", "echo", s), []);
});

test("no match gives an empty list", () => {
  assert.deepStrictEqual(toolUses("srv", "echo", surfaces()), []);
});
