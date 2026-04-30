export type ListNavOptions = {
  wrap?: boolean;
};

export function handleListNavKey(
  e: KeyboardEvent,
  currentIndex: number,
  total: number,
  options: ListNavOptions = {},
): number | null {
  if (total <= 0) return null;

  const mod = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  const isDown = e.key === "ArrowDown" || (mod && key === "j");
  const isUp = e.key === "ArrowUp" || (mod && key === "k");

  if (!isDown && !isUp) return null;

  const delta = isDown ? 1 : -1;
  const next = currentIndex + delta;

  if (options.wrap) {
    return ((next % total) + total) % total;
  }
  return Math.max(0, Math.min(total - 1, next));
}
