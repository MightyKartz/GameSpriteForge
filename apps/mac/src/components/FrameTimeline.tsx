import type { CSSProperties } from "react";
import type { ExtractFramesResult, NormalizeFramesResult } from "../tauriCommands";
import { usePreviewImage } from "../hooks/usePreviewImage";
import type { TFunction } from "../i18n";

type FrameTimelineProps = {
  extractResult: ExtractFramesResult | null;
  frameCount: number;
  framePaths: string[];
  normalizeResult: NormalizeFramesResult | null;
  onSelectFrame: (index: number) => void;
  selectedFrameIndex: number;
  state: "empty" | "live" | "sourcePending";
  t: TFunction;
};

export function FrameTimeline({
  extractResult,
  frameCount,
  framePaths,
  normalizeResult,
  onSelectFrame,
  selectedFrameIndex,
  state,
  t,
}: FrameTimelineProps) {
  const frameLabel = normalizeResult ? t("timeline.normalized") : extractResult ? t("timeline.raw") : t("timeline.timeline");
  const hasTimelineFrames = state === "live" && framePaths.length > 0;
  const renderedTracks = [
    {
      name: frameLabel,
      frames: frameCount,
      tone: "blue" as const,
      visible: Math.max(1, Math.min(framePaths.length, 24)),
      barLabel: String(frameCount),
    },
  ];
  const rulerValues = frameCount > 0 ? timelineMarks(frameCount) : [];

  return (
    <section className="timeline-panel">
      <div className="timeline-top">
        <strong>{t("timeline.title")}</strong>
        <span>{frameCount > 0 ? t("assets.frames", { count: frameCount }) : t("timeline.noFrames")}</span>
        {hasTimelineFrames ? (
          <span className="timeline-selected-inline">
            {t("timeline.selected", { count: frameCount, selected: selectedFrameIndex + 1 })}
          </span>
        ) : null}
      </div>
      <div className="timeline-ruler">
        <span className="ruler-label">{t("timeline.animations")}</span>
        {rulerValues.length ? rulerValues.map((value) => (
          <span key={value}>{value}</span>
        )) : <span className="ruler-empty">{t("timeline.rulerEmpty")}</span>}
      </div>
      {hasTimelineFrames ? (
        <div className="tracks">
        <span className="playhead" />
        <span className="playhead-label">{selectedFrameIndex + 1}</span>
        {renderedTracks.map((track, trackIndex) => (
          <div className={`track-row ${track.tone}`} key={track.name}>
            <div className="track-info">
              <div className="track-title">
                <span className={`asset-color ${track.tone}`} />
                <strong>{trackIndex === 0 && normalizeResult ? frameLabel : track.name}</strong>
              </div>
              <small>{t("assets.frames", { count: trackIndex === 0 && extractResult ? frameCount : track.frames })}</small>
            </div>
            <div className="track-main">
              <div className="track-bar">
                <span>1</span>
                <span>{track.barLabel}</span>
              </div>
              <div className="frame-strip">
                {Array.from({ length: track.visible }, (_, index) => {
                  const framePath = trackIndex === 0 ? framePaths[index] : undefined;
                  return (
                    <FrameThumb
                      alt={t("timeline.frameAlt", { index: index + 1, track: track.name })}
                      className={`frame-cell ${track.tone} ${trackIndex === 0 && index === selectedFrameIndex ? "selected" : ""}`}
                      framePath={framePath}
                      key={`${track.name}-${index}`}
                      onClick={() => onSelectFrame(index)}
                      style={{ "--hue": `${trackIndex * 18 + index * 3}deg`, "--n": `${(index % 6) - 2}px` } as CSSProperties}
                    />
                  );
                })}
              </div>
            </div>
          </div>
        ))}
        </div>
      ) : (
        <div className="timeline-empty-state">
          <strong>{state === "sourcePending" ? t("timeline.sourcePendingTitle") : t("timeline.emptyTitle")}</strong>
          <span>{state === "sourcePending" ? t("timeline.sourcePendingDetail") : t("timeline.emptyDetail")}</span>
        </div>
      )}
      <div className="timeline-transport">
        <span className="fps-control">{t("timeline.fpsFollowsExport")}</span>
      </div>
    </section>
  );
}

function FrameThumb({
  alt,
  className,
  framePath,
  onClick,
  style,
}: {
  alt: string;
  className: string;
  framePath?: string;
  onClick: () => void;
  style: CSSProperties;
}) {
  const src = usePreviewImage(framePath, null);

  return (
    <button aria-label={alt} className={className} onClick={onClick} style={style} type="button">
      {src ? <img alt="" src={src} /> : <span aria-hidden="true" className="green-box-thumb-placeholder" />}
    </button>
  );
}

function timelineMarks(frameCount: number) {
  const marks = new Set([1, frameCount]);
  const step = Math.max(1, Math.ceil(frameCount / 6));
  for (let value = step; value < frameCount; value += step) {
    marks.add(value);
  }
  return Array.from(marks).sort((left, right) => left - right);
}
