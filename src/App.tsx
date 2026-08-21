import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  calculateAppliedOffsets,
  defaultConfig,
  defaultRuntime,
  detectResolution,
  getConfig,
  getRuntimeSnapshot,
  onRuntimeState,
  saveConfig,
  setOverlayVisible,
  startSequence,
  stopSequence,
} from "./api";
import type {
  AppConfig,
  ResolutionInfo,
  RuntimeSnapshot,
  RuntimeStatus,
  TimingConfig,
} from "./types";
import { applyTheme, resolveTheme, THEME_STORAGE_KEY, type Theme } from "./theme";

const phases = ["进场准备", "飞升", "后退定位", "ADS 近战", "虚空箭", "冲刺", "终结"];
type WorkspaceTab = "display" | "aim" | "timing";
type SaveState = "idle" | "saving" | "saved" | "error";

const statusLabels: Record<RuntimeStatus, string> = {
  ready: "就绪",
  running: "运行中",
  stopping: "正在停止",
  completed: "已完成",
  aborted: "已中止",
  error: "发生错误",
};

function Icon({ name }: { name: "play" | "stop" | "display" | "target" | "clock" | "check" | "refresh" | "sun" | "moon" }) {
  const paths: Record<typeof name, ReactNode> = {
    play: <path d="m9 7 8 5-8 5V7Z" />,
    stop: <path d="M8 8h8v8H8z" />,
    display: <><rect x="3" y="5" width="18" height="12" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    target: <><circle cx="12" cy="12" r="7" /><circle cx="12" cy="12" r="2" /><path d="M12 2v3m0 14v3M2 12h3m14 0h3" /></>,
    clock: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
    check: <path d="m5 12 4 4L19 6" />,
    refresh: <><path d="M20 7v5h-5" /><path d="M18.5 16a7 7 0 1 1 .2-8.2L20 12" /></>,
    sun: <><circle cx="12" cy="12" r="3.5" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>,
    moon: <path d="M20 15.2A8.5 8.5 0 0 1 8.8 4a8.5 8.5 0 1 0 11.2 11.2Z" />,
  };
  return <svg className="icon" viewBox="0 0 24 24" aria-hidden="true">{paths[name]}</svg>;
}

interface NumberFieldProps {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  hint?: string;
}

function NumberField({ label, value, onChange, min, max, step = 1, unit, hint }: NumberFieldProps) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <span className="number-control">
        <input
          type="number"
          value={value}
          min={min}
          max={max}
          step={step}
          onChange={(event) => onChange(Number(event.target.value))}
        />
        {unit && <span className="unit">{unit}</span>}
      </span>
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}

interface VectorEditorProps {
  title: string;
  description: string;
  base: [number, number];
  applied: [number, number];
  trim?: [number, number];
  onBaseChange?: (next: [number, number]) => void;
  onTrimChange?: (next: [number, number]) => void;
  selected: boolean;
  onSelect: () => void;
}

function VectorEditor({
  title,
  description,
  base,
  applied,
  trim,
  onBaseChange,
  onTrimChange,
  selected,
  onSelect,
}: VectorEditorProps) {
  const values = trim ?? base;
  const change = onTrimChange ?? onBaseChange!;
  return (
    <section className={`vector-card${selected ? " selected" : ""}`} onFocusCapture={onSelect}>
      <button className="vector-heading" type="button" aria-pressed={selected} onClick={onSelect}>
        <span className="vector-copy">
          <strong>{title}</strong>
          <span>{description}</span>
        </span>
        <span className="applied-value">{applied[0]}, {applied[1]}</span>
      </button>
      <div className="pair-fields">
        <NumberField label={trim ? "X 微调" : "X 基准"} value={values[0]} onChange={(value) => change([value, values[1]])} />
        <NumberField label={trim ? "Y 微调" : "Y 基准"} value={values[1]} onChange={(value) => change([values[0], value])} />
      </div>
      {trim && <div className="base-note">基准 {base[0]}, {base[1]} · 当前值已换算灵敏度</div>}
    </section>
  );
}

function AimPreview({ label, value }: { label: string; value: [number, number] }) {
  const max = Math.max(Math.abs(value[0]), Math.abs(value[1]), 1);
  const x2 = 80 + (value[0] / max) * 48;
  const y2 = 50 + (value[1] / max) * 30;
  return (
    <figure className="aim-preview">
      <figcaption>
        <span>{label}</span>
        <strong>X {value[0]} · Y {value[1]}</strong>
      </figcaption>
      <svg viewBox="0 0 160 100" role="img" aria-label={`${label}方向预览`}>
        <path className="preview-grid" d="M16 50h128M80 10v80" />
        <circle className="preview-ring" cx="80" cy="50" r="24" />
        <path className="preview-vector" d={`M80 50 L${x2} ${y2}`} />
        <circle className="preview-point" cx={x2} cy={y2} r="4" />
      </svg>
    </figure>
  );
}

export default function App() {
  const [config, setConfig] = useState<AppConfig>(defaultConfig);
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot>(defaultRuntime);
  const [resolution, setResolution] = useState<ResolutionInfo | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<WorkspaceTab>("display");
  const [selectedVector, setSelectedVector] = useState<"ads" | "void" | "sprint">("void");
  const [theme, setTheme] = useState<Theme>(resolveTheme);
  const readyRef = useRef(false);
  const applied = useMemo(() => calculateAppliedOffsets(config), [config]);
  const running = ["running", "stopping"].includes(snapshot.status);

  const updateConfig = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => {
    setConfig((current) => ({ ...current, [key]: value }));
  };
  const updateTiming = <K extends keyof TimingConfig>(key: K, value: TimingConfig[K]) => {
    setConfig((current) => ({ ...current, timings: { ...current.timings, [key]: value } }));
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    Promise.all([getConfig(), getRuntimeSnapshot(), detectResolution()])
      .then(([nextConfig, nextSnapshot, nextResolution]) => {
        setConfig(nextConfig);
        setSnapshot(nextSnapshot);
        setResolution(nextResolution);
        readyRef.current = true;
      })
      .catch((reason: unknown) => {
        console.error("Failed to initialize the app", reason);
        setError("启动时没能读到设置或游戏窗口。请确认 Destiny 2 已启动，然后重新打开程序。");
      });
    onRuntimeState(setSnapshot).then((dispose) => { unlisten = dispose; });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!readyRef.current) return;
    setSaveState("saving");
    const timer = window.setTimeout(() => {
      saveConfig(config)
        .then(() => setSaveState("saved"))
        .catch((reason: unknown) => {
          console.error("Failed to save the configuration", reason);
          setSaveState("error");
          setError("设置没有保存。请检查当前用户是否能写入应用配置目录，再试一次。");
        });
    }, 450);
    return () => window.clearTimeout(timer);
  }, [config]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const syncSystemTheme = (event: MediaQueryListEvent) => {
      try {
        if (window.localStorage.getItem(THEME_STORAGE_KEY)) return;
      } catch {
        // Keep following the system when storage is unavailable.
      }
      const next = event.matches ? "dark" : "light";
      applyTheme(next);
      setTheme(next);
    };
    media.addEventListener("change", syncSystemTheme);
    return () => media.removeEventListener("change", syncSystemTheme);
  }, []);

  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    applyTheme(next, true);
    setTheme(next);
  };

  const refreshResolution = async () => {
    try {
      setResolution(await detectResolution());
      setError(null);
    } catch (reason) {
      console.error("Failed to detect the game resolution", reason);
      setError("没有读到游戏窗口。请确认 Destiny 2 已启动，并使用窗口或无边框模式后重试。");
    }
  };

  const toggleOverlay = async () => {
    const next = !config.overlayVisible;
    updateConfig("overlayVisible", next);
    try {
      await setOverlayVisible(next);
    } catch (reason) {
      console.error("Failed to toggle the overlay", reason);
      updateConfig("overlayVisible", !next);
      setError("Overlay 切换失败。请关闭后重新打开程序，再试一次。");
    }
  };

  const runAction = async (action: "start" | "stop") => {
    try {
      setError(null);
      if (action === "start") {
        setSaveState("saving");
        await saveConfig(config);
        setSaveState("saved");
        setSnapshot(await startSequence());
      } else {
        setSnapshot(await stopSequence());
      }
    } catch (reason) {
      console.error(`Failed to ${action} the sequence`, reason);
      setError(action === "start"
        ? "动作没有启动。请确认游戏和本程序使用相同的 Windows 完整性级别，再试一次。"
        : "停止请求没有完成。请松开相关键位并退出程序，再重新打开。");
    }
  };

  const currentResolution = config.resolutionMode === "manual"
    ? `${config.manualWidth} × ${config.manualHeight}`
    : resolution ? `${resolution.width} × ${resolution.height}` : "检测中";
  const selectedPreview = selectedVector === "ads"
    ? { label: "首次 ADS", value: applied.firstAds }
    : selectedVector === "void"
      ? { label: "虚空箭", value: applied.voidArrow }
      : { label: "冲刺", value: applied.sprint };

  return (
    <div className="app-shell">
      <header className="command-header">
        <div className="brand-block">
          <div className="brand-mark" aria-hidden="true">D2</div>
          <div>
            <h1>Morgath Kick</h1>
            <p>动作校准台</p>
          </div>
        </div>

        <div className="runtime-summary">
          <span className={`status-dot ${snapshot.status}`} />
          <div>
            <strong>{statusLabels[snapshot.status]}</strong>
            <span>{running ? snapshot.phaseName : snapshot.message}</span>
          </div>
        </div>
        <span className="sr-only" role="status" aria-live="polite">{statusLabels[snapshot.status]}，{snapshot.phaseName}</span>

        <div className="header-controls">
          <div className="resolution-chip">
            <Icon name="display" />
            <span><small>{config.resolutionMode === "auto" ? "自动识别" : "手动分辨率"}</small>{currentResolution}</span>
          </div>
          <button className="theme-toggle" type="button" onClick={toggleTheme} aria-label={`切换为${theme === "dark" ? "浅色" : "深色"}模式`} title={`切换为${theme === "dark" ? "浅色" : "深色"}模式`}>
            <Icon name={theme === "dark" ? "sun" : "moon"} />
            <span>{theme === "dark" ? "浅色" : "深色"}</span>
          </button>
          <label className="switch-control">
            <input type="checkbox" checked={config.overlayVisible} onChange={toggleOverlay} />
            <span className="switch-track" />
            <span>Overlay</span>
          </label>
          <button className="button secondary stop-button" type="button" onClick={() => runAction("stop")} disabled={!running}>
            <Icon name="stop" />停止 <kbd>F10</kbd>
          </button>
          <button className="button primary" type="button" onClick={() => runAction("start")} disabled={running}>
            <Icon name="play" />启动 <kbd>F8</kbd>
          </button>
        </div>
      </header>

      {error && <div className="error-banner" role="alert"><strong>操作未完成</strong><span>{error}</span><button type="button" onClick={() => setError(null)} aria-label="关闭错误提示">×</button></div>}

      <main>
        <section className="phase-panel" aria-labelledby="phase-title">
          <div className="section-heading phase-heading">
            <div>
              <h2 id="phase-title">七阶段动作序列</h2>
            </div>
            <div className="phase-meta">
              <span>当前阶段</span>
              <strong>{String(snapshot.phaseIndex + 1).padStart(2, "0")} / 07</strong>
            </div>
          </div>
          <ol className="phase-rail">
            {phases.map((phase, index) => {
              const state = index < snapshot.phaseIndex ? "complete" : index === snapshot.phaseIndex ? "active" : "upcoming";
              return (
                <li key={phase} className={state} aria-current={state === "active" ? "step" : undefined}>
                  <span className="phase-node">{state === "complete" ? <Icon name="check" /> : index + 1}</span>
                  <span className="phase-name">{phase}</span>
                </li>
              );
            })}
          </ol>
        </section>

        <nav className="workspace-tabs" aria-label="校准分组">
          {([
            ["display", "显示与灵敏度", "display"],
            ["aim", "瞄准偏移", "target"],
            ["timing", "动作时序", "clock"],
          ] as const).map(([id, label, icon]) => (
            <button key={id} className={tab === id ? "active" : ""} onClick={() => setTab(id)} aria-pressed={tab === id}>
              <Icon name={icon} />{label}
            </button>
          ))}
        </nav>

        <div className="calibration-grid">
          <section className={`calibration-column ${tab === "display" ? "mobile-active" : ""}`} aria-labelledby="display-title">
            <div className="column-heading">
              <span className="column-icon"><Icon name="display" /></span>
              <div><span className="column-index">01</span><h2 id="display-title">显示与灵敏度</h2><p>确认游戏画面尺寸，再换算鼠标移动量。</p></div>
            </div>
            <div className="segment-control" role="group" aria-label="分辨率来源">
              <button type="button" className={config.resolutionMode === "auto" ? "active" : ""} aria-pressed={config.resolutionMode === "auto"} onClick={() => updateConfig("resolutionMode", "auto")}>自动识别</button>
              <button type="button" className={config.resolutionMode === "manual" ? "active" : ""} aria-pressed={config.resolutionMode === "manual"} onClick={() => updateConfig("resolutionMode", "manual")}>手动指定</button>
            </div>
            <div className="resolution-result">
              <div><span>{config.resolutionMode === "auto" && resolution?.detectedGame ? "已识别 Destiny 2 客户区" : "当前动作分辨率"}</span><strong>{currentResolution}</strong></div>
              <button className="icon-button" onClick={refreshResolution} aria-label="重新检测分辨率" title="重新检测"><Icon name="refresh" /></button>
            </div>
            {config.resolutionMode === "manual" && (
              <div className="pair-fields">
                <NumberField label="宽度" value={config.manualWidth} min={640} max={7680} onChange={(value) => updateConfig("manualWidth", value)} unit="px" />
                <NumberField label="高度" value={config.manualHeight} min={480} max={4320} onChange={(value) => updateConfig("manualHeight", value)} unit="px" />
              </div>
            )}
            <div className="subsection-title"><span>游戏内设置</span><small>参考 15 / 1.00</small></div>
            <div className="pair-fields">
              <NumberField label="视角灵敏度" value={config.lookSensitivity} min={1} max={100} step={1} onChange={(value) => updateConfig("lookSensitivity", value)} />
              <NumberField label="ADS 修正" value={config.adsModifier} min={0.1} max={3} step={0.01} onChange={(value) => updateConfig("adsModifier", value)} />
            </div>
            <div className="scale-readout">
              <div><span>ADS 距离系数</span><strong>{applied.adsScale.toFixed(3)}×</strong></div>
              <div><span>腰射距离系数</span><strong>{applied.lookScale.toFixed(3)}×</strong></div>
            </div>
            <p className="info-note">按 15 / 1.00 的参考设置反向换算。FOV 不同或开启鼠标加速时，还要在游戏里微调。</p>
          </section>

          <section className={`calibration-column ${tab === "aim" ? "mobile-active" : ""}`} aria-labelledby="aim-title">
            <div className="column-heading">
              <span className="column-icon"><Icon name="target" /></span>
              <div><span className="column-index">02</span><h2 id="aim-title">瞄准偏移</h2><p>修正虚空箭落点与冲刺朝向。</p></div>
            </div>
            <AimPreview label={selectedPreview.label} value={selectedPreview.value} />
            <VectorEditor
              title="首次 ADS"
              description="近战前的主瞄准移动"
              base={config.firstAdsBase}
              applied={applied.firstAds}
              onBaseChange={(value) => updateConfig("firstAdsBase", value)}
              selected={selectedVector === "ads"}
              onSelect={() => setSelectedVector("ads")}
            />
            <VectorEditor
              title="虚空箭落点"
              description="在参考落点上进行像素级微调"
              base={config.voidArrowBase}
              trim={config.voidArrowTrim}
              applied={applied.voidArrow}
              onTrimChange={(value) => updateConfig("voidArrowTrim", value)}
              selected={selectedVector === "void"}
              onSelect={() => setSelectedVector("void")}
            />
            <VectorEditor
              title="冲刺朝向"
              description="终结前冲刺过程中的镜头修正"
              base={config.sprintBase}
              trim={config.sprintTrim}
              applied={applied.sprint}
              onTrimChange={(value) => updateConfig("sprintTrim", value)}
              selected={selectedVector === "sprint"}
              onSelect={() => setSelectedVector("sprint")}
            />
          </section>

          <section className={`calibration-column ${tab === "timing" ? "mobile-active" : ""}`} aria-labelledby="timing-title">
            <div className="column-heading">
              <span className="column-icon"><Icon name="clock" /></span>
              <div><span className="column-index">03</span><h2 id="timing-title">动作时序</h2><p>调整每段等待，F10 随时可停。</p></div>
            </div>
            <div className="timing-list">
              <NumberField label="飞升后等待" value={config.timings.ascensionWait} min={0} max={10} step={0.05} unit="秒" onChange={(value) => updateTiming("ascensionWait", value)} hint="X 飞升至后退定位" />
              <NumberField label="近战额外等待" value={config.timings.meleeExtraWait} min={0} max={5} step={0.05} unit="秒" onChange={(value) => updateTiming("meleeExtraWait", value)} hint="ADS 到 C 近战前追加" />
              <NumberField label="ADS 至超能" value={config.timings.adsToSuperWait} min={0} max={10} step={0.05} unit="秒" onChange={(value) => updateTiming("adsToSuperWait", value)} hint="从按下 ADS 起计算" />
              <NumberField label="超能后等待" value={config.timings.superWait} min={0} max={10} step={0.05} unit="秒" onChange={(value) => updateTiming("superWait", value)} hint="F 释放后至冲刺" />
              <NumberField label="冲刺侧移时间" value={config.timings.sprintATime} min={0} max={3} step={0.01} unit="秒" onChange={(value) => updateTiming("sprintATime", value)} hint="A 先行按下时长" />
              <NumberField label="冲刺至终结" value={config.timings.sprintToFinisher} min={0} max={5} step={0.05} unit="秒" onChange={(value) => updateTiming("sprintToFinisher", value)} hint="镜头微调后附加" />
              <NumberField label="终结后等待" value={config.timings.finisherWait} min={0} max={15} step={0.1} unit="秒" onChange={(value) => updateTiming("finisherWait", value)} hint="四次 G 后的收尾时间" />
            </div>
            <div className="safety-card">
              <div className="safety-icon"><Icon name="stop" /></div>
              <div><strong>全程可安全中止</strong><p>按 F10 后，程序会在当前短步骤结束前停止，并释放 W/A/S/D、Shift、Space、E/X/C/F/G 与鼠标右键。</p></div>
            </div>
          </section>
        </div>
      </main>

      <footer className="app-footer">
        <div className="fixed-keys">
          <span>固定键位</span>
          <kbd>WASD</kbd><kbd>Shift</kbd><kbd>Space</kbd><kbd>E</kbd><kbd>2</kbd><kbd>X</kbd><kbd>C</kbd><kbd>F</kbd><kbd>G × 4</kbd>
        </div>
        <div className={`save-indicator ${saveState}`} aria-live="polite">
          <span />
          {saveState === "saving" ? "正在保存" : saveState === "error" ? "保存失败" : "设置已保存"}
        </div>
      </footer>
    </div>
  );
}
