let focusFn: (() => void) | null = null;

export function setConversationInputFocus(fn: (() => void) | null): void {
  focusFn = fn;
}

export function focusConversationInput(): void {
  const fn = focusFn;
  if (!fn) return;
  requestAnimationFrame(() => fn());
}
