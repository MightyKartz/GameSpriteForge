import { Activity, AlertTriangle, CheckCircle2, Info, PlayCircle } from "lucide-react";
import { useState } from "react";
import type { QualityReport } from "../tauriCommands";
import type { TFunction, TranslationKey } from "../i18n";

type QualityInspectorProps = {
  canProcessAndQuality: boolean;
  compact?: boolean;
  disabled?: boolean;
  onOpenAnchor: () => void;
  onOpenBackground: () => void;
  onOpenFrames: () => void;
  onOpenSheet: () => void;
  onProcess: () => void;
  onRunSample: () => void;
  report: QualityReport | null;
  framesPendingQuality: boolean;
  sourcePendingExtraction: boolean;
  t: TFunction;
};

function StatusIcon({ status }: { status: string }) {
  if (status === "warning") {
    return <AlertTriangle size={14} />;
  }

  if (status === "info") {
    return <Info size={14} />;
  }

  return <CheckCircle2 size={14} />;
}

const pendingCheckKeys: TranslationKey[] = [
  "quality.metrics.bbox",
  "quality.metrics.background",
  "quality.metrics.anchor",
  "quality.metrics.loop",
  "quality.metrics.alpha",
  "quality.metrics.consistency",
];

const recommendationKeys: Record<string, TranslationKey> = {
  adjust_anchor: "quality.recommendation.adjustAnchor",
  trim_loop_range: "quality.recommendation.trimLoopRange",
  increase_chroma_threshold: "quality.recommendation.increaseChromaThreshold",
  reduce_chroma_threshold: "quality.recommendation.reduceChromaThreshold",
  use_shorter_clip: "quality.recommendation.useShorterClip",
  increase_canvas_margin: "quality.recommendation.increaseCanvasMargin",
};

export function QualityInspector({
  canProcessAndQuality,
  compact = false,
  disabled = false,
  onOpenAnchor,
  onOpenBackground,
  onOpenFrames,
  onOpenSheet,
  onProcess,
  onRunSample,
  report,
  framesPendingQuality,
  sourcePendingExtraction,
  t,
}: QualityInspectorProps) {
  const [showInfo, setShowInfo] = useState(false);
  const [showQualityDetails, setShowQualityDetails] = useState(false);
  const alphaState = report ? alphaCoverageState(report.metrics.alphaCoverageAvg, t) : null;
  const edgeState = report ? edgeQualityState(report.metrics.cellBoundarySafe, report.metrics.bboxWidthVariationPx, t) : null;
  const metrics = report
    ? [
        { label: t("quality.metrics.bbox"), state: verdictLabel(report.verdict, t), value: `±${report.metrics.bboxCenterXDriftPx.toFixed(0)}px`, status: report.verdict === "blocked" ? "warning" : "pass" },
        { label: t("quality.metrics.background"), state: alphaState?.label ?? t("quality.metricState.pending"), value: `${Math.round(report.metrics.alphaCoverageAvg * 100)}%`, status: alphaState?.status ?? "info" },
        { label: t("quality.metrics.anchor"), state: report.metrics.bboxBottomDriftPx > 1 ? t("quality.metricState.minor") : t("quality.metricState.stable"), value: `${report.metrics.bboxBottomDriftPx.toFixed(1)}px`, status: report.metrics.bboxBottomDriftPx > 1 ? "warning" : "pass" },
        { label: t("quality.metrics.loop"), state: report.metrics.loopMatchScore >= 0.75 ? t("quality.metricState.matched") : t("quality.metricState.drift"), value: `${Math.round((1 - report.metrics.loopMatchScore) * 10)}px`, status: report.metrics.loopMatchScore >= 0.75 ? "pass" : "warning" },
        { label: t("quality.metrics.alpha"), state: edgeState?.label ?? t("quality.metricState.pending"), value: `${report.metrics.bboxWidthVariationPx.toFixed(1)}px`, status: edgeState?.status ?? "info" },
        { label: t("quality.metrics.consistency"), state: report.metrics.frameSizeConsistent ? t("quality.metricState.consistent") : t("quality.metricState.mismatch"), value: "--", status: report.metrics.frameSizeConsistent ? "pass" : "warning" },
      ]
    : [];

  const shouldUseCompactReport = compact && Boolean(report);
  const qualityDetailsVisible = !shouldUseCompactReport || showQualityDetails;
  const showQualityAction = !sourcePendingExtraction;

  return (
    <section className={shouldUseCompactReport ? "quality-inspector quality-inspector-compact" : "quality-inspector"}>
      <div className="inspector-header">
        <span>
          <Activity size={14} />
          {t("quality.title")}
        </span>
        <strong>{report ? verdictLabel(report.verdict, t) : t("quality.waiting")}</strong>
      </div>
      {report ? (
        <>
          {shouldUseCompactReport ? (
            <div className={report.verdict === "blocked" || report.recommendations.length ? "quality-compact-summary warning" : "quality-compact-summary pass"}>
              <span>
                <StatusIcon status={report.verdict === "blocked" || report.recommendations.length ? "warning" : "pass"} />
                {verdictLabel(report.verdict, t)}
              </span>
              <small>{t("quality.warnings")}&nbsp; ({report.recommendations.length})</small>
              <button className="details-link" onClick={() => setShowQualityDetails((current) => !current)} type="button">
                {showQualityDetails ? t("quality.hideQualityDetails") : t("quality.showQualityDetails")}
              </button>
            </div>
          ) : null}
          {qualityDetailsVisible ? (
            <>
              <div className="quality-list">
                {metrics.map((metric) => (
                  <div className={`quality-row ${metric.status}`} key={metric.label}>
                    <span className="quality-name">{metric.label}</span>
                    <StatusIcon status={metric.status} />
                    <strong className={metric.status === "warning" ? "status-text warning" : "status-text"}>{metric.state}</strong>
                    <span className="quality-metric">{metric.value}</span>
                  </div>
                ))}
              </div>
              <div className="details-card">
                <span className="section-label">{t("quality.details")}</span>
                <strong>{t("quality.warnings")}&nbsp; ({report.recommendations.length})</strong>
                {report.recommendations.length ? (
                  report.recommendations.map((recommendation) => (
                    <p key={recommendation}><span className="bullet" /> {recommendationLabel(recommendation, t)}</p>
                  ))
                ) : (
                  <p><span className="bullet info" /> {t("quality.noWarnings")}</p>
                )}
                <button className="details-link" onClick={() => setShowInfo((current) => !current)} type="button">
                  {showInfo ? t("quality.hideInfo") : t("quality.info")}&nbsp; ({report.notes.length})
                </button>
                {showInfo ? (
                  <div className="quality-notes">
                    {report.notes.length ? (
                      report.notes.map((note) => <p key={note}><span className="bullet info" /> {note}</p>)
                    ) : (
                      <p><span className="bullet info" /> {t("quality.noNotes")}</p>
                    )}
                  </div>
                ) : null}
              </div>
              <div className="quality-actions" aria-label={t("quality.recoveryActions")}>
                <button onClick={onOpenBackground} type="button">
                  <span>{t("quality.fixBackground")}</span>
                  <small>{t("quality.fixBackgroundDetail")}</small>
                </button>
                <button onClick={onOpenAnchor} type="button">
                  <span>{t("quality.adjustAnchor")}</span>
                  <small>{t("quality.adjustAnchorDetail")}</small>
                </button>
                <button onClick={onOpenFrames} type="button">
                  <span>{t("quality.adjustLoop")}</span>
                  <small>{t("quality.adjustLoopDetail")}</small>
                </button>
                <button onClick={onOpenSheet} type="button">
                  <span>{t("quality.reviewSheet")}</span>
                  <small>{t("quality.reviewSheetDetail")}</small>
                </button>
              </div>
            </>
          ) : null}
        </>
      ) : (
        <div className="quality-empty">
          <CheckCircle2 size={24} />
          <strong>{sourcePendingExtraction ? t("quality.waitingForFramesTitle") : t("quality.noReport")}</strong>
          <span>
            {framesPendingQuality
              ? t("quality.processRawFrames")
              : sourcePendingExtraction
              ? t("quality.waitingForFramesDetail")
              : canProcessAndQuality
                ? t("quality.processImported")
                : t("quality.noReportDetail")}
          </span>
          <div className="quality-check-preview" aria-label={t("quality.checksAfterProcessing")}>
            <span>{t("quality.checksAfterProcessing")}</span>
            <div>
              {pendingCheckKeys.map((checkKey) => (
                <small key={checkKey}>{t(checkKey)}</small>
              ))}
            </div>
          </div>
          {showQualityAction ? (
            <button
              className={canProcessAndQuality ? "primary-button compact-quality-action" : "secondary-button compact-quality-action"}
              disabled={disabled}
              onClick={canProcessAndQuality ? onProcess : onRunSample}
              type="button"
            >
              <PlayCircle size={15} />
              {canProcessAndQuality ? t("quality.processImportedAction") : t("quality.runSample")}
            </button>
          ) : (
            <p className="quality-waiting-note">{t("quality.extractFirstNote")}</p>
          )}
        </div>
      )}
    </section>
  );
}

function recommendationLabel(value: string, t: TFunction) {
  const key = recommendationKeys[value];
  if (key) {
    return t(key);
  }
  return t("quality.recommendation.generic", { name: value });
}

function labelize(value: string) {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function verdictLabel(value: QualityReport["verdict"], t: TFunction) {
  if (value === "game_ready") {
    return t("quality.verdict.gameReady");
  }
  if (value === "needs_cleanup") {
    return t("quality.verdict.needsCleanup");
  }
  if (value === "prototype_usable") {
    return t("quality.verdict.prototypeUsable");
  }
  if (value === "blocked") {
    return t("quality.verdict.blocked");
  }
  return labelize(value);
}

function alphaCoverageState(value: number, t: TFunction) {
  if (value >= 0.92) {
    return { label: t("quality.metricState.clean"), status: "pass" };
  }
  if (value >= 0.65) {
    return { label: t("quality.metricState.partial"), status: "warning" };
  }
  return { label: t("quality.metricState.sparse"), status: "warning" };
}

function edgeQualityState(cellBoundarySafe: boolean, bboxWidthVariationPx: number, t: TFunction) {
  if (!cellBoundarySafe) {
    return { label: t("quality.metricState.cellRisk"), status: "warning" };
  }
  if (bboxWidthVariationPx > 4) {
    return { label: t("quality.metricState.variable"), status: "warning" };
  }
  return { label: t("quality.metricState.cellSafe"), status: "pass" };
}
