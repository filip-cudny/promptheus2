interface DigitSelectorDeps<T> {
  getItemCount: () => number;
  getDelayMs: () => number;
  run: (num: number) => void;
  setTimer: (fn: () => void, ms: number) => T;
  clearTimer: (timer: T) => void;
}

export function createDigitSelector<T>(deps: DigitSelectorDeps<T>) {
  let buffer = "";
  let timer: T | null = null;

  function clearPending() {
    if (timer !== null) deps.clearTimer(timer);
    timer = null;
  }

  function fire() {
    const num = Number(buffer);
    buffer = "";
    timer = null;
    deps.run(num);
  }

  return {
    push(digit: string) {
      clearPending();
      buffer += digit;
      if (Number(buffer) * 10 > deps.getItemCount()) fire();
      else timer = deps.setTimer(fire, deps.getDelayMs());
    },
    reset() {
      clearPending();
      buffer = "";
    },
  };
}
