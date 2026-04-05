function findScrollParent(el: HTMLElement): HTMLElement | null {
  let parent = el.parentElement;
  while (parent) {
    const { overflowY } = getComputedStyle(parent);
    if (overflowY === "auto" || overflowY === "scroll") return parent;
    parent = parent.parentElement;
  }
  return null;
}

export function resizeTextarea(textarea: HTMLTextAreaElement) {
  const scrollParent = findScrollParent(textarea);
  const prevScrollTop = scrollParent?.scrollTop ?? 0;

  textarea.style.height = "auto";
  textarea.style.height = textarea.scrollHeight + "px";

  if (scrollParent) {
    scrollParent.scrollTop = prevScrollTop;
    const textareaBottom = textarea.offsetTop + textarea.offsetHeight;
    const visibleBottom = scrollParent.scrollTop + scrollParent.clientHeight;
    if (textareaBottom > visibleBottom) {
      scrollParent.scrollTop = textareaBottom - scrollParent.clientHeight;
    }
  }
}

export function autoResize(node: HTMLTextAreaElement, maxHeight: string) {
  node.style.maxHeight = maxHeight;
  node.style.resize = "none";
  node.style.overflowY = "auto";

  function resize() {
    resizeTextarea(node);
  }

  node.addEventListener("input", resize);
  resize();
  requestAnimationFrame(resize);

  return {
    destroy() {
      node.removeEventListener("input", resize);
    },
  };
}
