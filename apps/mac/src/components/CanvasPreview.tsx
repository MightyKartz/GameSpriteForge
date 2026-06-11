import { usePreviewImage } from "../hooks/usePreviewImage";
import type { TFunction } from "../i18n";

type CanvasPreviewProps = {
  bboxLabel?: string;
  framePath?: string | null;
  placeholderMode: "empty" | "sourcePending";
  t: TFunction;
};

export function CanvasPreview({ bboxLabel, framePath, placeholderMode, t }: CanvasPreviewProps) {
  const imageSrc = usePreviewImage(framePath, null);
  const hasPipelineFrame = Boolean(framePath);
  const isSourcePending = placeholderMode === "sourcePending";
  const placeholderTitle = isSourcePending ? t("stage.sourcePendingTitle") : t("stage.emptyTitle");
  const placeholderDetail = isSourcePending ? t("stage.sourcePendingDetail") : t("stage.emptyDetail");
  const placeholderTag = isSourcePending ? t("stage.sourcePendingTag") : t("stage.emptyTag");

  return (
    <div
      className={[
        "canvas-preview",
        hasPipelineFrame ? "" : "design-preview",
        isSourcePending ? "source-pending-preview" : "",
        !hasPipelineFrame && !isSourcePending ? "empty-preview" : "",
      ].filter(Boolean).join(" ")}
      aria-label={t("stage.spritePreview")}
    >
      {!hasPipelineFrame ? (
        <div className="canvas-demo-banner">
          <strong>{placeholderTitle}</strong>
          <span>{placeholderDetail}</span>
        </div>
      ) : null}
      {imageSrc ? (
        <img alt={t("stage.selectedFrameAlt")} className="sprite-frame-image" src={imageSrc} />
      ) : (
        <div aria-hidden="true" className="empty-workspace-visual">
          <span />
          <span />
          <span />
        </div>
      )}
      <div className="center-line" />
      <div className="foot-line" />
      <span className="frame-tag">{bboxLabel ?? placeholderTag}</span>
    </div>
  );
}
