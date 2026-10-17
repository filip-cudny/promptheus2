export const COPY_FEEDBACK_MS = 1200;

export function useCopyFeedback<T>() {
  let confirmed = $state<T | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  return {
    get confirmed() {
      return confirmed;
    },
    flash(key: T) {
      clearTimeout(timer);
      confirmed = key;
      timer = setTimeout(() => (confirmed = null), COPY_FEEDBACK_MS);
    },
  };
}
