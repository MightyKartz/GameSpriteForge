import { FolderOpen, PackageCheck, Scissors, Upload } from "lucide-react";
import { useState } from "react";
import type { ExportPackOutput, PackSummary } from "../tauriCommands";
import { exportTargets } from "../forgeViewModel";
import type { TFunction } from "../i18n";

export type ExportReadinessItem = {
  detail: string;
  label: string;
  state: "blocked" | "complete" | "pending" | "ready" | "warning";
};

type ExportPanelProps = {
  allowMultiSheet: boolean;
  animationName: string;
  canExport: boolean;
  creatorName: string;
  disabled?: boolean;
  exportOutput: ExportPackOutput | null;
  framesPendingQuality: boolean;
  hasSource: boolean;
  sourcePendingExtraction: boolean;
  licenseType: string;
  loopAnimation: boolean;
  onAnimationNameChange: (value: string) => void;
  onAllowMultiSheetChange: (value: boolean) => void;
  onCreatorNameChange: (value: string) => void;
  onExport: () => void;
  onLicenseTypeChange: (value: string) => void;
  onLoopAnimationChange: (value: boolean) => void;
  onChooseOutputFolder: () => void | Promise<void>;
  onOpenExportFolder: () => void;
  onSheetColumnsChange: (value: number) => void;
  onSheetMarginChange: (value: number) => void;
  onSheetPaddingChange: (value: number) => void;
  onValidate: () => void;
  packName: string;
  packSummary: PackSummary | null;
  pendingQualityFrameCount: number;
  onPackNameChange: (value: string) => void;
  outputFolder: string;
  readinessItems: ExportReadinessItem[];
  sheetColumns: number;
  sheetMarginPx: number;
  sheetPaddingPx: number;
  t: TFunction;
};

export function ExportPanel({
  allowMultiSheet,
  animationName,
  canExport,
  creatorName,
  disabled = false,
  exportOutput,
  framesPendingQuality,
  hasSource,
  sourcePendingExtraction,
  licenseType,
  loopAnimation,
  onAnimationNameChange,
  onAllowMultiSheetChange,
  onCreatorNameChange,
  onExport,
  onLicenseTypeChange,
  onLoopAnimationChange,
  onChooseOutputFolder,
  onOpenExportFolder,
  onSheetColumnsChange,
  onSheetMarginChange,
  onSheetPaddingChange,
  onValidate,
  packSummary,
  packName,
  pendingQualityFrameCount,
  onPackNameChange,
  outputFolder,
  readinessItems,
  sheetColumns,
  sheetMarginPx,
  sheetPaddingPx,
  t,
}: ExportPanelProps) {
  const primaryTargets = exportTargets.filter((target) => target.available);
  const blockers = readinessItems.filter((item) => item.state === "blocked" || item.state === "pending");
  const hasWarnings = readinessItems.some((item) => item.state === "warning");
  const readinessTitle = canExport
    ? hasWarnings
      ? t("export.readyWithWarnings")
      : t("export.ready")
    : framesPendingQuality
      ? t("export.pendingQualityTitle")
    : blockers.length
      ? t("export.missingBeforeExport", { count: blockers.length })
      : t("export.reviewSettings");
  const readinessDetail = canExport
    ? hasWarnings
      ? t("export.readyWarningsDetail")
      : t("export.readyDetail")
    : framesPendingQuality
      ? t("export.pendingQualityDetail", { count: pendingQualityFrameCount })
    : blockers[0]?.detail ?? t("export.confirmSettings");
  const [userAdvancedOpen, setUserAdvancedOpen] = useState(false);
  const advancedOpen = userAdvancedOpen;
  const outputFolderReadiness = readinessItems[3];
  const exportLocation = exportOutput?.exportDir ?? outputFolder;
  const isExportPriority = canExport || Boolean(exportOutput);
  const panelClassName = ["export-panel", isExportPriority ? "export-ready-priority" : "", exportOutput ? "export-has-output" : ""]
    .filter(Boolean)
    .join(" ");

  const targetOptions = (
    <div className="target-grid primary-targets">
      {primaryTargets.map((target) => {
        const Icon = target.icon;
        const className = ["target-option", target.selected ? "selected" : ""].filter(Boolean).join(" ");
        return (
          <div
            className={className}
            key={target.labelKey}
            role="status"
          >
            <Icon size={26} />
            <span className="target-copy">
              <span>{t(target.labelKey)}</span>
              <small>{t(target.sublabelKey)}</small>
            </span>
            {target.badge ? <strong className="target-badge">{target.badge}</strong> : null}
          </div>
        );
      })}
    </div>
  );

  const locationCard = (
    <div className="export-location-card" aria-label={t("export.location")}>
      <div className="export-location-head">
        <span>{exportOutput ? t("export.outputLocation") : t("export.defaultOutputLocation")}</span>
        {exportOutput ? <strong>{t("export.exported")}</strong> : null}
      </div>
      <code title={exportLocation || t("export.readiness.chooseOutput")}>
        {exportLocation || t("export.readiness.chooseOutput")}
      </code>
      <div className="export-location-actions">
        <button disabled={disabled} onClick={() => void onChooseOutputFolder()} type="button">
          <FolderOpen size={14} />
          {t("export.changeOutputFolder")}
        </button>
        <button disabled={disabled || !exportOutput} onClick={onOpenExportFolder} type="button">
          <FolderOpen size={14} />
          {t("export.openExportsFolder")}
        </button>
      </div>
    </div>
  );

  const keyFields = (
    <div className="export-key-fields">
      <label className="pack-name-field">
        <span>{t("export.packName")}</span>
        <input disabled={disabled} onChange={(event) => onPackNameChange(event.target.value)} value={packName} />
      </label>
      <label className="pack-name-field">
        <span>{t("export.animationName")}</span>
        <input disabled={disabled} onChange={(event) => onAnimationNameChange(event.target.value)} value={animationName} />
      </label>
    </div>
  );

  const exportActions = exportOutput ? (
    <div className="export-actions export-actions-result export-sticky-actions">
      <button className="primary-button export-main" disabled={disabled} onClick={onOpenExportFolder} type="button">
        <FolderOpen size={18} />
        {t("export.openExportsFolder")}
      </button>
      <button className="secondary-button preview-export" disabled={disabled} onClick={onValidate} type="button">
        <PackageCheck size={17} />
        {t("export.validateReimport")}
      </button>
      <button className="secondary-button preview-export" disabled={disabled || !canExport} onClick={onExport} type="button">
        <Upload size={17} />
        {t("export.reexportPack")}
      </button>
    </div>
  ) : (
    <div className="export-actions export-sticky-actions">
      <button className="primary-button export-main" disabled={disabled || !canExport} onClick={onExport} type="button">
        <Upload size={18} />
        {t("export.exportPack")}
      </button>
      <button className="secondary-button preview-export" disabled={disabled || !exportOutput} onClick={onValidate} type="button">
        <PackageCheck size={17} />
        {t("export.validateReimport")}
      </button>
    </div>
  );

  if (sourcePendingExtraction && !exportOutput) {
    return (
      <section className="export-panel export-panel-muted">
        <div className="section-label export-label">{t("export.generatedOutputs")}</div>
        <div className="export-awaiting-card export-awaiting-card-next">
          <Scissors size={24} />
          <strong>{t("export.awaitingFramesTitle")}</strong>
          <span>{t("export.awaitingFramesDetail")}</span>
          <small>{t("export.readiness.outputFolder")}: {outputFolderReadiness?.detail ?? t("export.readiness.chooseOutput")}</small>
        </div>
        <div className="target-grid primary-targets muted-targets">
          {primaryTargets.map((target) => {
            const Icon = target.icon;
            return (
              <div className="target-option" key={target.labelKey} role="status">
                <Icon size={24} />
                <span className="target-copy">
                  <span>{t(target.labelKey)}</span>
                  <small>{t(target.sublabelKey)}</small>
                </span>
              </div>
            );
          })}
        </div>
        <button className="primary-button export-main" disabled type="button">
          <Upload size={18} />
          {t("export.exportPack")}
        </button>
        <p className="export-readiness">{t("export.extractFirst")}</p>
      </section>
    );
  }

  if (!hasSource && !exportOutput) {
    return (
      <section className="export-panel export-panel-muted">
        <div className="section-label export-label">{t("export.generatedOutputs")}</div>
        <div className="export-awaiting-card">
          <Upload size={24} />
          <strong>{t("export.awaitingImportTitle")}</strong>
          <span>{t("export.awaitingImportDetail")}</span>
          <small>{t("export.readiness.outputFolder")}: {outputFolderReadiness?.detail ?? t("export.readiness.chooseOutput")}</small>
        </div>
        <div className="target-grid primary-targets muted-targets">
          {primaryTargets.map((target) => {
            const Icon = target.icon;
            return (
              <div className="target-option" key={target.labelKey} role="status">
                <Icon size={24} />
                <span className="target-copy">
                  <span>{t(target.labelKey)}</span>
                  <small>{t(target.sublabelKey)}</small>
                </span>
              </div>
            );
          })}
        </div>
        <button className="primary-button export-main" disabled type="button">
          <Upload size={18} />
          {t("export.exportPack")}
        </button>
        <p className="export-readiness">{t("export.importFirst")}</p>
      </section>
    );
  }

  return (
    <section className={panelClassName}>
      <div className="section-label export-label">{t("export.generatedOutputs")}</div>
      <div className={canExport ? `export-readiness-card ${hasWarnings ? "warning" : "ready"}` : framesPendingQuality ? "export-readiness-card next-step" : "export-readiness-card blocked"}>
        <div className="export-readiness-head">
          <strong>{readinessTitle}</strong>
          <span>{readinessDetail}</span>
        </div>
        <div className="export-readiness-list" aria-label={t("export.readiness")}>
          {readinessItems.map((item) => (
            <div className={`export-readiness-item ${item.state}`} key={item.label}>
              <span className="readiness-dot" />
              <span>{item.label}</span>
              <small>{item.detail}</small>
            </div>
          ))}
        </div>
      </div>
      {isExportPriority ? (
        <>
          {locationCard}
          {keyFields}
          {exportActions}
          {targetOptions}
        </>
      ) : (
        <>
          {targetOptions}
          {locationCard}
          {keyFields}
        </>
      )}
      <details
        className="export-advanced"
        onToggle={(event) => setUserAdvancedOpen(event.currentTarget.open)}
        open={advancedOpen}
      >
        <summary data-closed-label={t("export.metadata.show")} data-open-label={t("export.metadata.hide")}>
          {t("export.metadata")}
        </summary>
        <div aria-hidden={!advancedOpen} className="export-advanced-body" hidden={!advancedOpen}>
          <div className="export-metadata-grid">
            <label className="export-settings-field">
              <span>{t("export.creator")}</span>
              <input disabled={disabled} onChange={(event) => onCreatorNameChange(event.target.value)} value={creatorName} />
            </label>
            <label className="export-settings-field">
              <span>{t("export.license")}</span>
              <select disabled={disabled} onChange={(event) => onLicenseTypeChange(event.target.value)} value={licenseType}>
                <option value="private">{t("export.license.private")}</option>
                <option value="cc0">CC0</option>
                <option value="cc-by">CC BY</option>
                <option value="proprietary">{t("export.license.proprietary")}</option>
              </select>
            </label>
          </div>
          <div className="export-metadata-grid compact-grid">
            <NumberField
              disabled={disabled}
              label={t("export.sheetColumns")}
              min={1}
              onChange={onSheetColumnsChange}
              value={sheetColumns}
            />
            <NumberField
              disabled={disabled}
              label={t("export.sheetPadding")}
              min={0}
              onChange={onSheetPaddingChange}
              value={sheetPaddingPx}
            />
            <NumberField
              disabled={disabled}
              label={t("export.sheetMargin")}
              min={0}
              onChange={onSheetMarginChange}
              value={sheetMarginPx}
            />
          </div>
          <label className="export-toggle">
            <input
              checked={allowMultiSheet}
              disabled={disabled}
              onChange={(event) => onAllowMultiSheetChange(event.target.checked)}
              type="checkbox"
            />
            <span>{t("export.splitSheets")}</span>
          </label>
          <label className="export-toggle">
            <input
              checked={loopAnimation}
              disabled={disabled}
              onChange={(event) => onLoopAnimationChange(event.target.checked)}
              type="checkbox"
            />
            <span>{t("export.loop")}</span>
          </label>

          <div className="format-row">
            <span>{t("export.output")}</span>
            <span className="select-like static">{t("export.outputFormat")}</span>
          </div>
        </div>
      </details>

      {isExportPriority ? null : exportActions}
      {!canExport ? <p className="export-readiness">{framesPendingQuality ? t("export.processQualityFirst") : t("export.resolveMissing")}</p> : null}

      <div className="export-footer">
        <span>{exportOutput ? t("export.lastExport") : t("export.noExport")}</span>
        <button className="open-folder" disabled={disabled || !exportOutput} onClick={onOpenExportFolder} type="button">
          <FolderOpen size={15} />
          {t("export.openExportsFolder")}
        </button>
      </div>

      {exportOutput ? (
        <div className="export-output-details" aria-label={t("export.outputDetails")}>
          <p className="export-path">{t("export.outputPack", { path: exportOutput.packDir })}</p>
          <p className="export-path">{t("export.outputFrames", { count: exportOutput.framePaths.length })}</p>
          <p className="export-path">{t("export.outputSpriteSheets", { count: exportOutput.spriteSheetPaths.length })}</p>
          <p className="export-path">{t("export.outputGodotHelper", { path: exportOutput.godotHelperPath })}</p>
        </div>
      ) : null}
      {packSummary ? (
        <p className="validation-result" role="status">
          {t("export.lastValidated", { count: packSummary.frameCount, name: packSummary.name })}
        </p>
      ) : null}
    </section>
  );
}

function NumberField({
  disabled,
  label,
  min,
  onChange,
  value,
}: {
  disabled: boolean;
  label: string;
  min: number;
  onChange: (value: number) => void;
  value: number;
}) {
  return (
    <label className="export-settings-field">
      <span>{label}</span>
      <input
        disabled={disabled}
        min={min}
        onChange={(event) => onChange(numberValue(event.target.value, value, min))}
        step={1}
        type="number"
        value={value}
      />
    </label>
  );
}

function numberValue(value: string, fallback: number, min: number) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? Math.max(min, parsed) : fallback;
}
