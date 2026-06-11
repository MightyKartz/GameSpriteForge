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

export type WorkbenchStage =
  | "empty"
  | "source_selected"
  | "frames_ready"
  | "processed_ready"
  | "quality_ready"
  | "export_ready"
  | "exported_unvalidated"
  | "validated"
  | "godot_project_ready"
  | "blocked"
  | "running";

export type WorkbenchStageInput = {
  canExport: boolean;
  extractFrameCount: number;
  hasExport: boolean;
  hasGodotProject: boolean;
  hasLiveFrames: boolean;
  hasQualityReport: boolean;
  hasSelectedSource: boolean;
  hasValidation: boolean;
  isRunning: boolean;
  normalizeFrameCount: number;
  qualityVerdict?: "game_ready" | "needs_cleanup" | "prototype_usable" | "blocked" | null;
  sourcePendingExtraction: boolean;
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

export function deriveWorkbenchStage(input: WorkbenchStageInput): WorkbenchStage {
  if (input.isRunning) {
    return "running";
  }

  if (input.qualityVerdict === "blocked") {
    return "blocked";
  }

  if (input.hasGodotProject) {
    return "godot_project_ready";
  }

  if (input.hasExport && input.hasValidation) {
    return "validated";
  }

  if (input.hasExport) {
    return "exported_unvalidated";
  }

  if (input.canExport) {
    return "export_ready";
  }

  if (input.hasQualityReport) {
    return "quality_ready";
  }

  if (input.normalizeFrameCount > 0) {
    return "processed_ready";
  }

  if (input.extractFrameCount > 0 || input.hasLiveFrames) {
    return "frames_ready";
  }

  if (input.hasSelectedSource || input.sourcePendingExtraction) {
    return "source_selected";
  }

  return "empty";
}
