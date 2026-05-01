export function useElapsedTimer(opts: {
  isActive: () => boolean;
  startedAt: () => string | null | undefined;
  completedAt: () => string | null | undefined;
}) {
  let elapsed = $state(0);

  $effect(() => {
    const startedAt = opts.startedAt();
    if (opts.isActive() && startedAt) {
      const start = new Date(startedAt).getTime();
      elapsed = (Date.now() - start) / 1000;
      const id = setInterval(() => {
        elapsed = (Date.now() - start) / 1000;
      }, 100);
      return () => clearInterval(id);
    }
    const completedAt = opts.completedAt();
    if (startedAt && completedAt) {
      elapsed =
        (new Date(completedAt).getTime() - new Date(startedAt).getTime()) /
        1000;
    }
  });

  return {
    get elapsed() {
      return elapsed;
    },
  };
}

export type ElapsedTimer = ReturnType<typeof useElapsedTimer>;
