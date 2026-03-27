export type ContextItem =
  | { item_type: "text"; content: string }
  | { item_type: "image"; data: string; media_type: string };
