import type { CSSProperties } from "react";
import { usePreviewImage } from "../hooks/usePreviewImage";
import type { TFunction } from "../i18n";
import type { FootAnchor, FrameBbox, FrameSize } from "../tauriCommands";

type PreviewOverlay = {
  anchor: FootAnchor;
  bbox: FrameBbox;
  size: FrameSize;
};

type PreviewMode = "empty" | "export" | "inspection" | "normalized" | "raw";

type CanvasPreviewProps = {
  bboxLabel?: string;
  framePath?: string | null;
  inspectionEnabled?: boolean;
  onInspectionToggle?: (enabled: boolean) => void;
  overlay?: PreviewOverlay | null;
  placeholderMode: "empty" | "sourcePending";
  previewMode: PreviewMode;
  t: TFunction;
};

export function CanvasPreview({
  bboxLabel,
  framePath,
  inspectionEnabled = false,
  onInspectionToggle,
  overlay = null,
  placeholderMode,
  previewMode,
  t,
}: CanvasPreviewProps) {
  const imageSrc = usePreviewImage(framePath, null);
  const hasPipelineFrame = Boolean(framePath);
  const isSourcePending = placeholderMode === "sourcePending";
  const placeholderTitle = isSourcePending ? t("stage.sourcePendingTitle") : t("stage.emptyTitle");
  const placeholderDetail = isSourcePending ? t("stage.sourcePendingDetail") : t("stage.emptyDetail");
  const placeholderTag = isSourcePending ? t("stage.sourcePendingTag") : t("stage.emptyTag");
  const hasOverlay = Boolean(overlay);
  const stageStyle = overlay
    ? ({ "--preview-aspect": `${Math.max(1, overlay.size.width)} / ${Math.max(1, overlay.size.height)}` } as CSSProperties)
    : undefined;

  return (
    <div
      className={[
        "canvas-preview",
        `preview-mode-${previewMode}`,
        hasOverlay ? "has-measure-overlay" : "",
        inspectionEnabled ? "inspection-enabled" : "inspection-disabled",
        hasPipelineFrame ? "" : "design-preview",
        isSourcePending ? "source-pending-preview" : "",
        !hasPipelineFrame && !isSourcePending ? "empty-preview" : "",
      ].filter(Boolean).join(" ")}
      aria-label={t("stage.spritePreview")}
    >
      <div className="preview-mode-control">
        <span className="preview-mode-label">{t(previewModeLabelKey(previewMode))}</span>
        {onInspectionToggle ? (
          <button
            aria-pressed={inspectionEnabled}
            onClick={() => onInspectionToggle(!inspectionEnabled)}
            type="button"
          >
            {inspectionEnabled ? t("stage.inspectionOn") : t("stage.inspectionOff")}
          </button>
        ) : null}
      </div>
      {!hasPipelineFrame ? (
        <div className="canvas-demo-banner">
          <strong>{placeholderTitle}</strong>
          <span>{placeholderDetail}</span>
        </div>
      ) : null}
      {imageSrc ? (
        <div className={["preview-frame-stage", hasOverlay ? "has-overlay" : ""].filter(Boolean).join(" ")} style={stageStyle}>
          <img alt={t("stage.selectedFrameAlt")} className="sprite-frame-image" src={imageSrc} />
          {overlay ? <PreviewMeasureOverlays overlay={overlay} /> : null}
        </div>
      ) : (
        <div aria-hidden="true" className="empty-workspace-visual">
          <span />
          <span />
          <span />
        </div>
      )}
      {!overlay && !hasPipelineFrame ? (
        <>
          <div className="center-line" />
          <div className="foot-line" />
        </>
      ) : null}
      <span className="frame-tag">{bboxLabel ?? placeholderTag}</span>
    </div>
  );
}

function previewModeLabelKey(mode: PreviewMode) {
  const keys = {
    empty: "stage.previewMode.empty",
    export: "stage.previewMode.export",
    inspection: "stage.previewMode.inspection",
    normalized: "stage.previewMode.normalized",
    raw: "stage.previewMode.raw",
  } as const;
  return keys[mode];
}

function PreviewMeasureOverlays({ overlay }: { overlay: PreviewOverlay }) {
  const safeWidth = Math.max(1, overlay.size.width);
  const safeHeight = Math.max(1, overlay.size.height);
  const bboxStyle = {
    height: percent(overlay.bbox.height, safeHeight),
    left: percent(overlay.bbox.left, safeWidth),
    top: percent(overlay.bbox.top, safeHeight),
    width: percent(overlay.bbox.width, safeWidth),
  };
  const anchorStyle = {
    left: percent(overlay.anchor.x, safeWidth),
    top: percent(overlay.anchor.y, safeHeight),
  };
  const anchorLineStyle = {
    top: percent(overlay.anchor.y, safeHeight),
  };
  const centerLineStyle = {
    left: percent(overlay.bbox.centerX, safeWidth),
  };

  return (
    <div aria-hidden="true" className="preview-measure-overlays">
      <span className="preview-bbox" style={bboxStyle} />
      <span className="preview-anchor-line" style={anchorLineStyle} />
      <span className="preview-center-line" style={centerLineStyle} />
      <span className="preview-anchor-node" style={anchorStyle} />
    </div>
  );
}

function percent(value: number, total: number) {
  const safeValue = Number.isFinite(value) ? value : 0;
  return `${Math.max(0, Math.min(100, (safeValue / total) * 100))}%`;
}
