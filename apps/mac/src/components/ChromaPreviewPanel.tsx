import {
  Check,
  Clock3,
  Film,
} from "lucide-react";
import type { ChromaParameters, FootAnchor } from "../tauriCommands";
import type { TFunction } from "../i18n";

type ManualAnchorDraft = Pick<FootAnchor, "x" | "y">;

type ChromaPreviewPanelProps = {
  activeWorkflow: string;
  activeWorkflowLabel: string;
  disabled?: boolean;
  framePreviewState: "empty" | "processed" | "raw";
  manualAnchor: ManualAnchorDraft;
  onApplyAnchor: () => void;
  onManualAnchorChange: (anchor: ManualAnchorDraft) => void;
  parameters: ChromaParameters;
  onParametersChange: (parameters: ChromaParameters) => void;
  t: TFunction;
};

export function ChromaPreviewPanel({
  activeWorkflow,
  activeWorkflowLabel,
  disabled = false,
  framePreviewState,
  manualAnchor,
  onApplyAnchor,
  onManualAnchorChange,
  onParametersChange,
  parameters,
  t,
}: ChromaPreviewPanelProps) {
  const PreviewStateIcon = framePreviewState === "processed" ? Check : framePreviewState === "raw" ? Film : Clock3;
  const previewStateLabel =
    framePreviewState === "processed"
      ? t("stage.processedPixels")
      : framePreviewState === "raw"
        ? t("stage.rawFramePixels")
        : t("stage.waitingForFrames");

  return (
    <div className="stage-tools">
      <div className="tool-group">
        <span className="stage-mode-label">{t("stage.workspace", { workflow: activeWorkflowLabel })}</span>
        <span className="zoom-chip">{t("stage.frameInspection")}</span>
      </div>
      <div className="tool-group">
        {activeWorkflow === "Anchor" ? (
          <div className="anchor-fields compact" aria-label={t("stage.footAnchorControls")}>
            <AnchorField
              disabled={disabled}
              label={t("stage.anchorX")}
              onChange={(x) => onManualAnchorChange({ ...manualAnchor, x })}
              value={manualAnchor.x}
            />
            <AnchorField
              disabled={disabled}
              label={t("stage.anchorY")}
              onChange={(y) => onManualAnchorChange({ ...manualAnchor, y })}
              value={manualAnchor.y}
            />
            <button className="stage-action-button" disabled={disabled} onClick={onApplyAnchor} type="button">
              {t("stage.applyAnchor")}
            </button>
          </div>
        ) : activeWorkflow === "Background" ? (
          <div className="chroma-fields compact" aria-label={t("stage.chromaSettings")}>
            <ChromaField
              label={t("stage.threshold")}
              max={255}
              min={0}
              onChange={(threshold) => onParametersChange({ ...parameters, threshold })}
              value={parameters.threshold}
            />
            <ChromaField
              label={t("stage.soft")}
              max={255}
              min={0}
              onChange={(softness) => onParametersChange({ ...parameters, softness })}
              value={parameters.softness}
            />
          </div>
        ) : null}
        <span className={`stage-chip pixel-preview ${framePreviewState}`}>
          <PreviewStateIcon size={14} />
          {previewStateLabel}
        </span>
      </div>
    </div>
  );
}

function AnchorField({
  disabled,
  label,
  onChange,
  value,
}: {
  disabled?: boolean;
  label: string;
  onChange: (value: number) => void;
  value: number;
}) {
  return (
    <label className="anchor-field">
      <span>{label}</span>
      <input
        disabled={disabled}
        min={0}
        onChange={(event) => onChange(numberValue(event.target.value, value))}
        step={1}
        type="number"
        value={Math.round(value)}
      />
    </label>
  );
}

function ChromaField({
  label,
  max,
  min,
  onChange,
  value,
}: {
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  value: number;
}) {
  return (
    <label className="chroma-field">
      <span>{label}</span>
      <input
        max={max}
        min={min}
        onChange={(event) => onChange(numberValue(event.target.value, value))}
        type="number"
        value={value}
      />
    </label>
  );
}

function numberValue(value: string, fallback: number) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}
