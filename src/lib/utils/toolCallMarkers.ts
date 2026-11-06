import type { ContentSegment } from "$lib/types/conversation";

const TOOL_CALL_MARKER_PATTERN = /\{\{tool_call:([^}]+)\}\}/g;
const TOOL_CALL_MARKER_TEST = /\{\{tool_call:[^}]+\}\}/;

export type RenderBlock =
  | { kind: "text"; text: string }
  | { kind: "tool_group"; toolCallIds: string[] };

export function hasToolCallMarkers(content: string): boolean {
  return TOOL_CALL_MARKER_TEST.test(content);
}

export function parseContentSegments(content: string): ContentSegment[] {
  const segments: ContentSegment[] = [];
  let lastIndex = 0;

  for (const match of content.matchAll(TOOL_CALL_MARKER_PATTERN)) {
    if (match.index > lastIndex) {
      segments.push({ type: "text", text: content.slice(lastIndex, match.index) });
    }
    segments.push({ type: "tool_call", tool_call_id: match[1] });
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < content.length) {
    segments.push({ type: "text", text: content.slice(lastIndex) });
  }

  return segments;
}

export function toRenderBlocks(segments: ContentSegment[]): RenderBlock[] {
  const blocks: RenderBlock[] = [];
  for (const seg of segments) {
    if (seg.type === "text" && seg.text.trim()) {
      blocks.push({ kind: "text", text: seg.text });
    } else if (seg.type === "tool_call") {
      const last = blocks[blocks.length - 1];
      if (last && last.kind === "tool_group") {
        last.toolCallIds.push(seg.tool_call_id);
      } else {
        blocks.push({ kind: "tool_group", toolCallIds: [seg.tool_call_id] });
      }
    }
  }
  return blocks;
}
