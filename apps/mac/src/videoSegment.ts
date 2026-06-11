export type FrameSelectionMode = "target_frames" | "manual_interval";

export function estimateSelectedFrames(
  selectedDuration: number,
  fps: number,
  keepEveryNFrames: number,
  fallback: number,
) {
  if (fps > 0 && selectedDuration > 0) {
    return Math.max(1, Math.round((selectedDuration * fps) / Math.max(1, keepEveryNFrames)));
  }
  return Math.max(0, Math.round(fallback));
}

export function keepEveryForTargetFrames(
  selectedDuration: number,
  fps: number,
  targetFrameCount: number,
  fallbackFrameCount: number,
) {
  const target = Math.max(1, Math.round(targetFrameCount));
  const sourceFrames = fps > 0 && selectedDuration > 0
    ? Math.max(1, Math.round(selectedDuration * fps))
    : Math.max(1, Math.round(fallbackFrameCount));

  return Math.max(1, Math.round(sourceFrames / target));
}
