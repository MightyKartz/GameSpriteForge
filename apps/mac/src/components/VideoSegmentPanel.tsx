import { Scissors, SkipForward, TimerReset } from "lucide-react";
import type { ExtractFramesResult, VideoProbe } from "../tauriCommands";
import type { TFunction } from "../i18n";

type VideoSegmentPanelProps = {
  disabled?: boolean;
  endTimeSeconds: number;
  extractResult: ExtractFramesResult | null;
  keepEveryNFrames: number;
  loopEndFrame: number;
  loopStartFrame: number;
  onEndTimeChange: (value: number) => void;
  onExtract: () => void;
  onKeepEveryChange: (value: number) => void;
  onLoopEndFrameChange: (value: number) => void;
  onLoopStartFrameChange: (value: number) => void;
  onStartTimeChange: (value: number) => void;
  probe: VideoProbe | null;
  startTimeSeconds: number;
  t: TFunction;
};

export function VideoSegmentPanel({
  disabled = false,
  endTimeSeconds,
  extractResult,
  keepEveryNFrames,
  loopEndFrame,
  loopStartFrame,
  onEndTimeChange,
  onExtract,
  onKeepEveryChange,
  onLoopEndFrameChange,
  onLoopStartFrameChange,
  onStartTimeChange,
  probe,
  startTimeSeconds,
  t,
}: VideoSegmentPanelProps) {
  const frameCount = extractResult?.frames.length ?? probe?.frameCountEstimate ?? 0;
  const fps = probe?.fps ?? 0;
  const duration = probe?.durationSeconds ?? Math.max(0, endTimeSeconds - startTimeSeconds);
  const sourceFrameCount = Math.max(0, Math.round(probe?.frameCountEstimate ?? frameCount));
  const selectedDuration = Math.max(0, endTimeSeconds - startTimeSeconds);
  const estimatedSelectionFrames = estimateSelectedFrames(selectedDuration, fps, keepEveryNFrames, frameCount);
  const firstSecondEnd = Math.min(Math.max(duration, 0.1), 1);
  const activeRangeStart = duration > 0 ? clampPercent((startTimeSeconds / duration) * 100) : 0;
  const activeRangeEnd = duration > 0 ? clampPercent((endTimeSeconds / duration) * 100) : 100;
  const activePreset = approxEqual(startTimeSeconds, 0) && approxEqual(endTimeSeconds, firstSecondEnd)
    ? "firstSecond"
    : duration > 0 && approxEqual(startTimeSeconds, 0) && approxEqual(endTimeSeconds, duration)
      ? "full"
      : "custom";

  function applyRangePreset(preset: "custom" | "firstSecond" | "full") {
    if (preset === "firstSecond") {
      onStartTimeChange(0);
      onEndTimeChange(firstSecondEnd);
      return;
    }
    if (preset === "full") {
      onStartTimeChange(0);
      onEndTimeChange(Math.max(0.1, duration));
    }
  }

  const panelBody = (
    <>
      <div className="panel-title segment-title">
        <span>{t("segment.title")}</span>
        <small>
          {t("segment.sourceMeta", {
            count: sourceFrameCount,
            duration: formatDuration(duration),
            fps: fps > 0 ? `${fps.toFixed(2)} FPS` : t("segment.fpsPending"),
          })}
        </small>
      </div>
      <div className="segment-decision-body">
        <div className="segment-section-label">{t("segment.range")}</div>
        <div className="segment-presets" role="group" aria-label={t("segment.rangePresets")}>
          <button
            className={activePreset === "firstSecond" ? "active" : ""}
            disabled={disabled}
            onClick={() => applyRangePreset("firstSecond")}
            type="button"
          >
            {t("segment.preset.firstSecond")}
          </button>
          <button
            className={activePreset === "full" ? "active" : ""}
            disabled={disabled}
            onClick={() => applyRangePreset("full")}
            type="button"
          >
            {t("segment.preset.fullVideo")}
          </button>
          <button
            className={activePreset === "custom" ? "active" : ""}
            disabled={disabled}
            onClick={() => applyRangePreset("custom")}
            type="button"
          >
            {t("segment.preset.custom")}
          </button>
        </div>

        <div className="segment-rail" aria-label={t("segment.selected")}>
          <span className="segment-muted" />
          <span
            className="segment-active"
            style={{
              left: `${Math.min(activeRangeStart, activeRangeEnd)}%`,
              right: `${100 - Math.max(activeRangeStart, activeRangeEnd)}%`,
            }}
          />
          <span className="segment-guidance">
            {t("segment.selectedRange", {
              end: formatTimestamp(endTimeSeconds),
              start: formatTimestamp(startTimeSeconds),
            })}
          </span>
        </div>

        <div className="segment-time-grid">
          <label className="segment-time-field">
            <span>
              <strong>{t("segment.start")}</strong>
              <small>{formatTimestamp(startTimeSeconds)}</small>
            </span>
            <input
              disabled={disabled}
              min={0}
              onChange={(event) => onStartTimeChange(numberValue(event.target.value, startTimeSeconds, 0))}
              step={0.1}
              type="number"
              value={startTimeSeconds}
            />
          </label>
          <label className="segment-time-field">
            <span>
              <strong>{t("segment.end")}</strong>
              <small>{formatTimestamp(endTimeSeconds)}</small>
            </span>
            <input
              disabled={disabled}
              min={0.1}
              onChange={(event) => onEndTimeChange(numberValue(event.target.value, endTimeSeconds, 0.1))}
              step={0.1}
              type="number"
              value={endTimeSeconds}
            />
          </label>
        </div>

        <p className="segment-estimate">{t("segment.estimatedSelection", { count: estimatedSelectionFrames })}</p>

        <label className="segment-interval-field">
          <span>{t("segment.interval")}</span>
          <span className="segment-interval-control">
            <SkipForward size={14} />
            <input
              disabled={disabled}
              min={1}
              onChange={(event) => onKeepEveryChange(numberValue(event.target.value, keepEveryNFrames, 1))}
              step={1}
              type="number"
              value={keepEveryNFrames}
            />
            <small>{t("segment.everyValue", { count: keepEveryNFrames })}</small>
          </span>
        </label>

        <details className="segment-advanced-settings">
          <summary>
            <span>
              <TimerReset size={14} />
              {t("segment.advanced")}
            </span>
            <small>{t("segment.loopRange", { end: loopEndFrame, start: loopStartFrame })}</small>
          </summary>
          <div className="segment-advanced-grid">
            <label className="mini-field loop-field">
              <span>{t("segment.loopStart")}</span>
              <input
                disabled={disabled || frameCount <= 0}
                max={Math.max(1, frameCount)}
                min={1}
                onChange={(event) => onLoopStartFrameChange(frameNumberValue(event.target.value, loopStartFrame, frameCount))}
                step={1}
                type="number"
                value={loopStartFrame}
              />
            </label>
            <label className="mini-field loop-field">
              <span>{t("segment.loopEnd")}</span>
              <input
                disabled={disabled || frameCount <= 0}
                max={Math.max(1, frameCount)}
                min={1}
                onChange={(event) => onLoopEndFrameChange(frameNumberValue(event.target.value, loopEndFrame, frameCount))}
                step={1}
                type="number"
                value={loopEndFrame}
              />
            </label>
          </div>
        </details>
      </div>
      <div className="segment-sticky-action">
        <button className="segment-extract-action" disabled={disabled} onClick={onExtract} type="button">
          <Scissors size={15} />
          {t("segment.extractEstimated", { count: estimatedSelectionFrames })}
        </button>
        <small>
          {t("segment.extractActionMeta", {
            count: keepEveryNFrames,
            end: formatTimestamp(endTimeSeconds),
            start: formatTimestamp(startTimeSeconds),
          })}
        </small>
      </div>
      <div className="segment-stats">
        <span>{t("segment.frames", { count: frameCount })}</span>
        <span>{fps > 0 ? `${fps.toFixed(2)} FPS` : t("segment.fpsPending")}</span>
        <span>{t("segment.seconds", { seconds: duration.toFixed(2) })}</span>
      </div>
      {probe ? (
        <div className="segment-metadata">
          <span>
            {probe.width} x {probe.height}
          </span>
          <span>{probe.codec || t("segment.codecUnknown")}</span>
          <span>{probe.pixelFormat || t("segment.pixelFormatUnknown")}</span>
        </div>
      ) : null}
    </>
  );

  if (extractResult) {
    return (
      <details className="video-segment-panel reextract-settings">
        <summary>
          <span>{t("segment.reextractSettings")}</span>
          <small>{t("segment.currentFrames", { count: extractResult.frames.length })}</small>
        </summary>
        {panelBody}
      </details>
    );
  }

  return <section className="video-segment-panel">{panelBody}</section>;
}

function approxEqual(left: number, right: number) {
  return Math.abs(left - right) < 0.05;
}

function clampPercent(value: number) {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(100, value));
}

function estimateSelectedFrames(selectedDuration: number, fps: number, keepEveryNFrames: number, fallback: number) {
  if (fps > 0 && selectedDuration > 0) {
    return Math.max(1, Math.round((selectedDuration * fps) / Math.max(1, keepEveryNFrames)));
  }
  return Math.max(0, Math.round(fallback));
}

function formatDuration(seconds: number) {
  if (!Number.isFinite(seconds)) {
    return "0.00";
  }
  return seconds.toFixed(2);
}

function formatTimestamp(seconds: number) {
  const safeSeconds = Math.max(0, Number.isFinite(seconds) ? seconds : 0);
  const minutes = Math.floor(safeSeconds / 60);
  const wholeSeconds = Math.floor(safeSeconds % 60);
  const hundredths = Math.round((safeSeconds - Math.floor(safeSeconds)) * 100);
  return `${minutes.toString().padStart(2, "0")}:${wholeSeconds.toString().padStart(2, "0")}.${hundredths.toString().padStart(2, "0")}`;
}

function numberValue(value: string, fallback: number, min: number) {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) && parsed >= min ? parsed : fallback;
}

function frameNumberValue(value: string, fallback: number, frameCount: number) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  return Math.max(1, Math.min(Math.max(1, frameCount), parsed));
}
