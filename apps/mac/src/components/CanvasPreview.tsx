import { useId, type CSSProperties } from "react";
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
  const inspectionOverlayId = useId();
  const previewModeStatusId = useId();
  const imageSrc = usePreviewImage(framePath, null);
  const hasPipelineFrame = Boolean(framePath);
  const isSourcePending = placeholderMode === "sourcePending";
  const placeholderTitle = isSourcePending ? t("stage.sourcePendingTitle") : t("stage.emptyTitle");
  const placeholderDetail = isSourcePending ? t("stage.sourcePendingDetail") : t("stage.emptyDetail");
  const placeholderTag = isSourcePending ? t("stage.sourcePendingTag") : t("stage.emptyTag");
  const hasOverlay = Boolean(overlay);
  const previewModeLabel = t(previewModeLabelKey(previewMode));
  const previewProcessingState = previewProcessingStateFor(previewMode);
  const inspectionStateLabel = inspectionEnabled ? t("stage.inspectionOn") : t("stage.inspectionOff");
  const overlayLabel = bboxLabel ?? previewModeLabel;
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
      aria-describedby={previewModeStatusId}
      data-inspection-overlay={hasOverlay ? "visible" : "hidden"}
      data-inspection-state={inspectionEnabled ? "on" : "off"}
      data-preview-mode={previewMode}
      data-preview-processing-state={previewProcessingState}
    >
      <div className="preview-mode-control" role="group" aria-label={t("stage.frameInspection")}>
        <span
          aria-label={previewModeLabel}
          className="preview-mode-label"
          data-preview-mode-label={previewMode}
          data-preview-processing-state={previewProcessingState}
          id={previewModeStatusId}
          title={previewModeLabel}
        >
          {previewModeLabel}
        </span>
        {onInspectionToggle ? (
          <button
            aria-controls={inspectionOverlayId}
            aria-label={`${previewModeLabel}: ${inspectionStateLabel}`}
            aria-describedby={previewModeStatusId}
            aria-pressed={inspectionEnabled}
            onClick={() => onInspectionToggle(!inspectionEnabled)}
            type="button"
          >
            {inspectionStateLabel}
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
          {overlay ? (
            <PreviewMeasureOverlays
              overlay={overlay}
              overlayId={inspectionOverlayId}
              overlayLabel={overlayLabel}
            />
          ) : null}
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
      {!overlay ? (
        <span
          data-inspection-overlay="hidden"
          id={inspectionOverlayId}
          style={screenReaderOnlyStyle}
        >
          {inspectionStateLabel}
        </span>
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

function previewProcessingStateFor(mode: PreviewMode) {
  const states = {
    empty: "empty",
    export: "export",
    inspection: "inspection",
    normalized: "after-processing",
    raw: "before-processing",
  } as const;
  return states[mode];
}

function PreviewMeasureOverlays({
  overlay,
  overlayId,
  overlayLabel,
}: {
  overlay: PreviewOverlay;
  overlayId: string;
  overlayLabel: string;
}) {
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
    <div
      aria-label={overlayLabel}
      className="preview-measure-overlays"
      data-inspection-overlay="visible"
      id={overlayId}
      role="img"
    >
      <span aria-hidden="true" className="preview-bbox" data-overlay-part="bbox" style={bboxStyle} />
      <span aria-hidden="true" className="preview-anchor-line" data-overlay-part="anchor-line" style={anchorLineStyle} />
      <span aria-hidden="true" className="preview-center-line" data-overlay-part="center-line" style={centerLineStyle} />
      <span aria-hidden="true" className="preview-anchor-node" data-overlay-part="anchor-node" style={anchorStyle} />
    </div>
  );
}

function percent(value: number, total: number) {
  const safeValue = Number.isFinite(value) ? value : 0;
  return `${Math.max(0, Math.min(100, (safeValue / total) * 100))}%`;
}

const screenReaderOnlyStyle: CSSProperties = {
  border: 0,
  clip: "rect(0 0 0 0)",
  height: 1,
  margin: -1,
  overflow: "hidden",
  padding: 0,
  position: "absolute",
  whiteSpace: "nowrap",
  width: 1,
};
