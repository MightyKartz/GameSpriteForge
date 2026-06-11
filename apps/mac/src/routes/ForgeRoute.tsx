import { ImportPanel, type SpriteGrid, type SpriteSheetSliceMode } from "../components/ImportPanel";
import { VideoSegmentPanel } from "../components/VideoSegmentPanel";
import { ChromaPreviewPanel } from "../components/ChromaPreviewPanel";
import { CanvasPreview } from "../components/CanvasPreview";
import { FrameTimeline } from "../components/FrameTimeline";
import { QualityInspector } from "../components/QualityInspector";
import { ExportPanel, type ExportReadinessItem } from "../components/ExportPanel";
import { hasLiveWorkbenchData } from "../forgeViewModel";
import {
  AlertTriangle,
  CheckCircle2,
  Circle,
  FileArchive,
  FileImage,
  Film,
  Images,
  PackageCheck,
  PlayCircle,
  Settings as SettingsIcon,
  Wrench,
} from "lucide-react";
import { APP_VERSION } from "../appMetadata";
import {
  chooseGsfpackFolder,
  choosePngSequence,
  chooseSpriteSheetFile,
  chooseVideoFile,
  openFileOrFolder,
} from "../systemDialogs";
import {
  checkFfmpeg,
  computeQualityReport,
  defaultChromaParameters,
  exportPack,
  extractFrames,
  importFrames,
  importGsfpack,
  importSpriteSheet,
  importVideo,
  previewChromaFrame,
  probeVideo,
  processChromaBatch,
  sampleVideoPath,
  normalizeFramesWithAnchor,
  sliceSpriteSheet,
  sliceSpriteSheetTransparent,
  validateGsfpack,
  type ChromaPreviewResult,
  type DependencyStatus,
  type ExportPackOutput,
  type ExtractFramesResult,
  type FootAnchor,
  type JobRecord,
  type LoopFrameRange,
  type LocalSettings,
  type NormalizeFramesResult,
  type PackSummary,
  type QualityReport,
  type VideoProbe,
} from "../tauriCommands";
import { usePreviewImage } from "../hooks/usePreviewImage";
import type { TFunction, TranslationKey } from "../i18n";
import { useEffect, useMemo, useState } from "react";

type ForgeRouteProps = {
  activeWorkflow: ForgeWorkflow;
  isActive: boolean;
  latestRecentExport: { output: ExportPackOutput; packName: string } | null;
  onExportRecorded: (record: { frameCount: number; output: ExportPackOutput; packName: string }) => void;
  onChooseOutputFolder: () => void | Promise<void>;
  onOpenSettings: () => void;
  onQueuedGsfpackImportConsumed: () => void;
  onReimportRecentExport: () => void;
  onWorkbenchStateChange: (state: WorkbenchState) => void;
  onWorkflowChange: (workflow: ForgeWorkflow) => void;
  queuedGsfpackImportPath: string | null;
  settings: LocalSettings;
  t: TFunction;
};

type PipelineStatus = {
  tone: "idle" | "running" | "success" | "error";
  message: string;
};

type RecoveryActionKey =
  | "background"
  | "checkToolchain"
  | "export"
  | "exportReview"
  | "extract"
  | "frames"
  | "import"
  | "process"
  | "sheet"
  | "settings";

type RecoveryAction = {
  disabled?: boolean;
  key: RecoveryActionKey;
  label: string;
};

type RecoveryPlan = {
  actions: RecoveryAction[];
  detail: string;
  title: string;
};

type RunSummaryItem = {
  label: string;
  tone: "blocked" | "ok" | "pending" | "ready" | "warning";
  value: string;
};

type ForgeWorkflow = "Import" | "Frames" | "Background" | "Anchor" | "Sheet" | "Export";
const defaultPackName = "Untitled Sprite Pack";
const bundledSamplePackName = "Green Box Character Pack";
const bundledSampleAnimationName = "walk";

const forgeWorkflowLabelKeys: Record<ForgeWorkflow, TranslationKey> = {
  Anchor: "workflow.anchor",
  Background: "workflow.background",
  Export: "workflow.export",
  Frames: "workflow.frames",
  Import: "workflow.import",
  Sheet: "workflow.sheet",
};

type WorkbenchState = {
  canRunQualityCheck: boolean;
  qualityStatus: "checked" | "pending" | "readyToCheck";
  workflowAccess: Record<ForgeWorkflow, boolean>;
};

export function ForgeRoute({
  activeWorkflow,
  isActive,
  latestRecentExport,
  onExportRecorded,
  onChooseOutputFolder,
  onOpenSettings,
  onQueuedGsfpackImportConsumed,
  onReimportRecentExport,
  onWorkbenchStateChange,
  onWorkflowChange,
  queuedGsfpackImportPath,
  settings,
  t,
}: ForgeRouteProps) {
  const [sourcePath, setSourcePath] = useState("");
  const [frameSequenceInput, setFrameSequenceInput] = useState("");
  const [spriteSheetPath, setSpriteSheetPath] = useState("");
  const [spriteGrid, setSpriteGrid] = useState<SpriteGrid>({ frameWidth: 64, frameHeight: 64, columns: 4, rows: 1 });
  const [spriteSheetSliceMode, setSpriteSheetSliceMode] = useState<SpriteSheetSliceMode>("grid");
  const [transparentSplitAlphaThreshold, setTransparentSplitAlphaThreshold] = useState(0);
  const [transparentSplitMinGapPx, setTransparentSplitMinGapPx] = useState(1);
  const [gsfpackPath, setGsfpackPath] = useState("");
  const [packName, setPackName] = useState(defaultPackName);
  const [animationName, setAnimationName] = useState("idle");
  const [creatorName, setCreatorName] = useState("Game Sprite Forge");
  const [licenseType, setLicenseType] = useState("private");
  const [loopAnimation, setLoopAnimation] = useState(true);
  const [loopStartFrame, setLoopStartFrame] = useState(1);
  const [loopEndFrame, setLoopEndFrame] = useState(1);
  const [sheetColumns, setSheetColumns] = useState(4);
  const [sheetPaddingPx, setSheetPaddingPx] = useState(2);
  const [sheetMarginPx, setSheetMarginPx] = useState(2);
  const [allowMultiSheet, setAllowMultiSheet] = useState(true);
  const [startTimeSeconds, setStartTimeSeconds] = useState(0);
  const [endTimeSeconds, setEndTimeSeconds] = useState(1);
  const [keepEveryNFrames, setKeepEveryNFrames] = useState(1);
  const [job, setJob] = useState<JobRecord | null>(null);
  const [workingSourcePath, setWorkingSourcePath] = useState<string | null>(null);
  const [activeSourceName, setActiveSourceName] = useState(t("source.noSource"));
  const [packPreviewPath, setPackPreviewPath] = useState<string | null>(null);
  const [ffmpegStatus, setFfmpegStatus] = useState<DependencyStatus | null>(null);
  const [probe, setProbe] = useState<VideoProbe | null>(null);
  const [extractResult, setExtractResult] = useState<ExtractFramesResult | null>(null);
  const [previewResult, setPreviewResult] = useState<ChromaPreviewResult | null>(null);
  const [normalizeResult, setNormalizeResult] = useState<NormalizeFramesResult | null>(null);
  const [qualityReport, setQualityReport] = useState<QualityReport | null>(null);
  const [exportOutput, setExportOutput] = useState<ExportPackOutput | null>(null);
  const [packSummary, setPackSummary] = useState<PackSummary | null>(null);
  const [selectedFrameIndex, setSelectedFrameIndex] = useState(0);
  const [chromaParameters, setChromaParameters] = useState(defaultChromaParameters);
  const [manualAnchor, setManualAnchor] = useState<Pick<FootAnchor, "x" | "y">>({ x: 1024, y: 2032 });
  const [appliedAnchor, setAppliedAnchor] = useState<FootAnchor | null>(null);
  const [status, setStatus] = useState<PipelineStatus>({
    tone: "idle",
    message: t("status.readyLocalImport"),
  });
  const [isRunning, setIsRunning] = useState(false);

  const timelineFramePaths = normalizeResult?.frames.map((frame) => frame.path) ?? extractResult?.frames ?? [];
  const selectedFramePath = timelineFramePaths[selectedFrameIndex] ?? previewResult?.processedPath ?? packPreviewPath ?? null;
  const bboxLabel = useMemo(() => {
    const frame = normalizeResult?.frames[selectedFrameIndex];
    if (!frame) {
      return previewResult ? t("stage.previewReady") : undefined;
    }
    return t("stage.bboxTag", {
      frame: frame.frame,
      height: Math.round(frame.bbox.height),
      width: Math.round(frame.bbox.width),
    });
  }, [normalizeResult, previewResult, selectedFrameIndex, t]);
  const canvasFrameTag = extractResult && !normalizeResult
    ? t("stage.rawFrameTag", { count: extractResult.frames.length, selected: selectedFrameIndex + 1 })
    : bboxLabel;
  const hasLiveFrames = timelineFramePaths.length > 0;
  const sourceFrameCount = probe ? Math.max(1, Math.round(probe.frameCountEstimate)) : null;
  const liveFrameCount = normalizeResult?.frames.length ?? extractResult?.frames.length ?? null;
  const frameCount = liveFrameCount ?? sourceFrameCount ?? 0;
  const hasOutputFolder = Boolean(settings.defaultOutputFolder.trim());
  const canExport = Boolean(
    job && normalizeResult?.frames.length && qualityReport && qualityReport.verdict !== "blocked" && hasOutputFolder,
  );
  const canExtractFrames = Boolean(job?.source_kind === "import_video" && probe);
  const canProcessAndQuality = Boolean(extractResult || (job?.source_kind === "import_video" && probe));
  const sourcePendingExtraction = Boolean(job?.source_kind === "import_video" && probe && !extractResult && !normalizeResult);
  const framesPendingQuality = Boolean(extractResult?.frames.length && !normalizeResult && !qualityReport);
  const headerQualityStatus = qualityReport ? "checked" : canProcessAndQuality ? "readyToCheck" : "pending";
  const hasSelectedSource = Boolean(job || workingSourcePath || probe);
  const showSideImportPanel = Boolean(
    hasSelectedSource || sourcePath.trim() || frameSequenceInput.trim() || spriteSheetPath.trim() || gsfpackPath.trim(),
  );
  const hasLiveData = hasLiveWorkbenchData([extractResult, normalizeResult, qualityReport, exportOutput, packSummary]) && hasLiveFrames;
  const visibleAssets = hasLiveData
    ? [{ name: packName.toLowerCase().replace(/\s+/g, "_"), frames: frameCount, tone: "blue" as const }]
    : [];
  const hasQualityWarnings = Boolean(qualityReport && qualityReport.recommendations.length > 0);
  const qualityStatus = qualityReport
    ? qualityReport.verdict === "blocked"
      ? { className: "status-warning", label: t("quality.status.needsAttention") }
      : hasQualityWarnings
        ? { className: "status-warning", label: t("quality.status.hasWarnings", { count: qualityReport.recommendations.length }) }
      : { className: "status-ok", label: t("quality.status.allPassed") }
    : { className: "status-pending", label: t("quality.status.pending") };
  const visibleQualityStatus = qualityReport
    ? qualityStatus
    : canProcessAndQuality
      ? { className: "status-ready", label: t("quality.status.readyToCheck") }
      : qualityStatus;
  const inspectorPanelClassName = [
    "inspector-panel",
    qualityReport && (canExport || exportOutput || activeWorkflow === "Export") ? "export-ready-inspector" : "",
  ].filter(Boolean).join(" ");
  const workspaceMode = hasLiveData
    ? t("source.liveWorkspace")
    : hasSelectedSource
      ? t("source.selectedWorkspace")
      : t("source.emptyWorkspace");
  const exportedSheetPreviewSrc = usePreviewImage(exportOutput?.spriteSheetPath, null);
  const sheetPreviewSrc = exportOutput ? exportedSheetPreviewSrc : null;
  const canApplyAnchor = Boolean(job && extractResult);
  const durationSeconds = probe?.durationSeconds ?? (extractResult ? frameCount / settings.defaultFps : null);
  const activeLoopRange = loopFrameRange(frameCount, loopStartFrame, loopEndFrame);
  const workflowAccess = useMemo<Record<ForgeWorkflow, boolean>>(
    () => ({
      Import: true,
      Frames: Boolean(job && (probe || extractResult)),
      Background: Boolean(extractResult || normalizeResult),
      Anchor: Boolean(extractResult || normalizeResult),
      Sheet: Boolean(normalizeResult || exportOutput),
      Export: Boolean(canExport || exportOutput),
    }),
    [canExport, exportOutput, extractResult, job, normalizeResult, probe],
  );
  const pipelineSteps = [
    { label: t("pipeline.step.import"), stateClass: job ? "done" : "ready", stateLabel: job ? t("pipeline.state.done") : t("pipeline.state.ready") },
    { label: t("pipeline.step.extract"), stateClass: extractResult ? "done" : canExtractFrames ? "ready" : "pending", stateLabel: extractResult ? t("pipeline.state.done") : canExtractFrames ? t("pipeline.state.ready") : t("pipeline.state.pending") },
    { label: t("pipeline.step.process"), stateClass: normalizeResult ? "done" : canProcessAndQuality ? "ready" : "pending", stateLabel: normalizeResult ? t("pipeline.state.done") : canProcessAndQuality ? t("pipeline.state.ready") : t("pipeline.state.pending") },
    { label: t("pipeline.step.quality"), stateClass: qualityReport ? "done" : normalizeResult ? "ready" : "pending", stateLabel: qualityReport ? verdictLabel(qualityReport.verdict, t) : normalizeResult ? t("pipeline.state.ready") : t("pipeline.state.pending") },
    { label: t("pipeline.step.export"), stateClass: exportOutput ? "done" : canExport ? "ready" : "pending", stateLabel: exportOutput ? t("pipeline.state.done") : canExport ? t("pipeline.state.ready") : t("pipeline.state.pending") },
  ];
  const exportReadiness: ExportReadinessItem[] = [
    {
      detail: job ? sourceKindLabel(job.source_kind, t) : t("export.readiness.noSource"),
      label: t("export.readiness.source"),
      state: job ? "complete" : "blocked",
    },
    {
      detail: normalizeResult?.frames.length
        ? t("export.readiness.processedFramesReady", { count: normalizeResult.frames.length })
        : extractResult?.frames.length
          ? t("export.readiness.rawFramesReady", { count: extractResult.frames.length })
          : probe
            ? t("export.readiness.videoReady")
          : t("export.readiness.importExtractFirst"),
      label: t("export.readiness.processedFrames"),
      state: normalizeResult?.frames.length ? "complete" : extractResult || probe ? "ready" : "pending",
    },
    {
      detail: qualityReport
        ? hasQualityWarnings
          ? t("export.readiness.verdictWithWarnings", {
              count: qualityReport.recommendations.length,
              verdict: verdictLabel(qualityReport.verdict, t),
            })
          : t("export.readiness.verdict", { verdict: verdictLabel(qualityReport.verdict, t) })
        : t("export.readiness.runQuality"),
      label: t("export.readiness.qualityReport"),
      state: qualityReport
        ? qualityReport.verdict === "blocked"
          ? "blocked"
          : hasQualityWarnings || qualityReport.verdict === "needs_cleanup"
            ? "warning"
            : "complete"
        : normalizeResult
          ? "ready"
          : "pending",
    },
    {
      detail: hasOutputFolder ? settings.defaultOutputFolder : t("export.readiness.chooseOutput"),
      label: t("export.readiness.outputFolder"),
      state: hasOutputFolder ? "complete" : "blocked",
    },
  ];
  const runSummaryItems: RunSummaryItem[] = [
    {
      label: t("summary.source"),
      tone: job ? "ok" : "pending",
      value: job ? activeSourceName : t("summary.noLiveSource"),
    },
    {
      label: t("summary.frames"),
      tone: normalizeResult ? "ok" : extractResult ? "ready" : probe ? "ready" : "pending",
      value: normalizeResult
        ? t("summary.processed", { count: normalizeResult.frames.length })
        : extractResult
          ? t("summary.raw", { count: extractResult.frames.length })
          : probe
            ? t("summary.estimated", { count: Math.max(1, Math.round(probe.frameCountEstimate)) })
            : t("summary.waiting"),
    },
    {
      label: t("summary.quality"),
      tone: qualityReport
        ? qualityReport.verdict === "blocked"
          ? "warning"
          : hasQualityWarnings
            ? "warning"
            : "ok"
        : canProcessAndQuality
          ? "ready"
          : "pending",
      value: qualityReport
        ? hasQualityWarnings
          ? t("summary.verdictWithWarnings", {
              count: qualityReport.recommendations.length,
              verdict: verdictLabel(qualityReport.verdict, t),
            })
          : verdictLabel(qualityReport.verdict, t)
        : canProcessAndQuality
          ? framesPendingQuality
            ? t("summary.readyToCheck")
            : t("summary.ready")
          : t("summary.pending"),
    },
    {
      label: t("summary.export"),
      tone: exportOutput ? "ok" : canExport ? "ready" : sourcePendingExtraction || framesPendingQuality ? "pending" : "blocked",
      value: exportOutput
        ? fileName(exportOutput.packDir)
        : canExport
          ? t("summary.ready")
          : sourcePendingExtraction
            ? t("summary.waitingForFrames")
            : framesPendingQuality
              ? t("summary.waitingForQuality")
              : t("summary.blocked"),
    },
    {
      label: t("summary.validate"),
      tone: packSummary ? "ok" : exportOutput ? "ready" : "pending",
      value: packSummary
        ? t("summary.validatedFrames", { count: packSummary.frameCount })
        : exportOutput
          ? t("summary.validationPending")
          : t("summary.notRun"),
    },
  ];
  const recoveryPlan = status.tone === "error"
    ? recoveryPlanFor(status.message, { canExport, canExtractFrames, canProcessAndQuality }, t)
    : null;
  const recoveryHandlers: Record<RecoveryActionKey, () => void | Promise<void>> = {
    background: () => onWorkflowChange("Background"),
    checkToolchain: handleCheckFfmpeg,
    export: handleExportPack,
    exportReview: () => onWorkflowChange("Export"),
    extract: handleExtractFrames,
    frames: () => onWorkflowChange("Frames"),
    import: () => onWorkflowChange("Import"),
    process: handleProcessAndQuality,
    sheet: () => onWorkflowChange("Sheet"),
    settings: onOpenSettings,
  };

  useEffect(() => {
    onWorkbenchStateChange({
      canRunQualityCheck: canProcessAndQuality,
      qualityStatus: headerQualityStatus,
      workflowAccess,
    });
  }, [canProcessAndQuality, headerQualityStatus, onWorkbenchStateChange, workflowAccess]);

  useEffect(() => {
    if (!queuedGsfpackImportPath || isRunning) {
      return;
    }

    setGsfpackPath(queuedGsfpackImportPath);
    onQueuedGsfpackImportConsumed();
    void handleImportGsfpack(queuedGsfpackImportPath);
  }, [queuedGsfpackImportPath, isRunning, onQueuedGsfpackImportConsumed]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (!isActive || isEditableTarget(event.target)) {
        return;
      }

      const commandPressed = event.metaKey || event.ctrlKey;
      if (commandPressed && event.key === "Enter" && canProcessAndQuality && !isRunning) {
        event.preventDefault();
        void handleProcessAndQuality();
        return;
      }

      if (commandPressed && event.key.toLowerCase() === "e" && canExport && !isRunning) {
        event.preventDefault();
        void handleExportPack();
        return;
      }

      if ((event.key === "[" || event.key === "ArrowLeft") && timelineFramePaths.length) {
        event.preventDefault();
        setSelectedFrameIndex((index) => Math.max(0, index - 1));
        return;
      }

      if ((event.key === "]" || event.key === "ArrowRight") && timelineFramePaths.length) {
        event.preventDefault();
        setSelectedFrameIndex((index) => Math.min(timelineFramePaths.length - 1, index + 1));
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  async function runStep(runningMessage: string, action: () => Promise<string>) {
    setIsRunning(true);
    setStatus({ tone: "running", message: runningMessage });
    try {
      const message = await action();
      setStatus({ tone: "success", message });
    } catch (error) {
      setStatus({ tone: "error", message: errorMessage(error) });
    } finally {
      setIsRunning(false);
    }
  }

  function resetLoopRange(count: number) {
    setLoopStartFrame(1);
    setLoopEndFrame(Math.max(1, count));
  }

  async function handleCheckFfmpeg() {
    await runStep(t("status.checkingTools"), async () => {
      const dependencyStatus = await checkFfmpeg(settings);
      setFfmpegStatus(dependencyStatus);
      return dependencyStatus.message ?? t("status.toolsAvailable");
    });
  }

  async function handleImportVideo() {
    await runStep(t("status.importingVideo"), async () => {
      const result = await importAndProbeVideo();
      onWorkflowChange("Frames");
      return t("status.importedVideo", { jobId: result.job.job_id, name: fileName(sourcePath) });
    });
  }

  async function handleChooseVideoFile() {
    await chooseAndSetPath(() => chooseVideoFile(sourcePath), setSourcePath, t("status.videoFileSelected"));
  }

  async function handleChooseVideoAndImport() {
    await runStep(t("status.importingVideo"), async () => {
      const path = await chooseVideoFile(sourcePath);
      if (!path) {
        return t("status.selectionCancelled");
      }
      setSourcePath(path);
      const result = await importAndProbeVideo(path);
      onWorkflowChange("Frames");
      return t("status.importedVideo", { jobId: result.job.job_id, name: fileName(path) });
    });
  }

  async function handleLoadSample() {
    await runStep(t("status.loadingSample"), async () => {
      const path = await sampleVideoPath();
      setSourcePath(path);
      setPackName(bundledSamplePackName);
      setAnimationName(bundledSampleAnimationName);
      return t("status.sampleLoaded");
    });
  }

  async function handleImportFrames() {
    await runStep(t("status.importingPng"), async () => {
      const paths = frameSequenceInput
        .split(/[\n,]+/)
        .map((value) => value.trim())
        .filter(Boolean);
      return importFramePaths(paths);
    });
  }

  async function handleChooseFrameFiles() {
    await runStep(t("status.choosingPng"), async () => {
      const paths = await choosePngSequence(firstInputPath(frameSequenceInput));
      if (!paths.length) {
        return t("status.pngCancelled");
      }
      setFrameSequenceInput(paths.join("\n"));
      return t("status.selectedPng", { count: paths.length });
    });
  }

  async function handleChooseFrameFilesAndImport() {
    await runStep(t("status.importingPng"), async () => {
      const paths = await choosePngSequence(firstInputPath(frameSequenceInput));
      if (!paths.length) {
        return t("status.pngCancelled");
      }
      setFrameSequenceInput(paths.join("\n"));
      return importFramePaths(paths);
    });
  }

  async function handleImportSpriteSheet() {
    await runStep(t("status.importingSpriteSheet"), async () => {
      const originalPath = requiredPath(spriteSheetPath, "sprite sheet path");
      return importSpriteSheetFromPath(originalPath);
    });
  }

  async function handleChooseSpriteSheetFile() {
    await chooseAndSetPath(() => chooseSpriteSheetFile(spriteSheetPath), setSpriteSheetPath, t("status.spriteSheetSelected"));
  }

  async function handleChooseSpriteSheetForConfiguration() {
    await chooseAndSetPath(() => chooseSpriteSheetFile(spriteSheetPath), setSpriteSheetPath, t("status.spriteSheetSelected"));
    onWorkflowChange("Import");
  }

  async function handleChooseSpriteSheetAndImport() {
    await runStep(t("status.importingSpriteSheet"), async () => {
      const path = await chooseSpriteSheetFile(spriteSheetPath);
      if (!path) {
        return t("status.selectionCancelled");
      }
      setSpriteSheetPath(path);
      return importSpriteSheetFromPath(path);
    });
  }

  async function handleImportGsfpack(pathOverride = gsfpackPath) {
    await runStep(t("status.importingGsfpack"), async () => {
      const packPath = requiredPath(pathOverride, ".gsfpack path");
      return importGsfpackFromPath(packPath);
    });
  }

  async function handleChooseGsfpackFolder() {
    await chooseAndSetPath(() => chooseGsfpackFolder(gsfpackPath), setGsfpackPath, t("status.gsfpackFolderSelected"));
  }

  async function handleChooseGsfpackAndImport() {
    await runStep(t("status.importingGsfpack"), async () => {
      const path = await chooseGsfpackFolder(gsfpackPath);
      if (!path) {
        return t("status.selectionCancelled");
      }
      setGsfpackPath(path);
      return importGsfpackFromPath(path);
    });
  }

  async function handleExtractFrames() {
    await runStep(t("status.extractingRaw"), async () => {
      const current = await ensureVideoJob();
      const result = await extractCurrentFrames(current.job, current.sourcePath);
      return t("status.extractedRaw", { count: result.frames.length });
    });
  }

  async function handleProcessAndQuality() {
    await runStep(t("status.processingQuality"), async () => {
      if (
        (job?.source_kind === "import_frames" ||
          job?.source_kind === "import_sprite_sheet" ||
          job?.source_kind === "import_gsfpack") &&
        extractResult
      ) {
        const result = await processAndQuality(job, extractResult);
        return t("status.processedWithVerdict", {
          count: result.normalizeResult.frames.length,
          verdict: verdictLabel(result.qualityReport.verdict, t),
        });
      }
      const current = await ensureVideoJob();
      const hadExtractedFrames = Boolean(extractResult);
      const rawFrames = extractResult ?? (await extractCurrentFrames(current.job, current.sourcePath));
      const result = await processAndQuality(
        current.job,
        rawFrames,
        appliedAnchor,
        hadExtractedFrames ? undefined : loopFrameRange(rawFrames.frames.length, 1, rawFrames.frames.length),
      );
      return t("status.processedWithVerdict", {
        count: result.normalizeResult.frames.length,
        verdict: verdictLabel(result.qualityReport.verdict, t),
      });
    });
  }

  async function handleApplyAnchor() {
    await runStep(t("status.applyingAnchor"), async () => {
      const currentJob = requireValue(job, "import a source first");
      const rawFrames = requireValue(extractResult, "extract or import frames first");
      const anchor = {
        x: Math.max(0, manualAnchor.x),
        y: Math.max(0, manualAnchor.y),
        lockedByUser: true,
      };
      setAppliedAnchor(anchor);
      const result = await processAndQuality(currentJob, rawFrames, anchor);
      return t("status.appliedAnchor", {
        verdict: verdictLabel(result.qualityReport.verdict, t),
        x: Math.round(anchor.x),
        y: Math.round(anchor.y),
      });
    });
  }

  async function handleExportPack() {
    await runStep(t("status.exportingPack"), async () => {
      const output = await exportCurrentPack();
      onExportRecorded({ frameCount: normalizeResult?.frames.length ?? 0, output, packName });
      return t("status.exportedPack", { name: fileName(output.packDir) });
    });
  }

  async function handleValidateExport() {
    await runStep(t("status.validatingReimport"), async () => {
      const output = requireValue(exportOutput, "export a pack first");
      const summary = await validateGsfpack(output.packDir);
      await importGsfpack(output.packDir);
      setPackSummary(summary);
      return t("status.validatedReimported", { count: summary.frameCount, name: summary.name });
    });
  }

  async function handleOpenExportsFolder() {
    await runStep(t("status.openingExportFolder"), async () => {
      const output = requireValue(exportOutput, "export a pack first");
      await openFileOrFolder(output.exportDir);
      return t("status.openedFolder", { name: fileName(output.exportDir) });
    });
  }

  async function handleRunFullPipeline() {
    await runStep(t("status.runningFullSample"), async () => {
      const dependencyStatus = await checkFfmpeg(settings);
      setFfmpegStatus(dependencyStatus);
      if (dependencyStatus.message) {
        throw new Error(dependencyStatus.message);
      }

      const usesBundledSample = !sourcePath.trim();
      const sourceForPipeline = usesBundledSample ? await sampleVideoPath() : sourcePath.trim();
      const packNameForPipeline = usesBundledSample ? bundledSamplePackName : packName;
      const animationNameForPipeline = usesBundledSample ? bundledSampleAnimationName : animationName;
      if (usesBundledSample) {
        setSourcePath(sourceForPipeline);
        setPackName(packNameForPipeline);
        setAnimationName(animationNameForPipeline);
      }
      const current = await importAndProbeVideo(sourceForPipeline);
      const rawFrames = await extractCurrentFrames(current.job, current.sourcePath);
      const fullLoopRange = loopFrameRange(rawFrames.frames.length, 1, rawFrames.frames.length);
      const processed = await processAndQuality(
        current.job,
        rawFrames,
        appliedAnchor,
        fullLoopRange,
      );
      const output = await exportCurrentPack(current.job, processed.normalizeResult, processed.qualityReport, {
        animationName: animationNameForPipeline,
        loopRange: fullLoopRange,
        packName: packNameForPipeline,
        sourceName: fileName(sourceForPipeline),
        sourcePath: sourceForPipeline,
      });
      onExportRecorded({
        frameCount: processed.normalizeResult.frames.length,
        output,
        packName: packNameForPipeline,
      });
      const summary = await validateGsfpack(output.packDir);
      await importGsfpack(output.packDir);
      setPackSummary(summary);
      return t("status.fullPipelinePassed", { count: summary.frameCount });
    });
  }

  async function chooseAndSetPath(
    choose: () => Promise<string | null>,
    setPath: (value: string) => void,
    successMessage: string,
  ) {
    await runStep(t("status.choosingPath"), async () => {
      const path = await choose();
      if (!path) {
        return t("status.selectionCancelled");
      }
      setPath(path);
      return successMessage;
    });
  }

  async function importFramePaths(paths: string[]) {
    if (!paths.length) {
      throw new Error("choose at least one PNG frame");
    }
    const createdJob = await importFrames(paths);
    setJob(createdJob);
    setWorkingSourcePath(null);
    setProbe(null);
    setPreviewResult(null);
    setNormalizeResult(null);
    setQualityReport(null);
    setExportOutput(null);
    setPackSummary(null);
    setPackPreviewPath(null);
    setActiveSourceName(`${paths.length} PNG frames`);
    setExtractResult({
      rawDirectory: `${createdJob.job_dir}/source`,
      frames: paths.map((_, index) => `${createdJob.job_dir}/source/frame_${String(index + 1).padStart(5, "0")}.png`),
    });
    resetLoopRange(paths.length);
    setSelectedFrameIndex(0);
    onWorkflowChange("Frames");
    return t("status.importedPng", { count: paths.length, jobId: createdJob.job_id });
  }

  async function importSpriteSheetFromPath(originalPath: string) {
    const createdJob = await importSpriteSheet(originalPath);
    const importedSheetPath = `${createdJob.job_dir}/source/sprite_sheet.${fileExtension(originalPath)}`;
    const sliced = spriteSheetSliceMode === "transparent"
      ? await sliceSpriteSheetTransparent({
        sheetPath: importedSheetPath,
        jobDir: createdJob.job_dir,
        alphaThreshold: transparentSplitAlphaThreshold,
        minGapPx: transparentSplitMinGapPx,
      })
      : await sliceSpriteSheet({
        sheetPath: importedSheetPath,
        jobDir: createdJob.job_dir,
        frameWidth: spriteGrid.frameWidth,
        frameHeight: spriteGrid.frameHeight,
        columns: spriteGrid.columns,
        rows: spriteGrid.rows,
      });
    setJob(createdJob);
    setWorkingSourcePath(null);
    setProbe(null);
    setExtractResult(sliced);
    resetLoopRange(sliced.frames.length);
    setSelectedFrameIndex(0);
    setPreviewResult(null);
    setNormalizeResult(null);
    setQualityReport(null);
    setExportOutput(null);
    setPackSummary(null);
    setPackPreviewPath(null);
    setActiveSourceName(fileName(originalPath));
    onWorkflowChange("Frames");
    return t("status.importedSpriteSheet", { count: sliced.frames.length, jobId: createdJob.job_id });
  }

  async function importGsfpackFromPath(packPath: string) {
    const summary = await validateGsfpack(packPath);
    const imported = await importGsfpack(packPath);
    setJob(imported.job);
    setWorkingSourcePath(null);
    setProbe(null);
    setExtractResult(imported.rawFrames);
    resetLoopRange(imported.rawFrames.frames.length);
    setSelectedFrameIndex(0);
    setPreviewResult(null);
    setNormalizeResult(null);
    setQualityReport(null);
    setExportOutput(null);
    setPackSummary(summary);
    setPackPreviewPath(imported.previewGifPath);
    setPackName(summary.name);
    setActiveSourceName(fileName(packPath));
    onWorkflowChange("Frames");
    return t("status.importedGsfpack", { count: summary.frameCount, name: summary.name });
  }

  async function ensureVideoJob() {
    if (job && probe && workingSourcePath && job.source_kind === "import_video") {
      return { job, probe, sourcePath: workingSourcePath };
    }
    return importAndProbeVideo();
  }

  async function importAndProbeVideo(sourcePathOverride = sourcePath) {
    const path = requiredPath(sourcePathOverride, "video path");
    if (path !== sourcePath) {
      setSourcePath(path);
    }
    const createdJob = await importVideo(path);
    const importedSourcePath = jobSourcePath(createdJob, path);
    const videoProbe = await probeVideo(importedSourcePath, settings);
    setJob(createdJob);
    setWorkingSourcePath(importedSourcePath);
    setProbe(videoProbe);
    setExtractResult(null);
    setPreviewResult(null);
    setNormalizeResult(null);
    setQualityReport(null);
    setExportOutput(null);
    setPackSummary(null);
    setPackPreviewPath(null);
    setActiveSourceName(fileName(path));
    setSelectedFrameIndex(0);
    setEndTimeSeconds(Math.min(Math.max(0.1, videoProbe.durationSeconds), Math.max(0.1, endTimeSeconds)));
    return { job: createdJob, probe: videoProbe, sourcePath: importedSourcePath };
  }

  async function extractCurrentFrames(currentJob: JobRecord, sourcePathOverride?: string) {
    const result = await extractFrames(
      sourcePathOverride ?? requireValue(workingSourcePath, "import a video first"),
      currentJob.job_dir,
      settings,
      startTimeSeconds,
      Math.max(endTimeSeconds, startTimeSeconds + 0.1),
      keepEveryNFrames,
    );
    setExtractResult(result);
    resetLoopRange(result.frames.length);
    setSelectedFrameIndex(0);
    setPreviewResult(null);
    setNormalizeResult(null);
    setQualityReport(null);
    setExportOutput(null);
    setPackPreviewPath(null);
    return result;
  }

  async function processAndQuality(
    currentJob: JobRecord,
    rawFrames: ExtractFramesResult,
    anchorOverride = appliedAnchor,
    loopRangeOverride?: LoopFrameRange | null,
  ) {
    if (!rawFrames.frames.length) {
      throw new Error("extract at least one raw frame before processing");
    }

    const preview = await previewChromaFrame(rawFrames.frames[0], currentJob.job_dir, chromaParameters);
    const processed = await processChromaBatch(rawFrames.frames, currentJob.job_dir, chromaParameters);
    const processedPaths = processed.frames.map((frame) => `${processed.processedDir}/${frame.frame}`);
    const normalized = await normalizeFramesWithAnchor(processedPaths, currentJob.job_dir, anchorOverride);
    const report = await computeQualityReport(
      normalized.frames,
      loopRangeOverride === undefined
        ? loopFrameRange(normalized.frames.length, loopStartFrame, loopEndFrame)
        : loopRangeOverride,
    );
    setPreviewResult(preview);
    setNormalizeResult(normalized);
    setSelectedFrameIndex(0);
    setQualityReport(report);
    setExportOutput(null);
    setPackSummary(null);
    setPackPreviewPath(null);
    return { normalizeResult: normalized, qualityReport: report };
  }

  async function exportCurrentPack(
    currentJob = requireValue(job, "import a video first"),
    currentNormalize = requireValue(normalizeResult, "process frames first"),
    currentQuality = requireValue(qualityReport, "compute quality first"),
    metadataOverride: Partial<{
      animationName: string;
      loopRange: LoopFrameRange | null;
      packName: string;
      sourceName: string;
      sourcePath: string;
    }> = {},
  ) {
    const output = await exportPack({
      jobDir: currentJob.job_dir,
      exportsDir: settings.defaultOutputFolder,
      sourcePath: metadataOverride.sourcePath ?? sourcePath,
      frames: currentNormalize.frames,
      qualityReport: currentQuality,
      packName: metadataOverride.packName ?? packName,
      fps: settings.defaultFps,
      sheetSize: settings.defaultSheetSize,
      animationName: metadataOverride.animationName ?? animationName,
      creatorName,
      licenseType,
      loopAnimation,
      loopRange: metadataOverride.loopRange ?? activeLoopRange,
      sheetColumns,
      allowMultiSheet,
      sheetMarginPx,
      sheetPaddingPx,
      sourceKind: currentJob.source_kind as "import_video" | "import_frames" | "import_sprite_sheet" | "import_gsfpack",
      sourceName: metadataOverride.sourceName ?? activeSourceName,
    });
    setExportOutput(output);
    setGsfpackPath(output.packDir);
    setPackSummary(null);
    return output;
  }

  const sideImportPanel = (
    <ImportPanel
      compact
      disabled={isRunning}
      frameSequenceInput={frameSequenceInput}
      gsfpackPath={gsfpackPath}
      onFrameSequenceInputChange={setFrameSequenceInput}
      onGsfpackPathChange={setGsfpackPath}
      onChooseFrameFiles={handleChooseFrameFiles}
      onChooseGsfpackFolder={handleChooseGsfpackFolder}
      onChooseSpriteSheetFile={handleChooseSpriteSheetFile}
      onChooseVideoFile={handleChooseVideoFile}
      onImportFrames={handleImportFrames}
      onImportGsfpack={handleImportGsfpack}
      onImportSpriteSheet={handleImportSpriteSheet}
      onImportVideo={handleImportVideo}
      onLoadSample={handleLoadSample}
      onSpriteGridChange={setSpriteGrid}
      onSpriteSheetSliceModeChange={setSpriteSheetSliceMode}
      onSourcePathChange={setSourcePath}
      onSpriteSheetPathChange={setSpriteSheetPath}
      onTransparentSplitAlphaThresholdChange={setTransparentSplitAlphaThreshold}
      onTransparentSplitMinGapPxChange={setTransparentSplitMinGapPx}
      sourcePath={sourcePath}
      spriteGrid={spriteGrid}
      spriteSheetPath={spriteSheetPath}
      spriteSheetSliceMode={spriteSheetSliceMode}
      t={t}
      transparentSplitAlphaThreshold={transparentSplitAlphaThreshold}
      transparentSplitMinGapPx={transparentSplitMinGapPx}
    />
  );

  return (
    <>
      <aside
        className={[
          "pack-panel",
          activeWorkflow === "Import" ? "import-mode" : "",
          activeWorkflow === "Frames" ? "frame-mode" : "",
        ].filter(Boolean).join(" ")}
      >
        <div className="panel-title pack-title">
          <span>{t("source.current")}</span>
          <span
            aria-label={workspaceMode}
            className={hasLiveData ? "workspace-pill live" : hasSelectedSource ? "workspace-pill selected" : "workspace-pill empty"}
            title={workspaceMode}
          >
            {hasLiveData ? t("source.live") : hasSelectedSource ? t("source.selected") : t("source.empty")}
          </span>
        </div>
        <div className="source-summary-card">
          <strong>{hasSelectedSource ? packName : t("source.noSource")}</strong>
          <span>{hasSelectedSource ? activeSourceName : t("source.chooseSource")}</span>
          <small>
            {hasLiveData
              ? normalizeResult
                ? t("source.processedFramesInWorkspace", { count: frameCount })
                : t("source.rawFramesInWorkspace", { count: frameCount })
              : hasSelectedSource
                ? t("source.awaitingFrames")
                : t("source.emptyNote")}
          </small>
        </div>
        {!hasSelectedSource ? (
          <ActivationRail
            canExport={canExport}
            canProcessAndQuality={canProcessAndQuality}
            disabled={isRunning}
            exportReady={Boolean(exportOutput)}
            ffmpegStatus={ffmpegStatus}
            hasJob={Boolean(job)}
            latestRecentExport={latestRecentExport}
            onCheckToolchain={handleCheckFfmpeg}
            onExport={handleExportPack}
            onOpenSettings={onOpenSettings}
            onProcess={handleProcessAndQuality}
            onReimportRecentExport={onReimportRecentExport}
            onRunSample={handleRunFullPipeline}
            processed={Boolean(normalizeResult)}
            t={t}
          />
        ) : (
          <LiveWorkflowSummary
            canExport={canExport}
            canProcessAndQuality={canProcessAndQuality}
            compact={activeWorkflow === "Frames" && hasSelectedSource && !hasLiveFrames}
            exportReady={Boolean(exportOutput)}
            frameCount={frameCount}
            hasLiveFrames={hasLiveFrames}
            items={runSummaryItems}
            processed={Boolean(normalizeResult)}
            validated={Boolean(packSummary)}
            t={t}
          />
        )}
        {recoveryPlan ? (
          <PipelineRecoveryCard
            onAction={(actionKey) => {
              void recoveryHandlers[actionKey]();
            }}
            plan={recoveryPlan}
          />
        ) : null}
        {activeWorkflow === "Import" && showSideImportPanel ? (
          hasLiveData ? (
            <details className="side-source-settings">
              <summary>
                <span>{t("import.sideChangeSource")}</span>
                <small>{t("import.sideCurrentSource", { name: activeSourceName })}</small>
              </summary>
              {sideImportPanel}
            </details>
          ) : sideImportPanel
        ) : null}
        {activeWorkflow === "Frames" ? (
          <VideoSegmentPanel
            disabled={isRunning || !canExtractFrames}
            endTimeSeconds={endTimeSeconds}
            extractResult={extractResult}
            keepEveryNFrames={keepEveryNFrames}
            loopEndFrame={loopEndFrame}
            loopStartFrame={loopStartFrame}
            onEndTimeChange={setEndTimeSeconds}
            onExtract={handleExtractFrames}
            onKeepEveryChange={setKeepEveryNFrames}
            onLoopEndFrameChange={(value) => setLoopEndFrame(Math.max(value, loopStartFrame))}
            onLoopStartFrameChange={(value) => setLoopStartFrame(Math.min(value, loopEndFrame))}
            onStartTimeChange={setStartTimeSeconds}
            probe={probe}
            startTimeSeconds={startTimeSeconds}
            t={t}
          />
        ) : null}
      </aside>

      <main className="forge-workspace">
        <section className="stage">
          <ChromaPreviewPanel
            activeWorkflow={activeWorkflow}
            activeWorkflowLabel={t(forgeWorkflowLabelKeys[activeWorkflow])}
            disabled={isRunning || !canApplyAnchor}
            framePreviewState={normalizeResult ? "processed" : extractResult ? "raw" : "empty"}
            manualAnchor={manualAnchor}
            onApplyAnchor={handleApplyAnchor}
            onManualAnchorChange={setManualAnchor}
            onParametersChange={setChromaParameters}
            parameters={chromaParameters}
            t={t}
          />
          <WorkflowFocusPanel
            activeWorkflow={activeWorkflow}
            canExport={canExport}
            exportOutput={exportOutput}
            frameCount={frameCount}
            hasQualityWarnings={hasQualityWarnings}
            outputFolder={settings.defaultOutputFolder}
            qualityReport={qualityReport}
            sheetColumns={sheetColumns}
            sheetMarginPx={sheetMarginPx}
            sheetPaddingPx={sheetPaddingPx}
            sheetSize={settings.defaultSheetSize}
            t={t}
          />
          {!hasSelectedSource && activeWorkflow === "Import" ? (
            <ImportLauncher
              disabled={isRunning}
              onChooseFrameFiles={handleChooseFrameFilesAndImport}
              onChooseGsfpack={handleChooseGsfpackAndImport}
              onChooseSpriteSheet={handleChooseSpriteSheetForConfiguration}
              onChooseVideo={sourcePath.trim() ? handleImportVideo : handleChooseVideoAndImport}
              onChangeVideo={handleChooseVideoAndImport}
              onImportSpriteSheet={handleImportSpriteSheet}
              onRunSample={handleRunFullPipeline}
              onSpriteGridChange={setSpriteGrid}
              onSpriteSheetSliceModeChange={setSpriteSheetSliceMode}
              onTransparentSplitAlphaThresholdChange={setTransparentSplitAlphaThreshold}
              onTransparentSplitMinGapPxChange={setTransparentSplitMinGapPx}
              selectedVideoName={sourcePath.trim() ? fileName(sourcePath) : null}
              selectedSpriteSheetName={spriteSheetPath.trim() ? fileName(spriteSheetPath) : null}
              spriteGrid={spriteGrid}
              spriteSheetSliceMode={spriteSheetSliceMode}
              t={t}
              transparentSplitAlphaThreshold={transparentSplitAlphaThreshold}
              transparentSplitMinGapPx={transparentSplitMinGapPx}
            />
          ) : (
            <CanvasPreview
              bboxLabel={canvasFrameTag}
              framePath={selectedFramePath}
              placeholderMode={hasSelectedSource && !hasLiveFrames ? "sourcePending" : "empty"}
              t={t}
            />
          )}
          <div className="stage-scrub" aria-label={t("stage.frameScrubber")}>
            <button
              disabled={!timelineFramePaths.length || selectedFrameIndex <= 0}
              onClick={() => setSelectedFrameIndex((index) => Math.max(0, index - 1))}
              type="button"
            >
              ‹ <span>{t("stage.previousFrame")}</span>
            </button>
            <div className="scrub-line">
              {scrubMarks(frameCount, selectedFrameIndex).map((mark) => (
                <button
                  aria-label={t("stage.selectFrame", { index: mark.index + 1 })}
                  className={`scrub-dot ${mark.active ? "active" : mark.tone}`}
                  disabled={!timelineFramePaths.length}
                  key={mark.index}
                  onClick={() => setSelectedFrameIndex(mark.index)}
                  style={{ left: `${mark.left}%` }}
                  type="button"
                />
              ))}
            </div>
            <button
              disabled={!timelineFramePaths.length || selectedFrameIndex >= timelineFramePaths.length - 1}
              onClick={() => setSelectedFrameIndex((index) => Math.min(timelineFramePaths.length - 1, index + 1))}
              type="button"
            >
              <span>{t("stage.nextFrame")}</span> ›
            </button>
          </div>
        </section>
      </main>

      <section className="lower-workspace">
        <aside className="active-assets-panel">
          <div className="asset-title">
            <span>{t("assets.active")}</span>
            <span>{visibleAssets.length}</span>
          </div>
          <div className="asset-list">
            {visibleAssets.length ? visibleAssets.map((asset) => (
              <article className="asset-card" key={asset.name}>
                <span className={`asset-color ${asset.tone}`} />
                <span className="asset-name">{asset.name}</span>
                <small>{t("assets.frames", { count: asset.frames })}</small>
              </article>
            )) : (
              <div className="asset-empty-state">
                <strong>{t("assets.emptyTitle")}</strong>
                <span>{t("assets.emptyDetail")}</span>
              </div>
            )}
          </div>
        </aside>
        <FrameTimeline
          extractResult={extractResult}
          frameCount={frameCount}
          framePaths={timelineFramePaths}
          normalizeResult={normalizeResult}
          onSelectFrame={setSelectedFrameIndex}
          selectedFrameIndex={selectedFrameIndex}
          state={hasLiveFrames ? "live" : hasSelectedSource ? "sourcePending" : "empty"}
          t={t}
        />
        <aside className="sheet-preview-panel">
          <div className="sheet-head">
            <span>{t("sheet.title")}</span>
            <span className="sheet-size" aria-label={t("sheet.size")}>{settings.defaultSheetSize}x{settings.defaultSheetSize}</span>
          </div>
          {sheetPreviewSrc ? (
            <img
              alt={t("sheet.exportedAlt")}
              className="sheet-image"
              src={sheetPreviewSrc}
            />
          ) : (
            <div className="sheet-empty">
              <strong>{t("sheet.emptyTitle")}</strong>
              <span>{t("sheet.emptyDetail")}</span>
            </div>
          )}
          <div className="sheet-foot">
            <span>{t("sheet.sizeValue", { size: `${settings.defaultSheetSize}x${settings.defaultSheetSize}` })}</span>
            <span>{t("sheet.marginValue", { margin: sheetMarginPx })}</span>
            <span>{t("sheet.paddingValue", { padding: sheetPaddingPx })}</span>
          </div>
        </aside>
      </section>

      <aside className={inspectorPanelClassName}>
        <QualityInspector
          canProcessAndQuality={canProcessAndQuality}
          compact={Boolean(qualityReport && (canExport || exportOutput || activeWorkflow === "Export"))}
          disabled={isRunning}
          framesPendingQuality={framesPendingQuality}
          onOpenAnchor={() => onWorkflowChange("Anchor")}
          onOpenBackground={() => onWorkflowChange("Background")}
          onOpenFrames={() => onWorkflowChange("Frames")}
          onOpenSheet={() => onWorkflowChange("Sheet")}
          onProcess={handleProcessAndQuality}
          onRunSample={handleRunFullPipeline}
          report={qualityReport}
          sourcePendingExtraction={sourcePendingExtraction}
          t={t}
        />
        <ExportPanel
          allowMultiSheet={allowMultiSheet}
          animationName={animationName}
          canExport={canExport}
          creatorName={creatorName}
          disabled={isRunning}
          exportOutput={exportOutput}
          framesPendingQuality={framesPendingQuality}
          licenseType={licenseType}
          loopAnimation={loopAnimation}
          onAnimationNameChange={setAnimationName}
          onAllowMultiSheetChange={setAllowMultiSheet}
          onChooseOutputFolder={onChooseOutputFolder}
          onCreatorNameChange={setCreatorName}
          onExport={handleExportPack}
          onLicenseTypeChange={setLicenseType}
          onLoopAnimationChange={setLoopAnimation}
          onOpenExportFolder={handleOpenExportsFolder}
          onPackNameChange={setPackName}
          onSheetColumnsChange={setSheetColumns}
          onSheetMarginChange={setSheetMarginPx}
          onSheetPaddingChange={setSheetPaddingPx}
          onValidate={handleValidateExport}
          outputFolder={settings.defaultOutputFolder}
          packName={packName}
          packSummary={packSummary}
          readinessItems={exportReadiness}
          sheetColumns={sheetColumns}
          sheetMarginPx={sheetMarginPx}
          sheetPaddingPx={sheetPaddingPx}
          hasSource={Boolean(job)}
          pendingQualityFrameCount={frameCount}
          sourcePendingExtraction={sourcePendingExtraction}
          t={t}
        />
      </aside>

      <footer className="status-bar">
        <div className="footer-pipeline-actions" aria-label={exportOutput ? t("footer.exportStatus") : t("pipeline.actions")}>
          {exportOutput ? (
            <div className="footer-export-status" role="status">
              <PackageCheck size={14} />
              <strong>{t("footer.exported")}</strong>
              <span title={fileName(exportOutput.packDir)}>{fileName(exportOutput.packDir)}</span>
              <small>{packSummary ? t("footer.validated") : t("footer.validationPending")}</small>
            </div>
          ) : (
            <>
              <button className="status-action" disabled={isRunning} onClick={handleCheckFfmpeg} type="button">
                {t("pipeline.checkFfmpeg")}
              </button>
              <button
                className="status-action"
                disabled={isRunning || !canExtractFrames}
                onClick={handleExtractFrames}
                type="button"
              >
                {t("pipeline.extractFrames")}
              </button>
              <button
                className="status-action primary"
                disabled={isRunning || !canProcessAndQuality}
                onClick={handleProcessAndQuality}
                type="button"
              >
                {t("pipeline.processQuality")}
              </button>
            </>
          )}
        </div>
        <div className="pipeline-steps" aria-label={t("pipeline.steps")}>
          <span aria-label={t("pipeline.steps")} className="pipeline-title">{t("pipeline.stepsShort")}</span>
          {pipelineSteps.map((step) => (
            <span
              aria-label={`${step.label}: ${step.stateLabel}`}
              className={`pipeline-step ${step.stateClass}`}
              key={step.label}
            >
              <strong>{step.label}</strong>
              <small>{step.stateLabel}</small>
            </span>
          ))}
        </div>
        <span className="status-spacer" />
        <span>
          {hasLiveData
            ? t("status.frames", { count: frameCount })
            : hasSelectedSource
              ? t("status.framesSelectedSource")
              : t("status.framesEmpty")}
        </span>
        <span>{t("status.size", { size: `${settings.defaultSheetSize}x${settings.defaultSheetSize}` })}</span>
        <span>{t("status.fps", { fps: settings.defaultFps })}</span>
        <span>{durationSeconds === null ? t("status.durationPending") : t("status.duration", { duration: formatSecondsCompact(durationSeconds) })}</span>
        <span className={status.tone === "error" ? "status-error" : status.tone === "success" ? "status-pass" : ""}>
          {status.tone === "idle" ? "" : status.message}
        </span>
        <span className={visibleQualityStatus.className}>
          <CheckCircle2 size={14} />
          {visibleQualityStatus.label}
        </span>
        <span>v{APP_VERSION}</span>
      </footer>
    </>
  );
}

function WorkflowFocusPanel({
  activeWorkflow,
  canExport,
  exportOutput,
  frameCount,
  hasQualityWarnings,
  outputFolder,
  qualityReport,
  sheetColumns,
  sheetMarginPx,
  sheetPaddingPx,
  sheetSize,
  t,
}: {
  activeWorkflow: ForgeWorkflow;
  canExport: boolean;
  exportOutput: ExportPackOutput | null;
  frameCount: number;
  hasQualityWarnings: boolean;
  outputFolder: string;
  qualityReport: QualityReport | null;
  sheetColumns: number;
  sheetMarginPx: number;
  sheetPaddingPx: number;
  sheetSize: number;
  t: TFunction;
}) {
  if (activeWorkflow !== "Sheet" && activeWorkflow !== "Export") {
    return null;
  }

  if (activeWorkflow === "Sheet") {
    return (
      <section className="workflow-focus-panel sheet-focus" aria-label={t("workflowFocus.sheet.title")}>
        <span>{t("workflowFocus.sheet.title")}</span>
        <strong>{t("workflowFocus.sheet.detail", { count: frameCount })}</strong>
        <div>
          <small>{t("sheet.sizeValue", { size: `${sheetSize}x${sheetSize}` })}</small>
          <small>{t("workflowFocus.sheet.columns", { count: sheetColumns })}</small>
          <small>{t("sheet.marginValue", { margin: sheetMarginPx })}</small>
          <small>{t("sheet.paddingValue", { padding: sheetPaddingPx })}</small>
        </div>
      </section>
    );
  }

  const qualityCopy = qualityReport
    ? qualityReport.verdict === "blocked"
      ? t("quality.status.needsAttention")
      : hasQualityWarnings
        ? t("quality.status.hasWarnings", { count: qualityReport.recommendations.length })
        : t("quality.status.allPassed")
    : t("quality.status.pending");

  return (
    <section className="workflow-focus-panel export-focus" aria-label={t("workflowFocus.export.title")}>
      <span>{t("workflowFocus.export.title")}</span>
      <strong>
        {exportOutput
          ? t("workflowFocus.export.exported")
          : canExport
            ? t("workflowFocus.export.ready")
            : t("workflowFocus.export.blocked")}
      </strong>
      <div>
        <small>{qualityCopy}</small>
        <small>{t("workflowFocus.export.frames", { count: frameCount })}</small>
        <small>{outputFolder}</small>
      </div>
    </section>
  );
}

function ImportLauncher({
  disabled,
  onChangeVideo,
  onChooseFrameFiles,
  onChooseGsfpack,
  onChooseSpriteSheet,
  onChooseVideo,
  onImportSpriteSheet,
  onRunSample,
  onSpriteGridChange,
  onSpriteSheetSliceModeChange,
  onTransparentSplitAlphaThresholdChange,
  onTransparentSplitMinGapPxChange,
  selectedSpriteSheetName,
  selectedVideoName,
  spriteGrid,
  spriteSheetSliceMode,
  t,
  transparentSplitAlphaThreshold,
  transparentSplitMinGapPx,
}: {
  disabled: boolean;
  onChangeVideo: () => void;
  onChooseFrameFiles: () => void;
  onChooseGsfpack: () => void;
  onChooseSpriteSheet: () => void;
  onChooseVideo: () => void;
  onImportSpriteSheet: () => void;
  onRunSample: () => void;
  onSpriteGridChange: (grid: SpriteGrid) => void;
  onSpriteSheetSliceModeChange: (mode: SpriteSheetSliceMode) => void;
  onTransparentSplitAlphaThresholdChange: (value: number) => void;
  onTransparentSplitMinGapPxChange: (value: number) => void;
  selectedSpriteSheetName: string | null;
  selectedVideoName: string | null;
  spriteGrid: SpriteGrid;
  spriteSheetSliceMode: SpriteSheetSliceMode;
  t: TFunction;
  transparentSplitAlphaThreshold: number;
  transparentSplitMinGapPx: number;
}) {
  const hasSelectedSpriteSheet = Boolean(selectedSpriteSheetName);
  const sourceActions = [
    { icon: Images, label: t("importLauncher.png"), onClick: onChooseFrameFiles },
    { icon: FileImage, label: t("importLauncher.spriteSheet"), onClick: onChooseSpriteSheet },
    { icon: FileArchive, label: t("importLauncher.gsfpack"), onClick: onChooseGsfpack },
  ];
  const steps = [
    { index: 1, label: t("importLauncher.step.import") },
    { index: 2, label: t("importLauncher.step.process") },
    { index: 3, label: t("importLauncher.step.export") },
  ];

  return (
    <section className="import-launcher" aria-label={t("importLauncher.title")}>
      <div className="import-launcher-copy">
        <span className="import-launcher-kicker">{t("importLauncher.kicker")}</span>
        <h2>{hasSelectedSpriteSheet ? t("importLauncher.spriteSheetReadyTitle") : t("importLauncher.title")}</h2>
        <p>{hasSelectedSpriteSheet ? t("importLauncher.spriteSheetReadyDetail") : t("importLauncher.detail")}</p>
      </div>

      <div className="import-launcher-dropzone">
        {hasSelectedSpriteSheet ? (
          <div className="import-launcher-sprite-config">
            <div className="import-launcher-selected-source">
              <FileImage size={18} />
              <div>
                <span>{t("importLauncher.selectedSpriteSheet")}</span>
                <strong title={selectedSpriteSheetName ?? undefined}>{selectedSpriteSheetName}</strong>
              </div>
            </div>
            <div className="sprite-split-mode import-launcher-sprite-mode" role="group" aria-label={t("import.spriteSheetSplitMode")}>
              <button
                className={spriteSheetSliceMode === "grid" ? "active" : ""}
                disabled={disabled}
                onClick={() => onSpriteSheetSliceModeChange("grid")}
                type="button"
              >
                {t("import.spriteSheetSplitGrid")}
              </button>
              <button
                className={spriteSheetSliceMode === "transparent" ? "active" : ""}
                disabled={disabled}
                onClick={() => onSpriteSheetSliceModeChange("transparent")}
                type="button"
              >
                {t("import.spriteSheetSplitTransparent")}
              </button>
            </div>
            {spriteSheetSliceMode === "grid" ? (
              <div className="import-launcher-fields">
                <LauncherNumberField
                  label="W"
                  onChange={(value) => onSpriteGridChange({ ...spriteGrid, frameWidth: value })}
                  value={spriteGrid.frameWidth}
                />
                <LauncherNumberField
                  label="H"
                  onChange={(value) => onSpriteGridChange({ ...spriteGrid, frameHeight: value })}
                  value={spriteGrid.frameHeight}
                />
                <LauncherNumberField
                  label={t("import.grid.columns")}
                  onChange={(value) => onSpriteGridChange({ ...spriteGrid, columns: value })}
                  value={spriteGrid.columns}
                />
                <LauncherNumberField
                  label={t("import.grid.rows")}
                  onChange={(value) => onSpriteGridChange({ ...spriteGrid, rows: value })}
                  value={spriteGrid.rows}
                />
              </div>
            ) : (
              <div className="import-launcher-fields compact">
                <LauncherNumberField
                  label={t("import.transparent.alpha")}
                  max={255}
                  min={0}
                  onChange={onTransparentSplitAlphaThresholdChange}
                  value={transparentSplitAlphaThreshold}
                />
                <LauncherNumberField
                  label={t("import.transparent.gap")}
                  min={1}
                  onChange={onTransparentSplitMinGapPxChange}
                  value={transparentSplitMinGapPx}
                />
              </div>
            )}
            <div className="import-launcher-sprite-actions">
              <button
                aria-label={selectedSpriteSheetName ? t("importLauncher.importSpriteSheetAria", { name: selectedSpriteSheetName }) : t("importLauncher.importSpriteSheet")}
                className="import-launcher-confirm"
                disabled={disabled}
                onClick={onImportSpriteSheet}
                type="button"
              >
                <FileImage size={18} />
                {t("importLauncher.importSpriteSheet")}
              </button>
              <button className="import-launcher-link" disabled={disabled} onClick={onChooseSpriteSheet} type="button">
                {t("importLauncher.changeSpriteSheet")}
              </button>
            </div>
          </div>
        ) : (
          <>
            <button
              aria-label={selectedVideoName ? t("importLauncher.importSelectedAria", { name: selectedVideoName }) : t("importLauncher.chooseVideo")}
              className="import-launcher-primary"
              disabled={disabled}
              onClick={onChooseVideo}
              type="button"
            >
              <Film size={28} />
              <span>{selectedVideoName ? t("importLauncher.importSelected") : t("importLauncher.chooseVideo")}</span>
            </button>
            <small>
              {selectedVideoName
                ? t("importLauncher.selectedVideo", { name: selectedVideoName })
                : t("importLauncher.videoHint")}
            </small>
            {selectedVideoName ? (
              <button className="import-launcher-link" disabled={disabled} onClick={onChangeVideo} type="button">
                {t("importLauncher.changeVideo")}
              </button>
            ) : null}
          </>
        )}

        <div className="import-launcher-divider">
          <span>{t("importLauncher.otherSources")}</span>
        </div>
        <div className="import-source-actions">
          {sourceActions.map((action) => {
            const Icon = action.icon;
            return (
              <button disabled={disabled} key={action.label} onClick={action.onClick} type="button">
                <Icon size={17} />
                {action.label}
              </button>
            );
          })}
        </div>
        <button className="import-sample-link" disabled={disabled} onClick={onRunSample} type="button">
          <PlayCircle size={15} />
          {t("importLauncher.runSample")}
        </button>
      </div>

      <div className="import-launcher-steps" aria-label={t("pipeline.steps")}>
        {steps.map((step) => (
          <span key={step.index}>
            <strong>{step.index}</strong>
            {step.label}
          </span>
        ))}
      </div>
    </section>
  );
}

function LauncherNumberField({
  label,
  max,
  min = 1,
  onChange,
  value,
}: {
  label: string;
  max?: number;
  min?: number;
  onChange: (value: number) => void;
  value: number;
}) {
  return (
    <label className="import-launcher-number-field">
      <span>{label}</span>
      <input
        max={max}
        min={min}
        onChange={(event) => onChange(launcherNumberValue(event.target.value, value, min, max))}
        step={1}
        type="number"
        value={value}
      />
    </label>
  );
}

function launcherNumberValue(value: string, fallback: number, min: number, max?: number) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  const lowerBounded = Math.max(min, parsed);
  return typeof max === "number" ? Math.min(max, lowerBounded) : lowerBounded;
}

function ActivationRail({
  canExport,
  canProcessAndQuality,
  disabled,
  exportReady,
  ffmpegStatus,
  hasJob,
  latestRecentExport,
  onCheckToolchain,
  onExport,
  onOpenSettings,
  onProcess,
  onReimportRecentExport,
  onRunSample,
  processed,
  t,
}: {
  canExport: boolean;
  canProcessAndQuality: boolean;
  disabled: boolean;
  exportReady: boolean;
  ffmpegStatus: DependencyStatus | null;
  hasJob: boolean;
  latestRecentExport: { output: ExportPackOutput; packName: string } | null;
  onCheckToolchain: () => void;
  onExport: () => void;
  onOpenSettings: () => void;
  onProcess: () => void;
  onReimportRecentExport: () => void;
  onRunSample: () => void;
  processed: boolean;
  t: TFunction;
}) {
  const toolchainWarning = Boolean(ffmpegStatus?.message);
  const toolchainDone = Boolean(ffmpegStatus && !ffmpegStatus.message);
  const steps = [
    {
      label: t("firstRun.checkToolchain"),
      state: toolchainWarning ? "warning" : toolchainDone ? "done" : "ready",
    },
    {
      label: t("firstRun.selectSource"),
      state: hasJob ? "done" : "pending",
    },
    {
      label: t("firstRun.processFrames"),
      state: processed ? "done" : canProcessAndQuality ? "ready" : "pending",
    },
    {
      label: t("firstRun.exportPack"),
      state: exportReady ? "done" : canExport ? "ready" : "pending",
    },
  ];

  return (
    <section className="activation-rail" aria-label={t("firstRun.title")}>
      <div className="activation-head">
        <span>{t("firstRun.title")}</span>
        <small>{t("firstRun.detail")}</small>
      </div>
      <div className="activation-steps">
        {steps.map((step) => (
          <span className={`activation-step ${step.state}`} key={step.label}>
            <ActivationIcon state={step.state} />
            {step.label}
          </span>
        ))}
      </div>
      <div className="activation-actions">
        <button className="status-action" disabled={disabled} onClick={onCheckToolchain} type="button">
          <Wrench size={13} />
          {t("firstRun.checkToolchain")}
        </button>
        <button className="status-action sample-secondary" disabled={disabled} onClick={onRunSample} type="button">
          <PlayCircle size={13} />
          {t("firstRun.runSample")}
        </button>
        {latestRecentExport ? (
          <button
            className="status-action"
            disabled={disabled}
            onClick={onReimportRecentExport}
            title={t("firstRun.reimportRecentHint", { name: latestRecentExport.packName })}
            type="button"
          >
            <PackageCheck size={13} />
            {t("firstRun.reimportRecent")}
          </button>
        ) : null}
        {canProcessAndQuality ? (
          <button className="status-action" disabled={disabled} onClick={onProcess} type="button">
            {t("firstRun.processNow")}
          </button>
        ) : null}
        {canExport ? (
          <button className="status-action" disabled={disabled} onClick={onExport} type="button">
            {t("firstRun.exportPack")}
          </button>
        ) : null}
      </div>
      {toolchainWarning ? (
        <div className="toolchain-recovery">
          <AlertTriangle size={15} />
          <span>
            <strong>{t("firstRun.toolchainNeedsAttention")}</strong>
            <small>{ffmpegStatus?.message}</small>
          </span>
          <button className="details-link" onClick={onOpenSettings} type="button">
            <SettingsIcon size={13} />
            {t("firstRun.openSettings")}
          </button>
        </div>
      ) : null}
      <div className="keyboard-shortcuts" aria-label={t("firstRun.keyboardShortcuts")}>
        <span>{t("firstRun.keyboardShortcuts")}</span>
        <kbd>{t("firstRun.shortcutProcess")}</kbd>
        <kbd>{t("firstRun.shortcutExport")}</kbd>
        <kbd>{t("firstRun.shortcutFrames")}</kbd>
      </div>
    </section>
  );
}

function LiveWorkflowSummary({
  canExport,
  canProcessAndQuality,
  compact = false,
  exportReady,
  frameCount,
  hasLiveFrames,
  items,
  processed,
  validated,
  t,
}: {
  canExport: boolean;
  canProcessAndQuality: boolean;
  compact?: boolean;
  exportReady: boolean;
  frameCount: number;
  hasLiveFrames: boolean;
  items: RunSummaryItem[];
  processed: boolean;
  validated: boolean;
  t: TFunction;
}) {
  const stateLabel = validated
    ? t("summary.state.exportValidated")
    : exportReady
      ? t("summary.state.exportAwaitingValidation")
    : canExport
      ? t("summary.state.readyToExport")
      : processed
        ? t("summary.state.qualityReady")
        : canProcessAndQuality
          ? t("summary.state.readyProcessQuality")
          : t("summary.state.extractBeforeProcessing");
  const visibleStateLabel = compact && !hasLiveFrames ? t("summary.state.awaitingExtraction") : stateLabel;

  return (
    <section className={compact ? "live-workflow-summary compact" : "live-workflow-summary"} aria-label={t("summary.title")}>
      <span>{t("summary.title")}</span>
      <strong>{visibleStateLabel}</strong>
      <small>
        {compact && !hasLiveFrames
          ? t("summary.compactAwaitingFrames", { count: frameCount })
          : hasLiveFrames
            ? t("summary.framesInActive", { count: frameCount })
            : t("summary.framesAwaitingExtraction")}
      </small>
      {compact ? null : (
        <div className="run-summary-grid">
          {items.map((item) => (
            <div className={`run-summary-item ${item.tone}`} key={item.label}>
              <span>{item.label}</span>
              <strong title={item.value}>{item.value}</strong>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function PipelineRecoveryCard({
  onAction,
  plan,
}: {
  onAction: (actionKey: RecoveryActionKey) => void;
  plan: RecoveryPlan;
}) {
  return (
    <section className="pipeline-recovery-card" aria-label="Recovery guidance">
      <div>
        <AlertTriangle size={15} />
        <span>
          <strong>{plan.title}</strong>
          <small>{plan.detail}</small>
        </span>
      </div>
      <div className="recovery-actions">
        {plan.actions.map((action) => (
          <button
            className="status-action"
            disabled={action.disabled}
            key={action.key}
            onClick={() => onAction(action.key)}
            type="button"
          >
            {action.label}
          </button>
        ))}
      </div>
    </section>
  );
}

function ActivationIcon({ state }: { state: string }) {
  if (state === "done") {
    return <CheckCircle2 size={13} />;
  }
  if (state === "warning") {
    return <AlertTriangle size={13} />;
  }
  return <Circle size={12} />;
}

function recoveryPlanFor(
  message: string,
  capability: { canExport: boolean; canExtractFrames: boolean; canProcessAndQuality: boolean },
  t: TFunction,
): RecoveryPlan {
  const lower = message.toLowerCase();

  if (/(invalid data found when processing input|ffprobe did not return a video stream|not a video|invalid video)/.test(lower)) {
    return {
      actions: [
        { key: "import", label: t("firstRun.selectSource") },
        { key: "frames", label: t("pipeline.step.extract") },
      ],
      detail: t("recovery.videoPath.detail"),
      title: t("recovery.videoPath.title"),
    };
  }

  if (/(ffmpeg|ffprobe|toolchain)/.test(lower)) {
    return {
      actions: [
        { key: "settings", label: t("firstRun.openSettings") },
        { key: "checkToolchain", label: t("firstRun.checkToolchain") },
      ],
      detail: t("recovery.toolchain.detail"),
      title: t("recovery.toolchain.title"),
    };
  }

  if (/(choose|path|source|file|folder|not found|no such|invalid)/.test(lower)) {
    return {
      actions: [
        { key: "import", label: t("firstRun.selectSource") },
        { key: "settings", label: t("firstRun.openSettings") },
      ],
      detail: t("recovery.localPath.detail"),
      title: t("recovery.localPath.title"),
    };
  }

  if (/(exceeds max texture size|texture too large|max texture)/.test(lower)) {
    return {
      actions: [
        { key: "exportReview", label: t("workflow.export") },
        { key: "sheet", label: t("workflow.sheet") },
      ],
      detail: t("recovery.sheetTooLarge.detail"),
      title: t("recovery.sheetTooLarge.title"),
    };
  }

  if (/(sprite sheet|grid|slice|cell|atlas)/.test(lower)) {
    return {
      actions: [
        { key: "import", label: t("workflow.import") },
        { key: "frames", label: t("workflow.frames") },
      ],
      detail: t("recovery.sheetMismatch.detail"),
      title: t("recovery.sheetMismatch.title"),
    };
  }

  if (/(extract|raw frame|video)/.test(lower)) {
    return {
      actions: [
        { disabled: !capability.canExtractFrames, key: "extract", label: t("pipeline.extractFrames") },
        { key: "frames", label: t("workflow.frames") },
      ],
      detail: t("recovery.framesNotReady.detail"),
      title: t("recovery.framesNotReady.title"),
    };
  }

  if (/(chroma|background|alpha|matte|normalize|anchor)/.test(lower)) {
    return {
      actions: [
        { key: "background", label: t("workflow.background") },
        { disabled: !capability.canProcessAndQuality, key: "process", label: t("pipeline.processQuality") },
      ],
      detail: t("recovery.processingPass.detail"),
      title: t("recovery.processingPass.title"),
    };
  }

  if (/(quality|process)/.test(lower)) {
    return {
      actions: [
        { disabled: !capability.canProcessAndQuality, key: "process", label: t("pipeline.processQuality") },
        { key: "frames", label: t("workflow.frames") },
      ],
      detail: t("recovery.qualityNotReady.detail"),
      title: t("recovery.qualityNotReady.title"),
    };
  }

  if (/(export|write|permission|output)/.test(lower)) {
    return {
      actions: [
        { disabled: !capability.canExport, key: "export", label: t("export.exportPack") },
        { key: "settings", label: t("settings.defaultOutputFolder") },
      ],
      detail: t("recovery.exportBlocked.detail"),
      title: t("recovery.exportBlocked.title"),
    };
  }

  return {
    actions: [
      { key: "import", label: t("workflow.import") },
      { disabled: !capability.canProcessAndQuality, key: "process", label: t("pipeline.processQuality") },
    ],
    detail: message || t("recovery.pipelineFailed.detail"),
    title: t("recovery.pipelineFailed.title"),
  };
}

function sourceKindLabel(value: string, t: TFunction) {
  switch (value) {
    case "import_video":
      return t("export.source.video");
    case "import_frames":
      return t("export.source.pngSequence");
    case "import_sprite_sheet":
      return t("export.source.spriteSheet");
    case "import_gsfpack":
      return t("export.source.gsfpack");
    default:
      return labelize(value);
  }
}

function verdictLabel(value: QualityReport["verdict"], t: TFunction) {
  if (value === "game_ready") {
    return t("quality.verdict.gameReady");
  }
  if (value === "needs_cleanup") {
    return t("quality.verdict.needsCleanup");
  }
  if (value === "prototype_usable") {
    return t("quality.verdict.prototypeUsable");
  }
  if (value === "blocked") {
    return t("quality.verdict.blocked");
  }
  return labelize(value);
}

function requiredPath(value: string, label: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error(`choose a ${label}`);
  }
  return trimmed;
}

function requireValue<T>(value: T | null | undefined, message: string): T {
  if (!value) {
    throw new Error(message);
  }
  return value;
}

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) || target.isContentEditable;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() || path;
}

function firstInputPath(value: string) {
  return value
    .split(/[\n,]+/)
    .map((part) => part.trim())
    .find(Boolean);
}

function fileExtension(path: string) {
  const name = fileName(path);
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1) : "png";
}

function jobSourcePath(job: JobRecord, originalPath: string) {
  return `${job.job_dir}/source/${fileName(originalPath)}`;
}

function labelize(value: string) {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatSecondsCompact(value: number) {
  if (!Number.isFinite(value) || value < 0) {
    return "pending";
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)}s`;
}

function loopFrameRange(frameCount: number, startFrame: number, endFrame: number): LoopFrameRange | null {
  if (frameCount < 2) {
    return null;
  }
  const startIndex = Math.max(0, Math.min(frameCount - 1, Math.round(startFrame) - 1));
  const endIndex = Math.max(startIndex, Math.min(frameCount - 1, Math.round(endFrame) - 1));
  if (startIndex >= endIndex) {
    return null;
  }
  return { startIndex, endIndex };
}

function scrubMarks(frameCount: number, selectedFrameIndex: number) {
  if (frameCount <= 0) {
    return [];
  }
  const count = Math.max(1, frameCount);
  const visibleMarkCount = Math.min(24, count);
  return Array.from({ length: visibleMarkCount }, (_, markIndex) => {
    const index = visibleMarkCount === 1 ? 0 : Math.round((markIndex / (visibleMarkCount - 1)) * (count - 1));
    const active = Math.abs(index - selectedFrameIndex) <= Math.max(0, Math.floor(count / visibleMarkCount / 2));
    const tone = markIndex === 0 ? "blue" : markIndex === visibleMarkCount - 1 ? "end" : markIndex % 5 === 0 ? "pink" : "mid";
    return {
      active,
      index,
      left: visibleMarkCount === 1 ? 50 : (markIndex / (visibleMarkCount - 1)) * 100,
      tone,
    };
  });
}
