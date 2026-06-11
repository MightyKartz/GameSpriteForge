import { FolderCog, Gauge, Languages, MonitorDown } from "lucide-react";
import { useEffect, useId, useRef } from "react";
import type { HTMLAttributes, ReactNode } from "react";
import { APP_VERSION } from "../appMetadata";
import { languageModes } from "../i18n";
import type { TFunction } from "../i18n";
import type { LocalSettings } from "../tauriCommands";

type SettingsRouteProps = {
  onChooseFfmpegPath: () => void;
  onChooseFfprobePath: () => void;
  onChooseOutputFolder: () => void;
  settings: LocalSettings;
  onSettingsChange: (settings: LocalSettings) => void;
  t: TFunction;
};

export function SettingsRoute({
  onChooseFfmpegPath,
  onChooseFfprobePath,
  onChooseOutputFolder,
  settings,
  onSettingsChange,
  t,
}: SettingsRouteProps) {
  const routeRef = useRef<HTMLElement>(null);

  useEffect(() => {
    routeRef.current?.focus();
  }, []);

  function updateField<K extends keyof LocalSettings>(key: K, value: LocalSettings[K]) {
    onSettingsChange({ ...settings, [key]: value });
  }

  return (
    <main aria-label={t("settings.title")} className="settings-route" ref={routeRef} role="main" tabIndex={-1}>
      <section className="settings-panel">
        <div className="panel-title tall">
          <span>{t("settings.title")}</span>
          <span>{t("settings.subtitle")}</span>
        </div>
        <div className="settings-layout">
          <div className="settings-grid">
            <label className="settings-field settings-language-field">
              <span className="settings-label">
                <Languages size={16} />
                {t("settings.language.label")}
              </span>
              <select
                aria-label={t("settings.language.label")}
                className="settings-language-select"
                value={settings.languageMode}
                onChange={(event) => updateField("languageMode", event.target.value as LocalSettings["languageMode"])}
              >
                {languageModes.map((mode) => (
                  <option key={mode} value={mode}>
                    {mode === "auto"
                      ? t("settings.language.auto")
                      : mode === "en-US"
                        ? t("settings.language.english")
                        : t("settings.language.chinese")}
                  </option>
                ))}
              </select>
            </label>
            <SettingsField
              chooseLabel={t("settings.chooseFfmpeg")}
              icon={<MonitorDown size={16} />}
              label={t("settings.ffmpegPath")}
              onChoose={onChooseFfmpegPath}
              placeholder="/opt/homebrew/bin/ffmpeg"
              value={settings.ffmpegPath}
              onChange={(value) => updateField("ffmpegPath", value)}
            />
            <SettingsField
              chooseLabel={t("settings.chooseFfprobe")}
              icon={<MonitorDown size={16} />}
              label={t("settings.ffprobePath")}
              onChoose={onChooseFfprobePath}
              placeholder="/opt/homebrew/bin/ffprobe"
              value={settings.ffprobePath}
              onChange={(value) => updateField("ffprobePath", value)}
            />
            <SettingsField
              chooseLabel={t("settings.chooseFolder")}
              icon={<FolderCog size={16} />}
              label={t("settings.defaultOutputFolder")}
              onChoose={onChooseOutputFolder}
              value={settings.defaultOutputFolder}
              onChange={(value) => updateField("defaultOutputFolder", value)}
            />
            <SettingsField
              icon={<Gauge size={16} />}
              inputMode="numeric"
              label={t("settings.defaultFps")}
              value={String(settings.defaultFps)}
              onChange={(value) => updateField("defaultFps", numberValue(value, settings.defaultFps))}
            />
            <SettingsField
              icon={<Gauge size={16} />}
              inputMode="numeric"
              label={t("settings.defaultSheetSize")}
              value={String(settings.defaultSheetSize)}
              onChange={(value) => updateField("defaultSheetSize", numberValue(value, settings.defaultSheetSize))}
            />
          </div>
          <aside className="settings-runtime-card" aria-label={t("settings.localRuntime.title")}>
            <span>{t("settings.localRuntime.title")}</span>
            <strong>{t("settings.localRuntime.bundle")}</strong>
            <p>{t("settings.localRuntime.detail")}</p>
            <dl>
              <div>
                <dt>{t("settings.localRuntime.version")}</dt>
                <dd>v{APP_VERSION}</dd>
              </div>
              <div>
                <dt>{t("settings.localRuntime.output")}</dt>
                <dd>{settings.defaultOutputFolder}</dd>
              </div>
              <div>
                <dt>{t("settings.localRuntime.install")}</dt>
                <dd>{t("settings.localRuntime.installValue")}</dd>
              </div>
            </dl>
          </aside>
        </div>
        <div className="settings-note">
          {t("settings.note")}
        </div>
      </section>
    </main>
  );
}

function SettingsField({
  chooseLabel = "Choose Folder",
  icon,
  inputMode,
  label,
  onChange,
  onChoose,
  placeholder,
  value,
}: {
  chooseLabel?: string;
  icon: ReactNode;
  inputMode?: HTMLAttributes<HTMLInputElement>["inputMode"];
  label: string;
  onChange: (value: string) => void;
  onChoose?: () => void;
  placeholder?: string;
  value: string;
}) {
  const inputId = useId();

  return (
    <div className="settings-field">
      <label className="settings-label" htmlFor={inputId}>
        {icon}
        {label}
      </label>
      <span className={onChoose ? "settings-input-row" : undefined}>
        <input
          aria-label={label}
          id={inputId}
          inputMode={inputMode}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder}
          value={value}
        />
        {onChoose ? (
          <button aria-label={chooseLabel} className="choose-path-button" onClick={onChoose} type="button">
            {chooseLabel}
          </button>
        ) : null}
      </span>
    </div>
  );
}

function numberValue(value: string, fallback: number) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}
