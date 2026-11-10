export function insertText(
  text: string,
  start: number,
  end: number,
  inserted: string,
): { text: string; caret: number } {
  const from = Math.min(Math.max(start, 0), text.length);
  const to = Math.min(Math.max(end, from), text.length);
  return { text: text.slice(0, from) + inserted + text.slice(to), caret: from + inserted.length };
}
