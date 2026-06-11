import { Box, Gamepad2, type LucideIcon } from "lucide-react";
import type { TranslationKey } from "./i18n";

export type DemoPack = {
  name: string;
  version: string;
  hue: string;
  sat: string;
  bri: string;
};

export type DemoAsset = {
  name: string;
  frames: number;
  tone: "pink" | "blue" | "violet";
};

export type DemoTrack = DemoAsset & {
  visible: number;
  barLabel: string;
};

export type ExportTarget = {
  labelKey: TranslationKey;
  sublabelKey: TranslationKey;
  icon: LucideIcon;
  selected?: boolean;
  available: boolean;
  badge?: string;
};

export const demoPacks: DemoPack[] = [
  { name: "Green Box Character Pack", version: "v1.2.0", hue: "112deg", sat: ".64", bri: ".88" },
  { name: "Green Box Run Pack", version: "v1.0.0", hue: "148deg", sat: ".52", bri: ".76" },
  { name: "Green Box Jump Pack", version: "v1.0.0", hue: "190deg", sat: ".58", bri: ".8" },
  { name: "Green Box Combat Pack", version: "v1.0.0", hue: "42deg", sat: ".72", bri: ".82" },
];

export const demoAssets: DemoAsset[] = [
  { name: "green_box_character", frames: 24, tone: "blue" },
];

export const demoTracks: DemoTrack[] = [
  { name: "green_box_character", frames: 24, tone: "blue", visible: 12, barLabel: "24" },
];

export const exportTargets: ExportTarget[] = [
  { labelKey: "export.target.pngJson", sublabelKey: "export.target.generic", icon: Box, selected: true, available: true },
  { labelKey: "export.target.godotHelper", sublabelKey: "export.target.metadata", icon: Gamepad2, available: true },
];

export function hasLiveWorkbenchData(values: Array<unknown>) {
  return values.some(Boolean);
}
