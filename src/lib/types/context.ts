export type ContextItem =
  | { item_type: "text"; number: number; content: string }
  | { item_type: "image"; number: number; data: string; media_type: string };
