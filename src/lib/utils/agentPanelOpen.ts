export type PanelOpenAction = "hide" | "reopen" | "search-row" | "show-link";

export function initialPanelOpen(value: unknown): boolean {
  return value === "widened" || value === "overlay";
}

export function panelOpenWrite(action: PanelOpenAction): boolean | null {
  switch (action) {
    case "hide":
      return false;
    case "reopen":
    case "search-row":
      return true;
    case "show-link":
      return null;
  }
}
