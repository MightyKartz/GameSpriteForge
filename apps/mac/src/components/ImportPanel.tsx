import { FileArchive, FileImage, Film, Images, PlayCircle } from "lucide-react";
import type { ReactNode } from "react";
import type { TFunction } from "../i18n";

type ImportPanelProps = {
  compact?: boolean;
  disabled?: boolean;
  frameSequenceInput: string;
  gsfpackPath: string;
  onFrameSequenceInputChange: (value: string) => void;
  onGsfpackPathChange: (value: string) => void;
  onChooseFrameFiles: () => void;
  onChooseGsfpackFolder: () => void;
  onChooseSpriteSheetFile: () => void;
  onChooseVideoFile: () => void;
  onImportFrames: () => void;
  onImportGsfpack: () => void;
  onImportSpriteSheet: () => void;
  onImportVideo: () => void;
  onLoadSample: () => void;
  onSpriteGridChange: (grid: SpriteGrid) => void;
  onSpriteSheetSliceModeChange: (value: SpriteSheetSliceMode) => void;
  onSourcePathChange: (value: string) => void;
  onSpriteSheetPathChange: (value: string) => void;
  onTransparentSplitAlphaThresholdChange: (value: number) => void;
  onTransparentSplitMinGapPxChange: (value: number) => void;
  sourcePath: string;
  spriteGrid: SpriteGrid;
  spriteSheetPath: string;
  spriteSheetSliceMode: SpriteSheetSliceMode;
  t: TFunction;
  transparentSplitAlphaThreshold: number;
  transparentSplitMinGapPx: number;
};

export type SpriteGrid = {
  frameWidth: number;
  frameHeight: number;
  columns: number;
  rows: number;
};

export type SpriteSheetSliceMode = "grid" | "transparent";

export function ImportPanel({
  compact = false,
  disabled = false,
  frameSequenceInput,
  gsfpackPath,
  onFrameSequenceInputChange,
  onGsfpackPathChange,
  onChooseFrameFiles,
  onChooseGsfpackFolder,
  onChooseSpriteSheetFile,
  onChooseVideoFile,
  onImportFrames,
  onImportGsfpack,
  onImportSpriteSheet,
  onImportVideo,
  onLoadSample,
  onSpriteGridChange,
  onSpriteSheetSliceModeChange,
  onSourcePathChange,
  onSpriteSheetPathChange,
  onTransparentSplitAlphaThresholdChange,
  onTransparentSplitMinGapPxChange,
  sourcePath,
  spriteGrid,
  spriteSheetPath,
  spriteSheetSliceMode,
  t,
  transparentSplitAlphaThreshold,
  transparentSplitMinGapPx,
}: ImportPanelProps) {
  return (
    <section className={compact ? "import-panel compact" : "import-panel"}>
      <div className="panel-title import-panel-title">
        <span>{t("import.sources")}</span>
        <div className="sample-action-group">
          <button className="load-sample-button" disabled={disabled} onClick={onLoadSample} type="button">
            <PlayCircle size={13} />
            {t("import.loadSamplePath")}
          </button>
          <small className="sample-action-hint">
            {t("import.sampleHint")}
          </small>
        </div>
      </div>
      <div className="import-actions">
        <PathAction
          chooseLabel={t("import.chooseFile")}
          disabled={disabled}
          icon={<Film size={17} />}
          importSelectedLabel={t("import.importSelected")}
          onAction={onImportVideo}
          onChoose={onChooseVideoFile}
          onValueChange={onSourcePathChange}
          placeholder="/path/to/source.mp4"
          selectSourceLabel={t("import.selectSource")}
          summary={t("import.video")}
          value={sourcePath}
        />
        <PathAction
          chooseLabel={t("import.chooseFile")}
          disabled={disabled}
          icon={<Images size={17} />}
          importSelectedLabel={t("import.importSelected")}
          multiline
          onAction={onImportFrames}
          onChoose={onChooseFrameFiles}
          onValueChange={onFrameSequenceInputChange}
          placeholder="/path/frame_001.png"
          selectSourceLabel={t("import.selectSource")}
          summary={t("import.pngSequence")}
          value={frameSequenceInput}
        />
        <PathAction
          chooseLabel={t("import.chooseFile")}
          disabled={disabled}
          icon={<FileImage size={17} />}
          importSelectedLabel={t("import.importSelected")}
          onAction={onImportSpriteSheet}
          onChoose={onChooseSpriteSheetFile}
          onValueChange={onSpriteSheetPathChange}
          placeholder="/path/sheet.png"
          selectSourceLabel={t("import.selectSource")}
          summary={t("import.spriteSheet")}
          value={spriteSheetPath}
        />
        <div className="sprite-split-mode" role="group" aria-label={t("import.spriteSheetSplitMode")}>
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
          <div className="sprite-grid-fields">
            <GridField label="W" onChange={(value) => onSpriteGridChange({ ...spriteGrid, frameWidth: value })} value={spriteGrid.frameWidth} />
            <GridField label="H" onChange={(value) => onSpriteGridChange({ ...spriteGrid, frameHeight: value })} value={spriteGrid.frameHeight} />
            <GridField label={t("import.grid.columns")} onChange={(value) => onSpriteGridChange({ ...spriteGrid, columns: value })} value={spriteGrid.columns} />
            <GridField label={t("import.grid.rows")} onChange={(value) => onSpriteGridChange({ ...spriteGrid, rows: value })} value={spriteGrid.rows} />
          </div>
        ) : (
          <div className="sprite-grid-fields transparent-split-fields">
            <GridField
              label={t("import.transparent.alpha")}
              max={255}
              min={0}
              onChange={onTransparentSplitAlphaThresholdChange}
              value={transparentSplitAlphaThreshold}
            />
            <GridField
              label={t("import.transparent.gap")}
              min={1}
              onChange={onTransparentSplitMinGapPxChange}
              value={transparentSplitMinGapPx}
            />
          </div>
        )}
        <PathAction
          chooseLabel={t("import.chooseFolder")}
          disabled={disabled}
          icon={<FileArchive size={17} />}
          importSelectedLabel={t("import.importSelected")}
          onAction={onImportGsfpack}
          onChoose={onChooseGsfpackFolder}
          onValueChange={onGsfpackPathChange}
          placeholder="/path/Pack.gsfpack"
          selectSourceLabel={t("import.selectSource")}
          summary={t("import.gsfpack")}
          value={gsfpackPath}
        />
      </div>
    </section>
  );
}

function GridField({
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
    <label className="sprite-grid-field">
      <span>{label}</span>
      <input
        max={max}
        min={min}
        onChange={(event) => onChange(numberValue(event.target.value, value, min, max))}
        step={1}
        type="number"
        value={value}
      />
    </label>
  );
}

function numberValue(value: string, fallback: number, min: number, max?: number) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  const upper = max ?? Number.POSITIVE_INFINITY;
  return Math.min(Math.max(parsed, min), upper);
}

function PathAction({
  chooseLabel,
  disabled,
  icon,
  importSelectedLabel,
  multiline = false,
  onAction,
  onChoose,
  onValueChange,
  placeholder,
  selectSourceLabel,
  summary,
  value,
}: {
  chooseLabel: string;
  disabled: boolean;
  icon: ReactNode;
  importSelectedLabel: string;
  multiline?: boolean;
  onAction: () => void;
  onChoose?: () => void;
  onValueChange: (value: string) => void;
  placeholder: string;
  selectSourceLabel: string;
  summary: string;
  value: string;
}) {
  const hasSelectedSource = value.trim().length > 0;
  const primaryLabel = hasSelectedSource ? importSelectedLabel : selectSourceLabel;
  const handlePrimaryAction = hasSelectedSource ? onAction : onChoose ?? onAction;

  return (
    <div className="import-action-wrap">
      <button className="import-action" disabled={disabled} onClick={handlePrimaryAction} type="button">
        {icon}
        <span>
          <strong>{primaryLabel}</strong>
          <small>{summary}</small>
        </span>
      </button>
      <div className="path-row">
        {multiline ? (
          <textarea
            className="path-input path-input-multiline"
            disabled={disabled}
            onChange={(event) => onValueChange(event.target.value)}
            placeholder={placeholder}
            rows={3}
            spellCheck={false}
            value={value}
          />
        ) : (
          <input
            className="path-input"
            disabled={disabled}
            onChange={(event) => onValueChange(event.target.value)}
            placeholder={placeholder}
            value={value}
          />
        )}
        <button className="choose-path-button" disabled={disabled || !onChoose} onClick={onChoose} type="button">
          {chooseLabel}
        </button>
      </div>
    </div>
  );
}
