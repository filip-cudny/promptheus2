export type MenuItemType =
  | "skill"
  | "preset"
  | "history"
  | "system"
  | "speech"
  | "context"
  | "last_interaction"
  | "settings_section"
  | "chat";

export interface MenuItem {
  id: string;
  label: string;
  item_type: MenuItemType;
  data: unknown | null;
  enabled: boolean;
  separator_after: boolean;
  style: string | null;
  tooltip: string | null;
  submenu_items: MenuItem[] | null;
  icon: string | null;
  section_id: string | null;
}
