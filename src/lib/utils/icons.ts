import {
  Check,
  Code,
  Copy,
  EllipsisVertical,
  Eye,
} from "lucide-static";
import { ICON_SIZE } from "$lib/constants/ui";

type IconSize = keyof typeof ICON_SIZE;

export function icon(svg: string, size: IconSize = "md"): string {
  const s = ICON_SIZE[size];
  return svg
    .replace(/width="\d+"/, `width="${s}"`)
    .replace(/height="\d+"/, `height="${s}"`);
}

export const icons = { Check, Code, Copy, EllipsisVertical, Eye } as const;
