type Theme = "dark" | "light";

let theme = $state<Theme>("dark");

export function getTheme() {
  return theme;
}

export function setTheme(t: Theme) {
  theme = t;
  document.documentElement.setAttribute("data-theme", t);
  localStorage.setItem("theme", t);
}

export async function initTheme() {
  const saved = localStorage.getItem("theme") as Theme | null;
  if (saved === "light" || saved === "dark") setTheme(saved);
}
