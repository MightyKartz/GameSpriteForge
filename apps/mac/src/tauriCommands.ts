import { invoke } from "@tauri-apps/api/core";
import type { LanguageMode } from "./i18n";

export type LocalSettings = {
  ffmpegPath: string;
  ffprobePath: string;
  defaultOutputFolder: string;
  defaultFps: number;
  defaultSheetSize: number;
  languageMode: LanguageMode;
};

export type DependencyStatus = {
  ffmpegPath: string | null;
  ffprobePath: string | null;
  message: string | null;
};

export type JobRecord = {
  job_id: string;
  source_kind: string;
  state: string;
  job_dir: string;
  error_summary: string | null;
  asset_id?: string | null;
  operation_kind?: "legacy_pipeline" | "prepare_asset" | "prepare_character_pack" | "install_godot";
  lifecycle_state?: "idle" | "queued" | "running" | "awaiting_review" | "succeeded" | "failed" | "cancelled";
  progress?: number;
  artifacts?: Array<{ kind: string; path: string; sha256?: string | null }>;
  error_code?: string | null;
  recoverable?: boolean;
  next_actions?: string[];
  repair?: RepairContext;
};

export type RepairChange = {
  id: string;
  scope: string;
  parameter: string;
  before: unknown;
  after: unknown;
  reason: string;
  confidence: string;
};

export type RepairQualitySnapshot = {
  verdict: QualityReport["verdict"];
  animations: Array<{ name: string; report: QualityReport }>;
};

export type RepairContext = {
  sourceJobId: string;
  attempt: number;
  changes: RepairChange[];
  baseline: RepairQualitySnapshot;
};

export type RepairAnalysis = {
  schemaVersion: "1";
  sourceJobId: string;
  attempt: number;
  canAutoRepair: boolean;
  quality: RepairQualitySnapshot;
  changes: RepairChange[];
  manualActions: string[];
  proposedOperation?: unknown;
};

export type VideoProbe = {
  width: number;
  height: number;
  fps: number;
  durationSeconds: number;
  frameCountEstimate: number;
  codec: string;
  pixelFormat: string;
};

export type ExtractFramesResult = {
  rawDirectory: string;
  frames: string[];
};

export type ChromaParameters = {
  keyMode: "auto_corners" | "manual";
  manualKeyColor: string;
  threshold: number;
  softness: number;
  despillStrength: number;
  haloPixels: number;
};

export type ChromaPreviewResult = {
  sourcePath: string;
  processedPath: string;
  previewJsonPath: string;
  rawWidth: number;
  rawHeight: number;
  processedWidth: number;
  processedHeight: number;
  bbox: unknown | null;
};

export type ProcessedFrame = {
  frame: string;
  width: number;
  height: number;
  bbox: unknown | null;
};

export type ProcessChromaBatchResult = {
  processedDir: string;
  frames: ProcessedFrame[];
};

export type FrameBbox = {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
  centerX: number;
  centerY: number;
  bottomY: number;
  alphaCoverage: number;
};

export type FrameSize = {
  width: number;
  height: number;
};

export type FootAnchor = {
  x: number;
  y: number;
  lockedByUser: boolean;
};

export type NormalizedFrameSummary = {
  frame: string;
  path: string;
  bbox: FrameBbox;
  size: FrameSize;
  anchor: FootAnchor;
  warnings: string[];
};

export type NormalizeFramesResult = {
  outputDirectory: string;
  frames: NormalizedFrameSummary[];
};

export type QualityReport = {
  verdict: "game_ready" | "needs_cleanup" | "prototype_usable" | "blocked";
  metrics: {
    bboxBottomDriftPx: number;
    bboxCenterXDriftPx: number;
    bboxCenterYDriftPx: number;
    bboxWidthVariationPx: number;
    alphaCoverageAvg: number;
    loopMatchScore: number;
    frameCount: number;
    frameSizeConsistent: boolean;
    cellBoundarySafe: boolean;
  };
  recommendations: string[];
  notes: string[];
};

export type LoopFrameRange = {
  startIndex: number;
  endIndex: number;
};

export type ExportPackOutput = {
  exportDir: string;
  framesDir: string;
  framePaths: string[];
  spriteSheetPath: string;
  spriteSheetPaths: string[];
  atlasPath: string;
  manifestPath: string;
  godotHelperPath: string;
  qualityReportPath: string;
  previewGifPath: string;
  packDir: string;
};

export type GodotProjectExportOutput = {
  projectDir: string;
  importScriptPath: string;
  projectFilePath: string;
  readmePath: string;
};

export type PackSummary = {
  id: string;
  name: string;
  version: string;
  frameCount: number;
  previewGif: string;
};

export type PackInspectSummary = PackSummary & {
  root: string;
  manifestPath: string;
  atlasPath: string;
  qualityReportPath: string;
  defaultAnimation?: string;
  animations?: Array<{ name: string; frameCount: number; fps: number; loop: boolean }>;
};

export type CharacterAnimationRequest = {
  name: string;
  input:
    | { kind: "png_sequence"; paths: string[] }
    | {
        kind: "sprite_sheet";
        path: string;
        split:
          | { mode: "fixed_grid"; frameWidth: number; frameHeight: number; columns: number; rows: number }
          | { mode: "transparent_gutters"; alpha_threshold: number; min_gap_px: number };
      };
  fps: number;
  loop: boolean;
  matting: { mode: "preserve_alpha" };
};

export type CharacterWorkflowAnimation = {
  name: string;
  fps: number;
  loop: boolean;
};

export type CharacterWorkflowPreset = {
  id: string;
  version: string;
  label: string;
  description: string;
  defaultAnimation: string;
  requiredAnimations: CharacterWorkflowAnimation[];
  optionalAnimations: CharacterWorkflowAnimation[];
};

export type CharacterWorkflowCatalog = {
  schemaVersion: "1";
  workflows: CharacterWorkflowPreset[];
};

export type PrepareCharacterPackRequest = {
  schemaVersion: "2";
  metadata: {
    name: string;
    defaultAnimation: string;
    creator: string;
    license: string;
  };
  workflow: { id: string; version: string };
  animations: CharacterAnimationRequest[];
  quality?: { requireGameReady: boolean };
};

export type ImportedPack = {
  summary: PackSummary;
  root: string;
  framePaths: string[];
};

export type ImportGsfpackJobResult = {
  job: JobRecord;
  imported: ImportedPack;
  rawFrames: ExtractFramesResult;
  previewGifPath: string;
};

export const defaultChromaParameters: ChromaParameters = {
  keyMode: "auto_corners",
  manualKeyColor: "#00FF00",
  threshold: 48,
  softness: 18,
  despillStrength: 0.5,
  haloPixels: 0,
};

export function startupAutomationJobId() {
  return invoke<string | null>("startup_automation_job_id");
}

export function startupRoute() {
  return invoke<"forge" | "character" | "exports" | "settings" | null>("startup_route");
}

export function loadAutomationJob(jobId: string) {
  return invoke<JobRecord>("load_automation_job", { jobId });
}

export function analyzeRepairJob(jobId: string) {
  return invoke<RepairAnalysis>("analyze_repair_job", { jobId });
}

export function executeRepairJob(jobId: string) {
  return invoke<JobRecord>("execute_repair_job", { jobId });
}

export function prepareCharacterPack(request: PrepareCharacterPackRequest) {
  return invoke<JobRecord>("prepare_character_pack", { request });
}

export function loadCharacterWorkflows() {
  return invoke<CharacterWorkflowCatalog>("character_workflows");
}

export function readPreviewImage(path: string) {
  return invoke<string>("read_preview_image", { path });
}

export function checkFfmpeg(settings: LocalSettings) {
  return invoke<DependencyStatus>("check_ffmpeg", {
    ffmpegPath: blankToNull(settings.ffmpegPath),
    ffprobePath: blankToNull(settings.ffprobePath),
  });
}

export function sampleVideoPath() {
  return invoke<string>("sample_video_path");
}

export function importVideo(sourcePath: string) {
  return invoke<JobRecord>("import_video", { params: { sourcePath } });
}

export function importFrames(sourcePaths: string[]) {
  return invoke<JobRecord>("import_frames", { params: { sourcePaths } });
}

export function importSpriteSheet(sourcePath: string) {
  return invoke<JobRecord>("import_sprite_sheet", { params: { sourcePath } });
}

export function probeVideo(sourcePath: string, settings: LocalSettings) {
  return invoke<VideoProbe>("probe_video", {
    params: {
      inputPath: sourcePath,
      configuredFfprobePath: blankToNull(settings.ffprobePath),
      bundledResourcePath: null,
    },
  });
}

export function extractFrames(
  sourcePath: string,
  jobDir: string,
  settings: LocalSettings,
  startTimeSeconds: number,
  endTimeSeconds: number,
  keepEveryNFrames: number,
) {
  return invoke<ExtractFramesResult>("extract_frames", {
    params: {
      inputPath: sourcePath,
      startTimeSeconds,
      endTimeSeconds,
      keepEveryNFrames,
      outputDirectory: jobDir,
      configuredFfmpegPath: blankToNull(settings.ffmpegPath),
      bundledResourcePath: null,
    },
  });
}

export function sliceSpriteSheet(args: {
  sheetPath: string;
  jobDir: string;
  frameWidth: number;
  frameHeight: number;
  columns: number;
  rows: number;
}) {
  return invoke<ExtractFramesResult>("slice_sprite_sheet", {
    params: {
      sheetPath: args.sheetPath,
      outputDirectory: args.jobDir,
      frameWidth: args.frameWidth,
      frameHeight: args.frameHeight,
      columns: args.columns,
      rows: args.rows,
    },
  });
}

export function sliceSpriteSheetTransparent(args: {
  sheetPath: string;
  jobDir: string;
  alphaThreshold: number;
  minGapPx: number;
}) {
  return invoke<ExtractFramesResult>("slice_sprite_sheet_transparent", {
    params: {
      sheetPath: args.sheetPath,
      outputDirectory: args.jobDir,
      alphaThreshold: args.alphaThreshold,
      minGapPx: args.minGapPx,
    },
  });
}

export function previewChromaFrame(
  rawFramePath: string,
  jobDir: string,
  parameters: ChromaParameters = defaultChromaParameters,
) {
  return invoke<ChromaPreviewResult>("preview_chroma_frame", {
    rawFramePath,
    parameters,
    targetCanvasMode: "square_bottom",
    previewsDir: `${jobDir}/previews`,
  });
}

export function processChromaBatch(
  rawFramePaths: string[],
  jobDir: string,
  parameters: ChromaParameters = defaultChromaParameters,
) {
  return invoke<ProcessChromaBatchResult>("process_chroma_batch", {
    rawFramePaths,
    processedDir: `${jobDir}/processed`,
    parameters,
  });
}

export function normalizeFrames(processedFramePaths: string[], jobDir: string) {
  return normalizeFramesWithAnchor(processedFramePaths, jobDir, null);
}

export function normalizeFramesWithAnchor(
  processedFramePaths: string[],
  jobDir: string,
  manualAnchor: FootAnchor | null,
) {
  return invoke<NormalizeFramesResult>("normalize_frames", {
    params: {
      framePaths: processedFramePaths,
      outputDirectory: `${jobDir}/processed/normalized`,
      options: {
        mode: "square_bottom",
        marginBottom: 16,
        margin: 12,
        alphaThreshold: 0,
        manualAnchor,
      },
    },
  });
}

export function computeQualityReport(frames: NormalizedFrameSummary[], loopRange: LoopFrameRange | null = null) {
  return invoke<QualityReport>("compute_quality_report", {
    bboxes: frames.map((frame) => frame.bbox),
    sizes: frames.map((frame) => frame.size),
    loopRange,
  });
}

export function exportPack(args: {
  jobDir: string;
  exportsDir: string;
  sourcePath: string;
  frames: NormalizedFrameSummary[];
  qualityReport: QualityReport;
  packName: string;
  animationName: string;
  creatorName: string;
  licenseType: string;
  loopAnimation: boolean;
  loopRange: LoopFrameRange | null;
  fps: number;
  sheetSize: number;
  sheetColumns: number;
  allowMultiSheet: boolean;
  sheetMarginPx: number;
  sheetPaddingPx: number;
  sourceKind: "import_video" | "import_frames" | "import_sprite_sheet" | "import_gsfpack";
  sourceName?: string;
}) {
  const safeName = sanitizeId(args.packName || "local-pack");
  const anchor = args.frames[0]?.anchor ?? { x: 0, y: 0, lockedByUser: false };
  const animationName = args.animationName.trim() || "animation";
  const creatorName = args.creatorName.trim() || "Local Creator";
  const licenseType = args.licenseType.trim() || "private";
  const animationFrames = animationFrameIndexes(args.frames.length, args.loopRange);

  return invoke<ExportPackOutput>("export_pack", {
    params: {
      exportsDir: args.exportsDir.trim() || `${args.jobDir}/exports`,
      exportId: `${safeName}-${Date.now()}`,
      framePaths: args.frames.map((frame) => frame.path),
      sheet: {
        columns: Math.max(1, Math.round(args.sheetColumns)),
        paddingPx: Math.max(0, Math.round(args.sheetPaddingPx)),
        marginPx: Math.max(0, Math.round(args.sheetMarginPx)),
        maxTextureSize: args.sheetSize,
        allowMultiSheet: args.allowMultiSheet,
      },
      gif: {
        fps: args.fps,
        loopAnimation: args.loopAnimation,
        background: "checkerboard",
      },
      metadata: {
        id: safeName,
        name: args.packName || "Local Pack",
        version: "0.1.0",
        creatorName,
        licenseType,
        sourceKind: args.sourceKind,
        sourceName: args.sourceName ?? fileName(args.sourcePath),
        animationName,
        animationFrames,
        fps: args.fps,
        loopAnimation: args.loopAnimation,
        anchor,
        qualityReport: args.qualityReport,
      },
    },
  });
}

export function exportGodotProject(args: {
  packDir: string;
  outputDir: string;
  projectName: string;
}) {
  return invoke<GodotProjectExportOutput>("export_godot_project", {
    params: {
      packDir: args.packDir,
      outputDir: args.outputDir,
      projectName: args.projectName,
    },
  });
}

function animationFrameIndexes(frameCount: number, loopRange: LoopFrameRange | null) {
  if (!loopRange) {
    return Array.from({ length: frameCount }, (_, index) => index);
  }
  const start = Math.max(0, Math.min(frameCount - 1, Math.round(loopRange.startIndex)));
  const end = Math.max(start, Math.min(frameCount - 1, Math.round(loopRange.endIndex)));
  return Array.from({ length: end - start + 1 }, (_, index) => start + index);
}

export function validateGsfpack(path: string) {
  return invoke<PackSummary>("validate_gsfpack", { path });
}

export function listLocalPacks(exportsDir: string) {
  return invoke<PackInspectSummary[]>("list_local_packs", { params: { exportsDir } });
}

export function inspectLocalPack(path: string) {
  return invoke<PackInspectSummary>("inspect_local_pack", { path });
}

export function importGsfpack(path: string) {
  return invoke<ImportGsfpackJobResult>("import_gsfpack", { path });
}

function blankToNull(value: string) {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function sanitizeId(value: string) {
  const sanitized = value.replace(/[^A-Za-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "");
  return sanitized || "local-pack";
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() || path;
}
