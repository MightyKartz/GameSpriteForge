import type { CSSProperties } from "react";
import type { ExtractFramesResult, NormalizeFramesResult } from "../tauriCommands";
import { usePreviewImage } from "../hooks/usePreviewImage";
import type { TFunction } from "../i18n";

type TimelineEvidence = {
  actualFrameCount: number;
  endTimeSeconds: number;
  loopEndFrame: number | null;
  loopStartFrame: number | null;
  samplingInterval: number;
  selectedFrameIndex: number;
  startTimeSeconds: number;
  targetFrameCount: number | null;
};

type TimelineEvidenceItem = {
  key: "target" | "actual" | "range" | "sample" | "loop" | "selected";
  label: string;
  value: number | string;
};

type TimelineLoopMarker = {
  frame: number;
  label: string;
  position: number;
};

type TimelineLoopMarkers = {
  end?: TimelineLoopMarker;
  start?: TimelineLoopMarker;
};

type FrameTimelineProps = {
  evidence?: TimelineEvidence | null;
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
  evidence = null,
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
  const selectedFrameNumber = selectedFrameIndex + 1;
  const selectedSummary = hasTimelineFrames
    ? t("timeline.selected", { count: frameCount, selected: selectedFrameNumber })
    : "";
  const evidenceItems = hasTimelineFrames && evidence ? timelineEvidenceItems(evidence, t) : [];
  const loopMarkers = hasTimelineFrames && evidence ? timelineLoopMarkers(evidence, frameCount, t) : null;

  return (
    <section className="timeline-panel">
      <div className="timeline-top">
        <strong>{t("timeline.title")}</strong>
        <span>{frameCount > 0 ? t("assets.frames", { count: frameCount }) : t("timeline.noFrames")}</span>
        {hasTimelineFrames ? (
          <span className="timeline-selected-inline">
            {selectedSummary}
          </span>
        ) : null}
      </div>
      {evidenceItems.length ? (
        <div className="timeline-evidence-strip" aria-label={t("timeline.evidence.title")} role="list">
          {evidenceItems.map((item) => (
            <span className="timeline-evidence-item" data-evidence={item.key} key={item.key} role="listitem">
              <strong>{item.label}</strong>
              {item.value}
            </span>
          ))}
        </div>
      ) : null}
      <div className="timeline-ruler">
        <span className="ruler-label">{t("timeline.animations")}</span>
        {rulerValues.length ? rulerValues.map((value) => (
          <span key={value}>{value}</span>
        )) : <span className="ruler-empty">{t("timeline.rulerEmpty")}</span>}
      </div>
      {hasTimelineFrames ? (
        <div className="tracks">
        <span className="playhead" />
        <span aria-label={selectedSummary} className="playhead-label" title={selectedSummary}>{selectedFrameNumber}</span>
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
              {trackIndex === 0 && loopMarkers ? (
                <div
                  aria-label={t("timeline.evidence.loop")}
                  className="timeline-loop-markers"
                >
                  {loopMarkers.start ? (
                    <span
                      aria-label={loopMarkers.start.label}
                      className="timeline-loop-marker start"
                      data-frame={loopMarkers.start.frame}
                      data-loop-role="start"
                      style={{ "--loop-position": `${loopMarkers.start.position}%` } as CSSProperties}
                      title={loopMarkers.start.label}
                    >
                      {loopMarkers.start.label}
                    </span>
                  ) : null}
                  {loopMarkers.end ? (
                    <span
                      aria-label={loopMarkers.end.label}
                      className="timeline-loop-marker end"
                      data-frame={loopMarkers.end.frame}
                      data-loop-role="end"
                      style={{ "--loop-position": `${loopMarkers.end.position}%` } as CSSProperties}
                      title={loopMarkers.end.label}
                    >
                      {loopMarkers.end.label}
                    </span>
                  ) : null}
                </div>
              ) : null}
              <div className="track-bar">
                <span>1</span>
                <span>{track.barLabel}</span>
              </div>
              <div className="frame-strip">
                {Array.from({ length: track.visible }, (_, index) => {
                  const frameNumber = index + 1;
                  const framePath = trackIndex === 0 ? framePaths[index] : undefined;
                  const frameAlt = t("timeline.frameAlt", { index: frameNumber, track: track.name });
                  const isSelected = trackIndex === 0 && index === selectedFrameIndex;
                  const selectedFrameLabel = isSelected ? `${frameAlt} - ${selectedSummary}` : frameAlt;
                  return (
                    <FrameThumb
                      className={`frame-cell ${track.tone} ${isSelected ? "selected" : ""}`}
                      frameIndex={index}
                      frameNumber={frameNumber}
                      framePath={framePath}
                      isSelected={isSelected}
                      key={`${track.name}-${index}`}
                      onClick={() => onSelectFrame(index)}
                      selectedFrameLabel={selectedFrameLabel}
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
  className,
  frameIndex,
  frameNumber,
  framePath,
  isSelected,
  onClick,
  selectedFrameLabel,
  style,
}: {
  className: string;
  frameIndex: number;
  frameNumber: number;
  framePath?: string;
  isSelected: boolean;
  onClick: () => void;
  selectedFrameLabel: string;
  style: CSSProperties;
}) {
  const src = usePreviewImage(framePath, null);

  return (
    <button
      aria-current={isSelected ? "true" : undefined}
      aria-label={selectedFrameLabel}
      className={className}
      data-frame-index={frameIndex}
      data-frame-number={frameNumber}
      data-frame-state={isSelected ? "selected" : "available"}
      onClick={onClick}
      style={style}
      title={selectedFrameLabel}
      type="button"
    >
      {src ? <img alt="" src={src} /> : <span aria-hidden="true" className="green-box-thumb-placeholder" />}
    </button>
  );
}

function timelineEvidenceItems(evidence: TimelineEvidence, t: TFunction): TimelineEvidenceItem[] {
  return [
    {
      key: "target",
      label: t("timeline.evidence.target"),
      value: evidence.targetFrameCount ?? t("timeline.evidence.auto"),
    },
    {
      key: "actual",
      label: t("timeline.evidence.actual"),
      value: evidence.actualFrameCount,
    },
    {
      key: "range",
      label: t("timeline.evidence.range"),
      value: `${formatTimelineSeconds(evidence.startTimeSeconds)} - ${formatTimelineSeconds(evidence.endTimeSeconds)}`,
    },
    {
      key: "sample",
      label: t("timeline.evidence.sample"),
      value: t("timeline.evidence.everyN", { count: evidence.samplingInterval }),
    },
    {
      key: "loop",
      label: t("timeline.evidence.loop"),
      value: formatTimelineLoopRange(evidence.loopStartFrame, evidence.loopEndFrame),
    },
    {
      key: "selected",
      label: t("timeline.evidence.selected"),
      value: evidence.selectedFrameIndex + 1,
    },
  ];
}

function timelineLoopMarkers(evidence: TimelineEvidence, frameCount: number, t: TFunction): TimelineLoopMarkers | null {
  const startFrame = normalizeTimelineFrame(evidence.loopStartFrame, frameCount);
  const endFrame = normalizeTimelineFrame(evidence.loopEndFrame, frameCount);

  if (startFrame === null && endFrame === null) {
    return null;
  }

  return {
    start: startFrame === null ? undefined : {
      frame: startFrame,
      label: `${t("segment.loopStart")}: ${startFrame}`,
      position: timelineFramePosition(startFrame, frameCount),
    },
    end: endFrame === null ? undefined : {
      frame: endFrame,
      label: `${t("segment.loopEnd")}: ${endFrame}`,
      position: timelineFramePosition(endFrame, frameCount),
    },
  };
}

function formatTimelineLoopRange(loopStartFrame: number | null, loopEndFrame: number | null) {
  return loopStartFrame !== null && loopEndFrame !== null ? `${loopStartFrame}-${loopEndFrame}` : "--";
}

function normalizeTimelineFrame(frameNumber: number | null, frameCount: number) {
  if (frameNumber === null || frameCount <= 0 || !Number.isFinite(frameNumber)) {
    return null;
  }

  return Math.max(1, Math.min(frameCount, Math.round(frameNumber)));
}

function timelineFramePosition(frameNumber: number, frameCount: number) {
  if (frameCount <= 1) {
    return 0;
  }

  return ((frameNumber - 1) / (frameCount - 1)) * 100;
}

function formatTimelineSeconds(value: number) {
  return `${Math.max(0, value).toFixed(2)}s`;
}

function timelineMarks(frameCount: number) {
  const marks = new Set([1, frameCount]);
  const step = Math.max(1, Math.ceil(frameCount / 6));
  for (let value = step; value < frameCount; value += step) {
    marks.add(value);
  }
  return Array.from(marks).sort((left, right) => left - right);
}
