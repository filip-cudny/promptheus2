export type DiffKind = "added" | "removed" | "unchanged";

export interface DiffLine {
  kind: DiffKind;
  text: string;
}

function lines(text: string): string[] {
  return text === "" ? [] : text.split("\n");
}

export function lineDiff(before: string, after: string): DiffLine[] {
  const a = lines(before);
  const b = lines(after);
  const common: number[][] = Array.from({ length: a.length + 1 }, () => new Array(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      common[i][j] = a[i] === b[j] ? common[i + 1][j + 1] + 1 : Math.max(common[i + 1][j], common[i][j + 1]);
    }
  }
  const result: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      result.push({ kind: "unchanged", text: a[i] });
      i++;
      j++;
    } else if (common[i + 1][j] >= common[i][j + 1]) {
      result.push({ kind: "removed", text: a[i++] });
    } else {
      result.push({ kind: "added", text: b[j++] });
    }
  }
  while (i < a.length) result.push({ kind: "removed", text: a[i++] });
  while (j < b.length) result.push({ kind: "added", text: b[j++] });
  return result;
}
