import { createScrollFollower } from "$lib/utils/scrollFollow";

export function useAutoScroll(opts: {
  getContainer: () => HTMLDivElement | undefined;
  pageSize: number;
  totalCount: () => number;
  trackChange: () => unknown;
  resetKey: () => unknown;
  pinRequest: () => number;
  resetRequest: () => number;
}) {
  let spacer = $state(false);
  let visibleCount = $state(opts.pageSize);
  let handledPin = opts.pinRequest();
  let handledReset = opts.resetRequest();

  const hasMore = $derived(visibleCount < opts.totalCount());

  const follower = createScrollFollower({
    getContainer: opts.getContainer,
    getPinnedTarget: () =>
      opts.getContainer()?.querySelector<HTMLElement>(".pair:last-child > :first-child"),
    onChange: (state) => {
      spacer = state.spacer;
    },
  });

  $effect(() => {
    opts.resetKey();
    visibleCount = opts.pageSize;
    follower.dispatch({ type: "reset" });
  });

  $effect(() => {
    const request = opts.resetRequest();
    if (request === handledReset) return;
    handledReset = request;
    follower.dispatch({ type: "reset" });
  });

  $effect(() => {
    const request = opts.pinRequest();
    if (request === handledPin) return;
    handledPin = request;
    follower.dispatch({ type: "pin" });
  });

  $effect(() => {
    opts.trackChange();
    follower.dispatch({ type: "contentChange" });
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
    follower.onScroll();
    if (el.scrollTop < 100 && hasMore) loadMore();
  }

  return {
    get visibleCount() {
      return visibleCount;
    },
    get hasMore() {
      return hasMore;
    },
    get spacer() {
      return spacer;
    },
    onScroll,
    onWheel: follower.onWheel,
    onKeydown: follower.onKeydown,
    onPointerdown: follower.onPointerdown,
  };
}

export type AutoScroll = ReturnType<typeof useAutoScroll>;
