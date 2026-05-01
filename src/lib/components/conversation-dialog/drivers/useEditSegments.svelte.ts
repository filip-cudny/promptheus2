import { resizeTextarea } from "$lib/utils/autoResize";
import type { ContentSegment } from "$lib/types/conversation";

export type EditSegment =
  | { type: "text"; leadingWs: string; text: string; trailingWs: string }
  | { type: "tool_call"; tool_call_id: string };

function splitTextSegment(text: string): {
  leadingWs: string;
  text: string;
  trailingWs: string;
} {
  const leadingWs = text.match(/^\s*/)?.[0] ?? "";
  const rest = text.slice(leadingWs.length);
  const trailingWs = rest.match(/\s*$/)?.[0] ?? "";
  const mid = rest.slice(0, rest.length - trailingWs.length);
  return { leadingWs, text: mid, trailingWs };
}

export function useEditSegments(opts: {
  parseContent: (content: string) => ContentSegment[];
}) {
  let segments = $state<EditSegment[]>([]);
  let textareaRefs: Array<HTMLTextAreaElement | undefined> = $state([]);

  function build(content: string): EditSegment[] {
    const parsed = opts.parseContent(content);
    if (parsed.length === 0) {
      return [{ type: "text", ...splitTextSegment(content) }];
    }
    return parsed.map((s) =>
      s.type === "text"
        ? { type: "text" as const, ...splitTextSegment(s.text) }
        : { type: "tool_call" as const, tool_call_id: s.tool_call_id },
    );
  }

  return {
    get segments() {
      return segments;
    },
    get textareaRefs() {
      return textareaRefs;
    },
    enter(content: string): void {
      segments = build(content);
      requestAnimationFrame(() => {
        for (const ta of textareaRefs) {
          if (ta) resizeTextarea(ta);
        }
      });
    },
    exit(): void {
      segments = [];
      textareaRefs = [];
    },
    rebuild(): string {
      return segments
        .map((s) =>
          s.type === "text"
            ? s.leadingWs + s.text + s.trailingWs
            : `{{tool_call:${s.tool_call_id}}}`,
        )
        .join("");
    },
    onSegmentInput(idx: number, e: Event): void {
      const target = e.target as HTMLTextAreaElement;
      const seg = segments[idx];
      if (seg.type !== "text") return;
      segments[idx] = { ...seg, text: target.value };
      resizeTextarea(target);
    },
  };
}

export type EditSegments = ReturnType<typeof useEditSegments>;
