export type MenuItemType =
  | "skill"
  | "speech"
  | "context"
  | "last_interaction"
  | "settings_section"
  | "chat"
  | "models";

export interface MenuItem {
  id: string;
  label: string;
  item_type: MenuItemType;
  data: unknown | null;
  enabled: boolean;
  separator_after: boolean;
  tooltip: string | null;
  icon: string | null;
  section_id: string | null;
}
