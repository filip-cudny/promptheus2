export function useAutoScroll(opts: {
  getContainer: () => HTMLDivElement | undefined;
  pageSize: number;
  totalCount: () => number;
  trackChange: () => unknown;
  resetKey: () => unknown;
}) {
  let userScrolledUp = $state(false);
  let visibleCount = $state(opts.pageSize);

  const hasMore = $derived(visibleCount < opts.totalCount());

  $effect(() => {
    opts.resetKey();
    visibleCount = opts.pageSize;
  });

  $effect(() => {
    opts.trackChange();
    const el = opts.getContainer();
    if (!userScrolledUp && el) {
      requestAnimationFrame(() => {
        el.scrollTop = el.scrollHeight;
      });
    }
  });

  $effect(() => {
    if (opts.totalCount() > visibleCount) {
      visibleCount = Math.max(visibleCount, opts.pageSize);
    }
  });

  function loadMore() {
    const el = opts.getContainer();
    if (!el || !hasMore) return;
    const prevHeight = el.scrollHeight;
    visibleCount = Math.min(visibleCount + opts.pageSize, opts.totalCount());
    requestAnimationFrame(() => {
      const cur = opts.getContainer();
      if (cur) cur.scrollTop += cur.scrollHeight - prevHeight;
    });
  }

  function onScroll() {
    const el = opts.getContainer();
    if (!el) return;
    const distanceFromBottom =
      el.scrollHeight - el.scrollTop - el.clientHeight;
    userScrolledUp = distanceFromBottom > 50;
    if (el.scrollTop < 100 && hasMore) loadMore();
  }

  return {
    get visibleCount() {
      return visibleCount;
    },
    get hasMore() {
      return hasMore;
    },
    onScroll,
  };
}

export type AutoScroll = ReturnType<typeof useAutoScroll>;
