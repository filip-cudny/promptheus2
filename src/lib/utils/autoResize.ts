export function resizeTextarea(textarea: HTMLTextAreaElement) {
  textarea.style.height = "auto";
  textarea.style.height = textarea.scrollHeight + "px";
}

export function autoResize(node: HTMLTextAreaElement, maxHeight: string) {
  node.style.maxHeight = maxHeight;
  node.style.resize = "none";
  node.style.overflowY = "auto";

  function resize() {
    resizeTextarea(node);
  }

  node.addEventListener("input", resize);
  requestAnimationFrame(resize);

  return {
    destroy() {
      node.removeEventListener("input", resize);
    },
  };
}
