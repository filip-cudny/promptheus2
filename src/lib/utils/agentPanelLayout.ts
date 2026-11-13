export const PANEL_MIN_WIDTH = 300;
export const DEFAULT_PANEL_WIDTH = 380;
export const CONTENT_MIN_WIDTH = 860;
export const SIDEBAR_WIDTH = 220;
export const COLUMN_WIDTH = 1040;

export interface AgentPanelLayout {
  docked: boolean;
  width: number;
}

function isDocked(containerWidth: number): boolean {
  return containerWidth >= CONTENT_MIN_WIDTH + PANEL_MIN_WIDTH;
}

export function clampPanelWidth(containerWidth: number, width: number): number {
  const room = isDocked(containerWidth) ? containerWidth - CONTENT_MIN_WIDTH : containerWidth;
  return Math.min(Math.max(width, PANEL_MIN_WIDTH), room);
}

export function sidebarLead(containerWidth: number, dockedPanelWidth: number): number {
  const free = containerWidth - SIDEBAR_WIDTH - COLUMN_WIDTH;
  return Math.max(0, Math.min(free / 2, free - dockedPanelWidth));
}

export function agentPanelLayout(containerWidth: number, savedWidth: number): AgentPanelLayout {
  return {
    docked: isDocked(containerWidth),
    width: clampPanelWidth(containerWidth, savedWidth),
  };
}
