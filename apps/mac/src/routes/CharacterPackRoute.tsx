import {
  Check,
  Compass,
  FileImage,
  FolderOpen,
  Images,
  PackagePlus,
  Plus,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { choosePngSequence, choosePngSpriteSheetFile, openFileOrFolder } from "../systemDialogs";
import {
  analyzeRepairJob,
  executeRepairJob,
  loadCharacterWorkflows,
  prepareCharacterPack,
  type CharacterAnimationRequest,
  type CharacterWorkflowCatalog,
  type CharacterWorkflowPreset,
  type JobRecord,
  type PrepareCharacterPackRequest,
  type RepairAnalysis,
} from "../tauriCommands";
import type { AppLocale } from "../i18n";

type DraftAnimation = {
  id: string;
  name: string;
  fps: number;
  loop: boolean;
  sourceKind: "png_sequence" | "sprite_sheet" | null;
  paths: string[];
  sheetPath: string;
  splitMode: "fixed_grid" | "transparent_gutters";
  frameWidth: number;
  frameHeight: number;
  columns: number;
  rows: number;
  alphaThreshold: number;
  minGapPx: number;
};

const fallbackCatalog: CharacterWorkflowCatalog = {
  schemaVersion: "1",
  workflows: [
    workflow("platformer", "Platformer", "Side-view movement with a compact core animation set.", [
      animationSpec("idle", 8, true), animationSpec("walk", 12, true), animationSpec("jump", 12, false),
    ], [animationSpec("run", 14, true), animationSpec("attack", 12, false), animationSpec("hurt", 10, false), animationSpec("death", 10, false)]),
    workflow("topdown", "Top-down", "Directional movement for RPG and action-adventure characters.", [
      animationSpec("idle", 8, true), animationSpec("walk_up", 12, true), animationSpec("walk_right", 12, true), animationSpec("walk_down", 12, true),
    ], [animationSpec("attack", 12, false), animationSpec("hurt", 10, false), animationSpec("death", 10, false)]),
    workflow("isometric", "Isometric", "Diagonal-facing movement with explicit front and side coverage.", [
      animationSpec("idle", 8, true), animationSpec("walk_down", 12, true), animationSpec("walk_right", 12, true),
    ], [animationSpec("idle_back", 8, true), animationSpec("attack", 12, false), animationSpec("hurt", 10, false), animationSpec("death", 10, false)]),
    workflow("custom", "Custom", "A free-form animation contract for props and unusual characters.", [], [
      animationSpec("idle", 8, true), animationSpec("walk", 12, true), animationSpec("attack", 12, false),
    ]),
  ],
};

const copy = {
  "en-US": {
    kicker: "Character workflow",
    title: "Compile a complete character, not a pile of sheets",
    detail: "Choose a gameplay contract, attach each animation source, then let Forge normalize one shared canvas and produce a verifiable Godot handoff.",
    deliveryLabel: "Project asset loop",
    deliveryValue: "Source → Quality → Pack → Godot",
    stages: ["Choose workflow", "Attach sources", "Build and inspect", "Install with Codex"],
    workflowTitle: "Choose the gameplay contract",
    workflowDetail: "Required animations become a machine-checkable contract for Codex and the engine.",
    required: "core",
    optional: "Add an optional animation",
    customHint: "Custom keeps your current animation list.",
    packName: "Pack name",
    defaultAnimation: "Default animation",
    advanced: "Creator and license metadata",
    addAnimation: "Add custom animation",
    png: "Choose PNG sequence",
    sheet: "Choose sprite sheet",
    build: "Compile Character Pack",
    building: "Compiling shared canvas…",
    empty: "Attach every required animation source to compile.",
    frames: "frames",
    fixedGrid: "Fixed grid",
    gutters: "Transparent gutters",
    result: "Compile job",
    openJob: "Open job folder",
    ready: "sources ready",
    creator: "Creator",
    license: "License",
    animation: "Animation",
    name: "Name",
    loop: "Loop",
    frameWidth: "Frame width",
    frameHeight: "Frame height",
    columns: "Columns",
    rows: "Rows",
    alpha: "Alpha threshold",
    gap: "Minimum gap",
    qualityReview: "Quality review required — open the job to inspect each animation.",
    built: "Pack compiled. Codex can now inspect it, install it into Godot, and register it in .forge/assets.json.",
    repairKicker: "Executable repair",
    repairTitle: "Turn quality evidence into a new recipe",
    repairDetail: "Forge keeps the original job intact, applies only safe parameter changes, and writes a before/after comparison into the new job.",
    repairAttempt: "Repair attempt",
    repairChanges: "Safe changes",
    repairManual: "Still needs judgment",
    repairApply: "Run safe repair",
    repairApplying: "Running repaired recipe…",
    repairUnavailable: "No safe automatic change is available. Review the listed decisions in Forge.",
  },
  "zh-CN": {
    kicker: "角色工作流",
    title: "构建完整角色，而不是堆放几张精灵表",
    detail: "先选择游戏工作流，再为每个动画绑定来源。Forge 会统一画布与脚底锚点，并生成可验证的 Godot 交付物。",
    deliveryLabel: "项目资产闭环",
    deliveryValue: "来源 → 质检 → 资源包 → Godot",
    stages: ["选择工作流", "绑定动画来源", "构建并检查", "由 Codex 安装"],
    workflowTitle: "选择游戏工作流合同",
    workflowDetail: "必需动画会成为 Codex 和游戏引擎都能验证的明确合同。",
    required: "必需",
    optional: "添加可选动画",
    customHint: "自定义模式会保留当前动画列表。",
    packName: "角色包名称",
    defaultAnimation: "默认动画",
    advanced: "创建者与许可证信息",
    addAnimation: "添加自定义动画",
    png: "选择 PNG 序列",
    sheet: "选择精灵表",
    build: "编译角色资源包",
    building: "正在编译共享画布…",
    empty: "请为所有必需动画绑定来源后再编译。",
    frames: "帧",
    fixedGrid: "固定网格",
    gutters: "透明间隔",
    result: "编译任务",
    openJob: "打开任务目录",
    ready: "个来源已就绪",
    creator: "创建者",
    license: "许可证",
    animation: "动画",
    name: "名称",
    loop: "循环",
    frameWidth: "帧宽",
    frameHeight: "帧高",
    columns: "列数",
    rows: "行数",
    alpha: "Alpha 阈值",
    gap: "最小间隔",
    qualityReview: "需要质量复核，请打开任务检查每个动画。",
    built: "角色包已编译。Codex 现在可以检查资源包、安装到 Godot，并登记到 .forge/assets.json。",
    repairKicker: "可执行修复",
    repairTitle: "把质量证据转换为新的处理配方",
    repairDetail: "Forge 会保留原任务，只应用安全的参数调整，并在新任务中写入修复前后对比。",
    repairAttempt: "修复轮次",
    repairChanges: "安全调整",
    repairManual: "仍需人工判断",
    repairApply: "执行安全修复",
    repairApplying: "正在运行修复配方…",
    repairUnavailable: "没有可安全自动执行的调整，请在 Forge 中处理下列判断项。",
  },
} as const;

export function CharacterPackRoute({ locale, automationJob = null }: { locale: AppLocale; automationJob?: JobRecord | null }) {
  const t = copy[locale];
  const initialWorkflow = fallbackCatalog.workflows[0];
  const [catalog, setCatalog] = useState(fallbackCatalog);
  const [selectedWorkflow, setSelectedWorkflow] = useState({ id: initialWorkflow.id, version: initialWorkflow.version });
  const [packName, setPackName] = useState("Untitled Character");
  const [creator, setCreator] = useState("Game Sprite Forge");
  const [license, setLicense] = useState("private");
  const [animations, setAnimations] = useState<DraftAnimation[]>(() => initialWorkflow.requiredAnimations.map(draftFromSpec));
  const [defaultAnimation, setDefaultAnimation] = useState(initialWorkflow.defaultAnimation);
  const [isRunning, setIsRunning] = useState(false);
  const [status, setStatus] = useState<string>(t.empty);
  const [job, setJob] = useState<JobRecord | null>(null);
  const [repairAnalysis, setRepairAnalysis] = useState<RepairAnalysis | null>(null);
  const [repairError, setRepairError] = useState<string | null>(null);
  const [isRepairing, setIsRepairing] = useState(false);

  useEffect(() => {
    void loadCharacterWorkflows().then(setCatalog).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (automationJob?.operation_kind === "prepare_character_pack") {
      setJob(automationJob);
      setStatus(automationJob.lifecycle_state === "awaiting_review" ? t.qualityReview : t.built);
    }
  }, [automationJob, t.built, t.qualityReview]);

  useEffect(() => {
    if (!job || job.lifecycle_state !== "awaiting_review") {
      setRepairAnalysis(null);
      setRepairError(null);
      return;
    }
    let disposed = false;
    setRepairAnalysis(null);
    setRepairError(null);
    void analyzeRepairJob(job.job_id)
      .then((analysis) => { if (!disposed) setRepairAnalysis(analysis); })
      .catch((error) => { if (!disposed) setRepairError(String(error)); });
    return () => { disposed = true; };
  }, [job?.job_id, job?.lifecycle_state]);

  const activeWorkflow = catalog.workflows.find((item) => item.id === selectedWorkflow.id) ?? initialWorkflow;
  const requiredNames = useMemo(
    () => new Set(activeWorkflow.requiredAnimations.map((animation) => animation.name)),
    [activeWorkflow],
  );
  const readyCount = useMemo(
    () => animations.filter((animation) => animation.paths.length >= 2 || Boolean(animation.sheetPath)).length,
    [animations],
  );
  const canBuild = animations.length >= 2
    && readyCount === animations.length
    && activeWorkflow.requiredAnimations.every((required) => animations.some((animation) => animation.name === required.name))
    && new Set(animations.map((animation) => animation.name.trim())).size === animations.length
    && animations.some((animation) => animation.name === defaultAnimation)
    && animations.every((animation) => /^[A-Za-z0-9_-]+$/.test(animation.name.trim()))
    && animations.every(validAnimationSettings);

  function applyWorkflow(preset: CharacterWorkflowPreset) {
    setSelectedWorkflow({ id: preset.id, version: preset.version });
    setDefaultAnimation(preset.defaultAnimation);
    setJob(null);
    setAnimations((current) => {
      if (preset.id === "custom") return current.length >= 2 ? current : [draft("idle"), draft("walk")];
      return preset.requiredAnimations.map((spec) => current.find((animation) => animation.name === spec.name) ?? draftFromSpec(spec));
    });
    setStatus(preset.id === "custom" ? t.customHint : `${preset.label}: ${preset.requiredAnimations.map((item) => item.name).join(" · ")}`);
  }

  function updateAnimation(id: string, update: Partial<DraftAnimation>) {
    setAnimations((current) => current.map((animation) => animation.id === id ? { ...animation, ...update } : animation));
  }

  async function chooseFrames(animation: DraftAnimation) {
    const paths = await choosePngSequence(animation.paths[0]);
    if (paths.length) {
      updateAnimation(animation.id, { sourceKind: "png_sequence", paths, sheetPath: "" });
      setStatus(`${animation.name}: ${paths.length} ${t.frames}`);
    }
  }

  async function chooseSheet(animation: DraftAnimation) {
    const path = await choosePngSpriteSheetFile(animation.sheetPath);
    if (path) {
      updateAnimation(animation.id, { sourceKind: "sprite_sheet", paths: [], sheetPath: path });
      setStatus(`${animation.name}: ${fileName(path)}`);
    }
  }

  function addAnimation(name = "attack", fps = 12, loop = false) {
    const nextName = uniqueName(animations.map((animation) => animation.name), name);
    setAnimations((current) => [...current, draft(nextName, fps, loop)]);
  }

  function removeAnimation(id: string) {
    setAnimations((current) => {
      const next = current.filter((animation) => animation.id !== id);
      if (!next.some((animation) => animation.name === defaultAnimation)) setDefaultAnimation(next[0]?.name ?? "");
      return next;
    });
  }

  async function buildPack() {
    if (!canBuild) return;
    setIsRunning(true);
    setJob(null);
    setStatus(t.building);
    try {
      const request: PrepareCharacterPackRequest = {
        schemaVersion: "2",
        metadata: {
          name: packName.trim() || "Untitled Character",
          defaultAnimation,
          creator: creator.trim() || "Game Sprite Forge",
          license: license.trim() || "private",
        },
        workflow: selectedWorkflow,
        animations: animations.map(toRequest),
        quality: { requireGameReady: true },
      };
      const completed = await prepareCharacterPack(request);
      setJob(completed);
      setStatus(completed.lifecycle_state === "awaiting_review" ? t.qualityReview : t.built);
    } catch (error) {
      setStatus(String(error));
    } finally {
      setIsRunning(false);
    }
  }

  async function runRepair() {
    if (!job || !repairAnalysis?.canAutoRepair) return;
    setIsRepairing(true);
    setRepairError(null);
    setStatus(t.repairApplying);
    try {
      const completed = await executeRepairJob(job.job_id);
      setJob(completed);
      setStatus(completed.lifecycle_state === "awaiting_review" ? t.qualityReview : t.built);
    } catch (error) {
      setRepairError(String(error));
      setStatus(String(error));
    } finally {
      setIsRepairing(false);
    }
  }

  return (
    <main className="character-pack-route" role="main">
      <section className="character-pack-hero">
        <div>
          <span>{t.kicker}</span>
          <h1>{t.title}</h1>
          <p>{t.detail}</p>
        </div>
        <div className="character-delivery-mark">
          <Compass size={18} />
          <span>{t.deliveryLabel}</span>
          <strong>{t.deliveryValue}</strong>
        </div>
      </section>

      <ol className="character-stage-strip">
        {t.stages.map((stage, index) => <li key={stage}><span>{index + 1}</span>{stage}</li>)}
      </ol>

      <section className="character-workflow-panel">
        <header>
          <div><strong>{t.workflowTitle}</strong><span>{t.workflowDetail}</span></div>
          <div className="character-pack-summary"><strong>{readyCount}/{animations.length}</strong><span>{t.ready}</span></div>
        </header>
        <div className="character-workflow-grid">
          {catalog.workflows.map((preset) => (
            <button
              aria-pressed={activeWorkflow.id === preset.id}
              className={activeWorkflow.id === preset.id ? "character-workflow-card active" : "character-workflow-card"}
              key={`${preset.id}-${preset.version}`}
              onClick={() => applyWorkflow(preset)}
              type="button"
            >
              <span>{preset.id === "custom" ? (locale === "zh-CN" ? "自由" : "FREE") : String(preset.requiredAnimations.length).padStart(2, "0")}</span>
              <strong>{workflowLabel(preset.id, preset.label, locale)}</strong>
              <small>{workflowDescription(preset.id, preset.description, locale)}</small>
              <em>{preset.requiredAnimations.length ? preset.requiredAnimations.map((item) => item.name).join(" · ") : "your own contract"}</em>
              {activeWorkflow.id === preset.id ? <Check size={16} /> : null}
            </button>
          ))}
        </div>
        <div className="character-workflow-options">
          <span>{t.optional}</span>
          {activeWorkflow.optionalAnimations.map((item) => (
            <button disabled={animations.some((animation) => animation.name === item.name)} key={item.name} onClick={() => addAnimation(item.name, item.fps, item.loop)} type="button">
              <Plus size={13} /> {item.name}
            </button>
          ))}
        </div>
      </section>

      <section className="character-pack-metadata">
        <label><span>{t.packName}</span><input value={packName} onChange={(event) => setPackName(event.target.value)} /></label>
        <label>
          <span>{t.defaultAnimation}</span>
          <select value={defaultAnimation} onChange={(event) => setDefaultAnimation(event.target.value)}>
            {animations.map((animation) => <option key={animation.id} value={animation.name}>{animation.name}</option>)}
          </select>
        </label>
        <details className="character-advanced-metadata">
          <summary>{t.advanced}</summary>
          <div>
            <label><span>{t.creator}</span><input value={creator} onChange={(event) => setCreator(event.target.value)} /></label>
            <label><span>{t.license}</span><input value={license} onChange={(event) => setLicense(event.target.value)} /></label>
          </div>
        </details>
      </section>

      <section className="character-animation-grid">
        {animations.map((animation, index) => (
          <article className="character-animation-card" key={animation.id}>
            <header>
              <span>{t.animation} {index + 1}{requiredNames.has(animation.name) ? <em>{t.required}</em> : null}</span>
              <button aria-label={`Remove ${animation.name}`} disabled={animations.length <= 2 || requiredNames.has(animation.name) || isRunning} onClick={() => removeAnimation(animation.id)} type="button"><Trash2 size={15} /></button>
            </header>
            <div className="character-animation-fields">
              <label>
                <span>{t.name}</span>
                <input value={animation.name} onChange={(event) => {
                  const previous = animation.name;
                  const name = event.target.value;
                  updateAnimation(animation.id, { name });
                  if (defaultAnimation === previous) setDefaultAnimation(name);
                }} />
              </label>
              <label><span>FPS</span><input min={1} type="number" value={animation.fps} onChange={(event) => updateAnimation(animation.id, { fps: Number(event.target.value) })} /></label>
              <label className="character-loop-field"><input checked={animation.loop} type="checkbox" onChange={(event) => updateAnimation(animation.id, { loop: event.target.checked })} /><span>{t.loop}</span></label>
            </div>
            <div className="character-source-actions">
              <button className={animation.sourceKind === "png_sequence" ? "selected" : ""} onClick={() => void chooseFrames(animation)} type="button"><Images size={16} /> {t.png}</button>
              <button className={animation.sourceKind === "sprite_sheet" ? "selected" : ""} onClick={() => void chooseSheet(animation)} type="button"><FileImage size={16} /> {t.sheet}</button>
            </div>
            {animation.sourceKind === "png_sequence" ? <p className="character-source-path">{animation.paths.length} {t.frames} · {fileName(animation.paths[0] ?? "")}</p> : null}
            {animation.sourceKind === "sprite_sheet" ? (
              <div className="character-sheet-settings">
                <p className="character-source-path">{fileName(animation.sheetPath)}</p>
                <select value={animation.splitMode} onChange={(event) => updateAnimation(animation.id, { splitMode: event.target.value as DraftAnimation["splitMode"] })}>
                  <option value="fixed_grid">{t.fixedGrid}</option><option value="transparent_gutters">{t.gutters}</option>
                </select>
                {animation.splitMode === "fixed_grid" ? (
                  <div>{(["frameWidth", "frameHeight", "columns", "rows"] as const).map((field) => <label key={field}><span>{t[field]}</span><input min={1} type="number" value={animation[field]} onChange={(event) => updateAnimation(animation.id, { [field]: Number(event.target.value) })} /></label>)}</div>
                ) : (
                  <div><label><span>{t.alpha}</span><input min={0} max={255} type="number" value={animation.alphaThreshold} onChange={(event) => updateAnimation(animation.id, { alphaThreshold: Number(event.target.value) })} /></label><label><span>{t.gap}</span><input min={1} type="number" value={animation.minGapPx} onChange={(event) => updateAnimation(animation.id, { minGapPx: Number(event.target.value) })} /></label></div>
                )}
              </div>
            ) : null}
          </article>
        ))}
        {activeWorkflow.id === "custom" ? (
          <button className="character-add-animation" disabled={isRunning} onClick={() => addAnimation()} type="button"><Plus size={20} /><strong>{t.addAnimation}</strong><span>idle · walk · attack · hurt · death</span></button>
        ) : null}
      </section>

      {job?.lifecycle_state === "awaiting_review" ? (
        <section className="character-repair-panel" aria-live="polite">
          <header>
            <div><span>{t.repairKicker}</span><strong>{t.repairTitle}</strong><p>{t.repairDetail}</p></div>
            <em>{t.repairAttempt} {repairAnalysis?.attempt ?? "…"}/3</em>
          </header>
          {repairAnalysis?.changes.length ? (
            <div className="character-repair-columns">
              <div><strong>{t.repairChanges}</strong>{repairAnalysis.changes.map((change) => (
                <article key={change.id}><span>{change.scope}</span><b>{change.parameter}</b><code>{formatRepairValue(change.before)} → {formatRepairValue(change.after)}</code><small>{change.reason}</small></article>
              ))}</div>
              <div><strong>{t.repairManual}</strong>{repairAnalysis.manualActions.length ? repairAnalysis.manualActions.map((action) => <article key={action}><b>{formatRepairAction(action)}</b></article>) : <p>—</p>}</div>
            </div>
          ) : null}
          {repairAnalysis && !repairAnalysis.canAutoRepair ? <p className="character-repair-note">{t.repairUnavailable}</p> : null}
          {repairError ? <p className="character-repair-error">{repairError}</p> : null}
          <footer><button className="primary-button" disabled={!repairAnalysis?.canAutoRepair || isRepairing} onClick={() => void runRepair()} type="button"><PackagePlus size={16} /> {isRepairing ? t.repairApplying : t.repairApply}</button></footer>
        </section>
      ) : null}

      <section className="character-pack-footer">
        <div><span>{status}</span>{job ? <strong>{t.result}: {lifecycleLabel(job.lifecycle_state ?? job.state, locale)}</strong> : null}</div>
        <div>
          {job ? <button className="secondary-button" onClick={() => void openFileOrFolder(job.job_dir)} type="button"><FolderOpen size={16} /> {t.openJob}</button> : null}
          {job?.artifacts?.filter((artifact) => artifact.kind === "gsfpack" || artifact.kind === "animation_quality_report" || artifact.kind === "repair_comparison").map((artifact) => <button className="secondary-button" key={`${artifact.kind}-${artifact.path}`} onClick={() => void openFileOrFolder(artifact.path)} type="button">{artifact.kind.split("_").join(" ")}</button>)}
          <button className="primary-button" disabled={!canBuild || isRunning} onClick={() => void buildPack()} type="button"><PackagePlus size={17} /> {isRunning ? t.building : t.build}</button>
        </div>
      </section>
    </main>
  );
}

function workflow(id: string, label: string, description: string, requiredAnimations: CharacterWorkflowPreset["requiredAnimations"], optionalAnimations: CharacterWorkflowPreset["optionalAnimations"]): CharacterWorkflowPreset {
  return { id, version: "1.0.0", label, description, defaultAnimation: "idle", requiredAnimations, optionalAnimations };
}

function animationSpec(name: string, fps: number, loop: boolean) {
  return { name, fps, loop };
}

function draftFromSpec(spec: { name: string; fps: number; loop: boolean }) {
  return draft(spec.name, spec.fps, spec.loop);
}

function draft(name: string, fps = 12, loop = name !== "attack" && name !== "death" && name !== "jump"): DraftAnimation {
  return {
    id: crypto.randomUUID(), name, fps, loop, sourceKind: null, paths: [], sheetPath: "",
    splitMode: "fixed_grid", frameWidth: 64, frameHeight: 64, columns: 4, rows: 1,
    alphaThreshold: 0, minGapPx: 1,
  };
}

function toRequest(animation: DraftAnimation): CharacterAnimationRequest {
  const input: CharacterAnimationRequest["input"] = animation.sourceKind === "png_sequence"
    ? { kind: "png_sequence", paths: animation.paths }
    : { kind: "sprite_sheet", path: animation.sheetPath, split: animation.splitMode === "fixed_grid"
      ? { mode: "fixed_grid", frameWidth: animation.frameWidth, frameHeight: animation.frameHeight, columns: animation.columns, rows: animation.rows }
      : { mode: "transparent_gutters", alpha_threshold: animation.alphaThreshold, min_gap_px: animation.minGapPx } };
  return { name: animation.name.trim(), input, fps: Math.max(1, animation.fps), loop: animation.loop, matting: { mode: "preserve_alpha" } };
}

function workflowLabel(id: string, fallback: string, locale: AppLocale) {
  if (locale !== "zh-CN") return fallback;
  const labels: Record<string, string> = { platformer: "平台跳跃", topdown: "俯视角色", isometric: "等距角色", custom: "自定义" };
  return labels[id] ?? fallback;
}

function workflowDescription(id: string, fallback: string, locale: AppLocale) {
  if (locale !== "zh-CN") return fallback;
  const descriptions: Record<string, string> = {
    platformer: "面向侧视移动与紧凑的核心动画集。",
    topdown: "面向 RPG 和动作冒险的方向移动。",
    isometric: "覆盖前侧与斜向移动的等距视角合同。",
    custom: "适合道具和特殊角色的自由动画合同。",
  };
  return descriptions[id] ?? fallback;
}

function fileName(path: string) { return path.split(/[\\/]/).filter(Boolean).pop() ?? path; }
function uniqueName(names: string[], base: string) { if (!names.includes(base)) return base; let index = 2; while (names.includes(`${base}_${index}`)) index += 1; return `${base}_${index}`; }

function validAnimationSettings(animation: DraftAnimation) {
  if (!Number.isFinite(animation.fps) || animation.fps <= 0) return false;
  if (animation.sourceKind !== "sprite_sheet") return true;
  if (animation.splitMode === "transparent_gutters") return Number.isFinite(animation.alphaThreshold) && animation.alphaThreshold >= 0 && animation.alphaThreshold <= 255 && Number.isFinite(animation.minGapPx) && animation.minGapPx >= 1;
  return [animation.frameWidth, animation.frameHeight, animation.columns, animation.rows].every((value) => Number.isFinite(value) && value >= 1);
}

function formatRepairValue(value: unknown) {
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

function formatRepairAction(value: string) {
  return value.split(":").map((part) => part.split("_").join(" ")).join(" · ");
}

function lifecycleLabel(value: string, locale: AppLocale) {
  if (locale !== "zh-CN") return value.split("_").join(" ");
  const labels: Record<string, string> = { succeeded: "已完成", awaiting_review: "等待复核", failed: "失败", cancelled: "已取消", queued: "已排队", running: "处理中" };
  return labels[value] ?? value;
}
