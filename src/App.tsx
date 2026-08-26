import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";
import {
  calculateAppliedOffsets,
  checkForAppUpdate,
  closeWindow,
  defaultConfig,
  defaultRuntime,
  detectResolution,
  errorText,
  getAppVersion,
  getConfig,
  getRuntimeSnapshot,
  installAppUpdate,
  minimizeWindow,
  nativePing,
  onRuntimeState,
  saveConfig,
  setHotkeyCaptureActive,
  setOverlayVisible,
  startSequence,
  stopSequence,
  type AppUpdate,
  type AppUpdateDownloadEvent,
} from "./api";
import DiagnosticsPanel from "./DiagnosticsPanel";
import { Icon } from "./icon";
import type {
  AppConfig,
  BackendHandshakeState,
  GameKeyConfig,
  HotkeyConfig,
  ResolutionInfo,
  RuntimeSnapshot,
  RuntimeStatus,
  TimingConfig,
} from "./types";
import {
  buildHotkey,
  formatHotkey,
  formatKey,
  hotkeyModifiersFromEvent,
  isGameKeyCode,
  isHotkeyPrimaryCode,
  isModifierCode,
  isReservedMovementCode,
  mouseBindingFromButton,
} from "./keyboard";
import { applyTheme, resolveTheme, THEME_STORAGE_KEY, type Theme } from "./theme";
import morgethLogo from "../portal/morgeth-logo.png";

type WorkspaceTab = "display" | "aim" | "timing";
type SaveState = "idle" | "saving" | "saved" | "error";
type OpenPanel = "guide" | "keys" | "diagnostics" | null;
type UpdatePhase = "idle" | "checking" | "current" | "available" | "downloading" | "installing" | "error";

interface UpdateNotice {
  phase: UpdatePhase;
  manual?: boolean;
  version?: string;
  notes?: string;
  downloaded?: number;
  total?: number;
  message?: string;
}

const STARTUP_UPDATE_DELAY_MS = 4_500;
const PERIODIC_UPDATE_INTERVAL_MS = 12 * 60 * 60 * 1_000;
const SKIPPED_UPDATE_STORAGE_KEY = "d2-morgeth-kick-skipped-update";
const LEGACY_SKIPPED_UPDATE_STORAGE_KEY = "d2-morgath-kick-skipped-update";
let hotkeyCaptureReleaseTimer: number | undefined;

function suppressGlobalHotkeys(active: boolean) {
  if (hotkeyCaptureReleaseTimer) {
    window.clearTimeout(hotkeyCaptureReleaseTimer);
    hotkeyCaptureReleaseTimer = undefined;
  }
  if (active) {
    void setHotkeyCaptureActive(true);
    return;
  }
  hotkeyCaptureReleaseTimer = window.setTimeout(() => {
    void setHotkeyCaptureActive(false);
    hotkeyCaptureReleaseTimer = undefined;
  }, 200);
}

function getSkippedUpdate(): string | null {
  try {
    return window.localStorage.getItem(SKIPPED_UPDATE_STORAGE_KEY)
      ?? window.localStorage.getItem(LEGACY_SKIPPED_UPDATE_STORAGE_KEY);
  } catch {
    return null;
  }
}

function skipUpdateVersion(version: string) {
  try {
    window.localStorage.setItem(SKIPPED_UPDATE_STORAGE_KEY, version);
  } catch {
    // Skipping remains valid for this session even when storage is unavailable.
  }
}

function formatVersion(version: string) {
  return version.startsWith("v") ? version : `v${version}`;
}

const statusLabels: Record<RuntimeStatus, string> = {
  ready: "就绪",
  running: "运行中",
  stopping: "正在停止",
  completed: "已完成",
  aborted: "已中止",
  error: "发生错误",
};

interface AppDialogProps {
  title: string;
  description: string;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
  className?: string;
}

function AppDialog({ title, description, onClose, children, wide = false, className }: AppDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (dialog && !dialog.open) dialog.showModal();
  }, []);

  return (
    <dialog
      ref={dialogRef}
      className={`app-dialog${wide ? " wide" : ""}${className ? ` ${className}` : ""}`}
      aria-labelledby="dialog-title"
      aria-describedby="dialog-description"
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onClick={(event) => { if (event.target === event.currentTarget) onClose(); }}
    >
      <div className="dialog-heading">
        <div>
          <h2 id="dialog-title">{title}</h2>
          <p id="dialog-description">{description}</p>
        </div>
        <button className="dialog-close" type="button" onClick={onClose} aria-label="关闭"><Icon name="close" /></button>
      </div>
      {children}
    </dialog>
  );
}

interface UsageGuideProps {
  usesAds: boolean;
  startHotkey: string;
  onClose: () => void;
  onOpenKeys: () => void;
}

function UsageGuide({ usesAds, startHotkey, onClose, onOpenKeys }: UsageGuideProps) {
  const [page, setPage] = useState(0);
  const pages = [
    {
      eyebrow: "准备 01",
      title: "游戏内设置",
      description: "先让游戏配置与软件参数保持一致。",
      content: (
        <ul className="guide-checklist">
          <li><span>1</span><div><strong>同步灵敏度</strong><p>软件中的视角灵敏度、瞄准灵敏度和 FOV 必须与游戏内一致。</p></div></li>
          <li><span>2</span><div><strong>选择棱镜配置</strong><p>近战选择冰飞镖，星相选择飞升和分身。</p></div></li>
          <li><span>3</span><div><strong>核对近战属性</strong><p>近战属性推荐叠至 140 左右。</p></div></li>
          <li><span>4</span><div><strong>设置切换冲刺</strong><p>游戏内与软件中的切换冲刺按键需要保持一致。</p></div></li>
          <li><span>5</span><div><strong>装备运动强化模组</strong><p>腿部须装备运动强化模组。</p></div></li>
        </ul>
      ),
    },
    {
      eyebrow: "执行 02",
      title: "选择执行方式",
      description: "根据二号位武器选择对应模式。",
      content: (
        <div className="guide-mode-list">
          <section className={usesAds ? "current" : ""}>
            <span>01</span>
            <div><strong>无礼言论</strong><p>必须装备“无礼言论”，程序使用开镜瞄准完成首次转向。</p></div>
            {usesAds && <small>当前模式</small>}
          </section>
          <section className={!usesAds ? "current" : ""}>
            <span>02</span>
            <div><strong>通用配枪</strong><p>除<strong>刀剑/轻质框架</strong>（例如<strong>岁时之巅</strong>）以外的武器均可，腰射视角定位。</p></div>
            {!usesAds && <small>当前模式</small>}
          </section>
          <div className="guide-callout"><strong>启动前保持不动</strong><p>进入游戏后不要移动人物或鼠标，直接按 <kbd>{startHotkey}</kbd> 启动程序。</p></div>
        </div>
      ),
    },
    {
      eyebrow: "校准 03",
      title: "瞄点与动作时序",
      description: "默认参数不合适时，再进行小幅调整。",
      content: (
        <div className="guide-feature-list">
          <section>
            <span>XY</span>
            <div><strong>瞄点微调</strong><p>输入 X、Y 偏移量，可修正近战技能、虚空箭和终结时的瞄准点。</p><small>X 向左为负、向右为正；Y 向上为负、向下为正。</small></div>
          </section>
          <section>
            <span>秒</span>
            <div><strong>动作时序</strong><p>当近战、超能或终结时机过早、过晚时，调整对应动作之间的等待时间。</p><small>每次只改一个参数，并用小幅度变化验证结果。</small></div>
          </section>
        </div>
      ),
    },
    {
      eyebrow: "排查 04",
      title: "常见问题",
      description: "按现象快速核对设置与执行时机。",
      content: (
        <dl className="guide-faq">
          <div><dt>人物没跑到位置就停</dt><dd>确认游戏内已设置“切换冲刺”，并与软件按键映射一致。</dd></div>
          <div><dt>移动过度/不足，撞墙等问题</dt><dd>检查是否装备了轻质框架武器和强化移动金装。</dd></div>
          <div><dt>为什么总是往左边飞？</dt><dd>小怪没有拉到位，需调整虚空箭落点与超能后等待时间。</dd></div>
          <div><dt>为什么启动后没反应？</dt><dd>点击“诊断”查看是否全部正常，常见原因是键盘驱动冲突。</dd></div>
          <div><dt>虚空箭或冰飞镖偏得很远</dt><dd>确认软件与游戏的灵敏度一致，并检查执行方式是否与二号位武器匹配。</dd></div>
          <div><dt>无法终结或 BOSS 跺脚</dt><dd>微调近战、超能间隔和虚空箭落点，使飞镖与冲刺终结落在正确时机。</dd></div>
        </dl>
      ),
    },
    {
      eyebrow: "确认 05",
      title: "安全与使用边界",
      description: "开始前确认项目来源与适用条款。",
      content: (
        <div className="guide-final">
          <div className="guide-final-mark"><Icon name="help" /></div>
          <strong>代码公开，风险自查</strong>
          <p>本项目代码完全开源，可通过源码与发布文件自行核对。使用本工具前，请确认适用的游戏条款，并自行承担使用风险。</p>
          <small>运行期间可随时按停止热键中止，程序会释放已按下的操作键。</small>
        </div>
      ),
    },
  ];
  const activePage = pages[page];
  const lastPage = page === pages.length - 1;

  useEffect(() => {
    const handleGuideKey = (event: KeyboardEvent) => {
      if (event.key === "ArrowLeft") setPage((current) => Math.max(0, current - 1));
      if (event.key === "ArrowRight") setPage((current) => Math.min(pages.length - 1, current + 1));
    };
    window.addEventListener("keydown", handleGuideKey);
    return () => window.removeEventListener("keydown", handleGuideKey);
  }, [pages.length]);

  return (
    <AppDialog title="使用说明" description="使用左右方向键或下方按钮翻页。" onClose={onClose} wide>
      <div className="guide-progress" aria-label={`使用说明，第 ${page + 1} 页，共 ${pages.length} 页`}>
        <span>{page + 1} / {pages.length}</span>
        <div>
          {pages.map((item, index) => (
            <button key={item.title} type="button" className={index === page ? "active" : ""} onClick={() => setPage(index)} aria-label={`第 ${index + 1} 页：${item.title}`} aria-current={index === page ? "step" : undefined} />
          ))}
        </div>
      </div>
      <section className="guide-page" aria-live="polite" aria-labelledby="guide-page-title">
        <header><span>{activePage.eyebrow}</span><h3 id="guide-page-title">{activePage.title}</h3><p>{activePage.description}</p></header>
        {activePage.content}
      </section>
      <div className="guide-actions">
        <button className="text-button" type="button" onClick={onOpenKeys}><Icon name="keyboard" />检查按键设置</button>
        <span>也可使用键盘 ← → 翻页</span>
        <div>
          <button className="button secondary" type="button" onClick={() => setPage((current) => Math.max(0, current - 1))} disabled={page === 0}>上一页</button>
          {lastPage
            ? <button className="button primary" type="button" onClick={onClose}>我已了解并关闭</button>
            : <button className="button primary" type="button" onClick={() => setPage((current) => Math.min(pages.length - 1, current + 1))}>下一页</button>}
        </div>
      </div>
    </AppDialog>
  );
}

interface KeyCaptureFieldProps {
  label: string;
  value: string;
  disabled: boolean;
  mode: "hotkey" | "game";
  onChange: (value: string) => void;
}

function KeyCaptureField({ label, value, disabled, mode, onChange }: KeyCaptureFieldProps) {
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState("");
  const [issue, setIssue] = useState("");

  useEffect(() => {
    if (!recording) return;
    suppressGlobalHotkeys(true);
    return () => suppressGlobalHotkeys(false);
  }, [recording]);

  const beginRecording = () => {
    if (disabled) return;
    setRecording(true);
    setPreview("");
    setIssue("");
  };

  const commit = (binding: string) => {
    onChange(binding);
    setRecording(false);
    setPreview("");
    setIssue("");
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (!recording || disabled || event.repeat) return;
    event.preventDefault();
    event.stopPropagation();

    if (mode === "hotkey") {
      const modifiers = hotkeyModifiersFromEvent(event.nativeEvent);
      if (isModifierCode(event.code)) {
        setPreview(buildHotkey("等待主键…", modifiers));
        return;
      }
      if (!isHotkeyPrimaryCode(event.code)) {
        setIssue("暂不支持 " + (event.code || "该按键") + "，请按其他键");
        return;
      }
      commit(buildHotkey(event.code, modifiers));
      return;
    }

    if (isReservedMovementCode(event.code)) {
      setIssue("W / A / S / D 已固定用于移动，不能重复绑定");
      return;
    }
    if (!isGameKeyCode(event.code)) {
      setIssue("暂不支持 " + (event.code || "该按键") + "，请按其他键");
      return;
    }
    commit(event.code);
  };

  const handleMouseDown = (event: ReactMouseEvent<HTMLButtonElement>) => {
    if (!recording || disabled) return;
    const mouseBinding = mouseBindingFromButton(event.button);
    if (!mouseBinding) return;
    event.preventDefault();
    event.stopPropagation();
    commit(mode === "hotkey"
      ? buildHotkey(mouseBinding, hotkeyModifiersFromEvent(event.nativeEvent))
      : mouseBinding);
  };

  const displayValue = recording
    ? preview ? formatHotkey(preview) : "请按下目标按键…"
    : mode === "hotkey" ? formatHotkey(value) : formatKey(value);

  return (
    <div className={"key-capture-field" + (recording ? " recording" : "") + (issue ? " invalid" : "")}>
      <span className="key-capture-label">{label}</span>
      <button
        className="key-capture-button"
        type="button"
        disabled={disabled}
        aria-label={label + "，当前为 " + (mode === "hotkey" ? formatHotkey(value) : formatKey(value)) + "。点击后按下新按键"}
        aria-pressed={recording}
        onClick={beginRecording}
        onKeyDown={handleKeyDown}
        onMouseDown={handleMouseDown}
        onContextMenu={(event) => { if (recording) event.preventDefault(); }}
        onBlur={() => setRecording(false)}
      >
        <kbd>{displayValue}</kbd>
        <small>{recording ? "正在识别键盘、鼠标中键或侧键" : "点击后直接按键"}</small>
      </button>
      {issue && <span className="key-capture-error" role="alert">{issue}</span>}
    </div>
  );
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
      {trim && <div className="base-note">基准 {base[0]}, {base[1]} · 当前值已换算视角灵敏度</div>}
    </section>
  );
}

function AimPreview({ label, reference, value }: { label: string; reference: [number, number]; value: [number, number] }) {
  const delta: [number, number] = [value[0] - reference[0], value[1] - reference[1]];
  const clamp = (input: number, min: number, max: number) => Math.max(min, Math.min(max, input));
  const x2 = clamp(80 + delta[0] / 20, 18, 142);
  const y2 = clamp(52 + delta[1] / 20, 14, 88);
  const signed = (input: number) => input > 0 ? `+${input}` : `${input}`;
  return (
    <figure className="aim-preview">
      <figcaption>
        <span>{label}微调预览</span>
        <strong>ΔX {signed(delta[0])} · ΔY {signed(delta[1])}</strong>
      </figcaption>
      <svg viewBox="0 0 160 104" role="img" aria-label={`${label}默认落点到调整后落点，X 偏移 ${delta[0]}，Y 偏移 ${delta[1]}`}>
        <path className="preview-grid" d="M16 52h128M80 12v80" />
        <circle className="preview-ring" cx="80" cy="52" r="28" />
        <path className="preview-vector" d={`M80 52 L${x2} ${y2}`} />
        <circle className="preview-reference" cx="80" cy="52" r="6" />
        <circle className="preview-point" cx={x2} cy={y2} r="4" />
      </svg>
      <div className="preview-legend" title="预览比例：每 20 个鼠标计数显示 1 px">
        <span><i className="reference" />默认落点</span>
        <span><i className="current" />调整后</span>
        <small>当前 {value[0]}, {value[1]} · 20:1</small>
      </div>
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
  const [selectedVector, setSelectedVector] = useState<"first" | "void" | "sprint">("void");
  const [theme, setTheme] = useState<Theme>(resolveTheme);
  const [openPanel, setOpenPanel] = useState<OpenPanel>(null);
  const [appVersion, setAppVersion] = useState("0.3.5");
  const [updateNotice, setUpdateNotice] = useState<UpdateNotice>({ phase: "idle" });
  const [backend, setBackend] = useState<BackendHandshakeState>({ status: "checking" });
  const [lastRuntimeError, setLastRuntimeError] = useState<string | null>(null);
  const readyRef = useRef(false);
  const pendingUpdateRef = useRef<AppUpdate | null>(null);
  const checkingUpdateRef = useRef(false);
  const applied = useMemo(() => calculateAppliedOffsets(config), [config]);
  const running = ["running", "stopping"].includes(snapshot.status);

  const disposePendingUpdate = useCallback(async () => {
    const pending = pendingUpdateRef.current;
    pendingUpdateRef.current = null;
    if (pending) await pending.close().catch(() => undefined);
  }, []);

  const checkForUpdates = useCallback(async (manual = false) => {
    if (checkingUpdateRef.current) return;
    checkingUpdateRef.current = true;
    setUpdateNotice({ phase: "checking", manual });
    try {
      const update = await checkForAppUpdate();
      await disposePendingUpdate();
      if (!update) {
        setUpdateNotice(manual
          ? { phase: "current", manual: true, message: `当前已是最新版本 v${appVersion}` }
          : { phase: "idle" });
        return;
      }
      if (!manual && getSkippedUpdate() === update.version) {
        await update.close().catch(() => undefined);
        setUpdateNotice({ phase: "idle" });
        return;
      }
      pendingUpdateRef.current = update;
      setUpdateNotice({
        phase: "available",
        version: update.version,
        notes: update.body,
      });
    } catch (reason) {
      console.info("Update check did not complete", reason);
      setUpdateNotice(manual
        ? { phase: "error", manual: true, message: "暂时无法连接 GitHub。动作功能不受影响，可稍后重试。" }
        : { phase: "idle" });
    } finally {
      checkingUpdateRef.current = false;
    }
  }, [appVersion, disposePendingUpdate]);

  const updateConfig = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => {
    setConfig((current) => ({ ...current, [key]: value }));
  };
  const updateTiming = <K extends keyof TimingConfig>(key: K, value: TimingConfig[K]) => {
    setConfig((current) => ({ ...current, timings: { ...current.timings, [key]: value } }));
  };
  const updateHotkey = <K extends keyof HotkeyConfig>(key: K, value: HotkeyConfig[K]) => {
    setConfig((current) => ({ ...current, hotkeys: { ...current.hotkeys, [key]: value } }));
  };
  const updateGameKey = <K extends keyof GameKeyConfig>(key: K, value: GameKeyConfig[K]) => {
    setConfig((current) => ({ ...current, gameKeys: { ...current.gameKeys, [key]: value } }));
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getAppVersion().then(setAppVersion).catch(() => undefined);
    // 原生后端握手：正式构建中失败（例如脱离 Tauri 运行）会禁用动作执行，
    // 绝不静默进入前端模拟模式。
    const pingStarted = performance.now();
    nativePing()
      .then((ping) => {
        setBackend({
          status: "ok",
          ping,
          latencyMs: Math.round(performance.now() - pingStarted),
        });
      })
      .catch((reason: unknown) => {
        const message = errorText(reason);
        console.error("Native backend handshake failed", reason);
        setBackend({ status: "unavailable", message });
        setError(message);
      });
    Promise.all([getConfig(), getRuntimeSnapshot(), detectResolution()])
      .then(([nextConfig, nextSnapshot, nextResolution]) => {
        setConfig(nextConfig);
        setSnapshot(nextSnapshot);
        setResolution(nextResolution);
        // 启动时就已处于 error 的 runtime-state（例如热键注册失败）同样完整保留，
        // 不能只依赖后续 runtime-state 事件（订阅前的事件会丢失）。
        if (nextSnapshot.status === "error" && nextSnapshot.message) {
          setLastRuntimeError(nextSnapshot.message);
          setError(nextSnapshot.message);
        }
        readyRef.current = true;
        if (!nextConfig.usageGuideSeen) setOpenPanel("guide");
      })
      .catch((reason: unknown) => {
        console.error("Failed to initialize the app", reason);
        setError(errorText(reason) || "启动时没能读到设置或游戏窗口。请确认 Destiny 2 已启动，然后重新打开程序。");
      });
    onRuntimeState((next) => {
      setSnapshot(next);
      // runtime-state 返回 error 时自动保留完整原始错误：
      // 顶部状态栏可以省略显示，但完整错误始终保留在横幅与诊断面板中。
      if (next.status === "error" && next.message) {
        setLastRuntimeError(next.message);
        setError(next.message);
      }
    }).then((dispose) => { unlisten = dispose; });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    const startupTimer = window.setTimeout(() => void checkForUpdates(false), STARTUP_UPDATE_DELAY_MS);
    const periodicTimer = window.setInterval(() => void checkForUpdates(false), PERIODIC_UPDATE_INTERVAL_MS);
    return () => {
      window.clearTimeout(startupTimer);
      window.clearInterval(periodicTimer);
    };
  }, [checkForUpdates]);

  useEffect(() => () => { void disposePendingUpdate(); }, [disposePendingUpdate]);

  useEffect(() => {
    if (!["current", "error"].includes(updateNotice.phase)) return;
    const timer = window.setTimeout(() => setUpdateNotice({ phase: "idle" }), 7_000);
    return () => window.clearTimeout(timer);
  }, [updateNotice.phase]);

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

  const deferUpdate = () => {
    void disposePendingUpdate();
    setUpdateNotice({ phase: "idle" });
  };

  const skipAvailableUpdate = () => {
    if (updateNotice.version) skipUpdateVersion(updateNotice.version);
    void disposePendingUpdate();
    setUpdateNotice({ phase: "idle" });
  };

  const installAvailableUpdate = async () => {
    const update = pendingUpdateRef.current;
    if (!update || running) return;
    let downloaded = 0;
    let total: number | undefined;
    setUpdateNotice((current) => ({ ...current, phase: "downloading", downloaded: 0 }));
    const onEvent = (event: AppUpdateDownloadEvent) => {
      if (event.event === "Started") {
        total = event.data.contentLength;
        setUpdateNotice((current) => ({ ...current, phase: "downloading", downloaded: 0, total }));
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        setUpdateNotice((current) => ({ ...current, phase: "downloading", downloaded, total }));
      } else {
        setUpdateNotice((current) => ({ ...current, phase: "installing", downloaded, total }));
      }
    };
    try {
      await installAppUpdate(update, onEvent);
    } catch (reason) {
      console.info("Update installation did not complete", reason);
      await disposePendingUpdate();
      setUpdateNotice({
        phase: "error",
        manual: true,
        message: "更新下载或安装未完成。当前设置与动作功能不受影响，请稍后重试。",
      });
    }
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
      setError("悬浮窗切换失败。请关闭后重新打开程序，再试一次。");
    }
  };

  const runAction = async (action: "start" | "stop") => {
    if (backend.status !== "ok") {
      setError(backend.message ?? "原生后端不可用：握手未完成，动作执行已禁用。");
      return;
    }
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
      // 保留后端返回的完整原始错误，不再替换为通用提示。
      const message = errorText(reason);
      if (action === "start") setLastRuntimeError(message);
      setError(message);
    }
  };

  const currentResolution = config.resolutionMode === "manual"
    ? `${config.manualWidth} × ${config.manualHeight}`
    : resolution ? `${resolution.width} × ${resolution.height}` : "检测中";
  const usesAds = config.firstAimMode === "ads";
  const firstAimLabel = usesAds ? "ADS 转向" : "腰射转向";
  const defaultApplied = calculateAppliedOffsets({
    ...config,
    firstAdsBase: defaultConfig.firstAdsBase,
    firstHipBase: defaultConfig.firstHipBase,
    voidArrowBase: defaultConfig.voidArrowBase,
    voidArrowTrim: defaultConfig.voidArrowTrim,
    sprintBase: defaultConfig.sprintBase,
    sprintTrim: defaultConfig.sprintTrim,
  });
  const selectedPreview = selectedVector === "first"
    ? { label: firstAimLabel, reference: usesAds ? defaultApplied.firstAds : defaultApplied.firstHip, value: usesAds ? applied.firstAds : applied.firstHip }
    : selectedVector === "void"
      ? { label: "虚空箭", reference: defaultApplied.voidArrow, value: applied.voidArrow }
      : { label: "冲刺", reference: defaultApplied.sprint, value: applied.sprint };
  const runtimeMessage = snapshot.status === "ready"
    ? `参数已就绪，按 ${formatHotkey(config.hotkeys.start)} 启动`
    : running ? snapshot.phaseName : snapshot.message;
  const closeGuide = () => {
    if (!config.usageGuideSeen) updateConfig("usageGuideSeen", true);
    setOpenPanel(null);
  };
  const openKeysFromGuide = () => {
    if (!config.usageGuideSeen) updateConfig("usageGuideSeen", true);
    setOpenPanel("keys");
  };
  const resetParameters = () => {
    setConfig((current) => ({
      ...structuredClone(defaultConfig),
      hotkeys: current.hotkeys,
      gameKeys: current.gameKeys,
      overlayVisible: current.overlayVisible,
      overlayOpacity: current.overlayOpacity,
      usageGuideSeen: current.usageGuideSeen,
    }));
    setSelectedVector("first");
  };
  const resetKeyBindings = () => {
    updateConfig("hotkeys", structuredClone(defaultConfig.hotkeys));
    updateConfig("gameKeys", structuredClone(defaultConfig.gameKeys));
  };
  const gameKeyRows: Array<[keyof GameKeyConfig, string, string]> = [
    ["sprint", "切换冲刺", "Shift"],
    ["jump", "跳跃", "Space"],
    ["interact", "插旗 / 交互", "E"],
    ["weaponSlot2", "切换到 2 号位武器", "2"],
    ["melee", "近战", "C"],
    ["ascension", "飞升", "X"],
    ["superAbility", "超能", "F"],
    ["finisher", "终结技", "G"],
  ];
  const updateBusy = ["downloading", "installing"].includes(updateNotice.phase);
  const showUpdateNotice = ["available", "downloading", "installing"].includes(updateNotice.phase)
    || Boolean(updateNotice.manual && ["checking", "current", "error"].includes(updateNotice.phase));
  const updateProgress = updateNotice.total
    ? Math.min(100, Math.round(((updateNotice.downloaded ?? 0) / updateNotice.total) * 100))
    : undefined;
  const updateNotes = updateNotice.notes?.replace(/\s+/g, " ").trim();
  const updateTitle = updateNotice.phase === "checking"
    ? "正在检查更新"
    : updateNotice.phase === "current"
      ? "当前已是最新版本"
      : updateNotice.phase === "available"
        ? `发现新版本 ${formatVersion(updateNotice.version ?? "")}`
        : updateNotice.phase === "downloading"
          ? `正在下载 ${formatVersion(updateNotice.version ?? "")}`
          : updateNotice.phase === "installing"
            ? "正在校验并安装更新"
            : "更新检查未完成";
  const updateMessage = updateNotice.message
    ?? (updateNotice.phase === "checking"
      ? "正在读取 GitHub Release 的 latest.json。"
      : updateNotice.phase === "available"
        ? updateNotes || "新版本已准备好，可以立即下载并安装。"
        : updateNotice.phase === "downloading"
          ? updateProgress === undefined ? "正在下载完整安装包…" : `安装包已下载 ${updateProgress}%`
          : updateNotice.phase === "installing"
            ? "输入已释放，安装程序接管后应用会自动退出并重新启动。"
            : "当前版本不需要更新。"
    );

  return (
    <div className="app-shell">
      <header className="command-header" data-tauri-drag-region>
        <div className="brand-block" data-tauri-drag-region>
          <img className="brand-mark" src={morgethLogo} alt="" aria-hidden="true" />
          <div>
            <h1>Morgeth Kick</h1>
          </div>
        </div>

        <div className="runtime-summary" data-tauri-drag-region>
          <span className={`status-dot ${snapshot.status}`} />
          <div>
            <strong>{statusLabels[snapshot.status]}</strong>
            <span>{runtimeMessage}</span>
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
          <div className="overlay-control">
            <label className="switch-control">
              <input type="checkbox" checked={config.overlayVisible} onChange={toggleOverlay} />
              <span className="switch-track" />
              <span>悬浮窗</span>
            </label>
            <label className="overlay-opacity" title={`悬浮窗透明度 ${Math.round(config.overlayOpacity * 100)}%`}>
              <span>{Math.round(config.overlayOpacity * 100)}%</span>
              <input
                type="range"
                min="30"
                max="100"
                step="1"
                value={Math.round(config.overlayOpacity * 100)}
                disabled={!config.overlayVisible}
                aria-label="悬浮窗透明度"
                onChange={(event) => updateConfig("overlayOpacity", Number(event.target.value) / 100)}
              />
            </label>
          </div>
          <button className="button secondary stop-button" type="button" onClick={() => runAction("stop")} disabled={!running || backend.status !== "ok"} title={backend.status !== "ok" ? "原生后端不可用，停止动作已禁用" : undefined}>
            <Icon name="stop" />停止 <kbd>{formatHotkey(config.hotkeys.stop)}</kbd>
          </button>
          <button className="button primary" type="button" onClick={() => runAction("start")} disabled={running || updateBusy || backend.status !== "ok"} title={backend.status !== "ok" ? backend.message ?? "原生后端握手未完成" : updateBusy ? "更新期间暂时不能启动动作" : undefined}>
            <Icon name="play" />启动 <kbd>{formatHotkey(config.hotkeys.start)}</kbd>
          </button>
          <div className="window-controls" aria-label="窗口控制">
            <button type="button" onClick={() => void minimizeWindow()} aria-label="最小化" title="最小化"><Icon name="minimize" /></button>
            <button className="window-close" type="button" onClick={() => void closeWindow()} aria-label="关闭" title="关闭"><Icon name="close" /></button>
          </div>
        </div>
      </header>

      {error && <div className="error-banner" role="alert"><strong>操作未完成</strong><span>{error}</span><button className="text-button error-copy" type="button" onClick={() => void navigator.clipboard?.writeText(error).catch(() => undefined)} title="复制完整错误">复制</button><button type="button" onClick={() => setError(null)} aria-label="关闭错误提示">×</button></div>}
      {showUpdateNotice && (
        <section className={`update-banner ${updateNotice.phase}`} aria-live="polite" aria-label="应用更新">
          <span className="update-mark" aria-hidden="true"><Icon name="refresh" /></span>
          <div className="update-copy">
            <strong>{updateTitle}</strong>
            <span>{updateMessage}</span>
            {updateNotice.phase === "available" && running && <small>动作序列正在运行，请先停止并等待输入释放后再更新。</small>}
            {updateBusy && (
              <div
                className={`update-progress${updateProgress === undefined ? " indeterminate" : ""}`}
                role="progressbar"
                aria-label="更新进度"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={updateProgress}
              >
                <span style={updateProgress === undefined ? undefined : { width: `${updateProgress}%` }} />
              </div>
            )}
          </div>
          <div className="update-actions">
            {updateNotice.phase === "available" && (
              <>
                <button className="text-button" type="button" onClick={skipAvailableUpdate}>跳过此版本</button>
                <button className="button secondary" type="button" onClick={deferUpdate}>稍后提醒</button>
                <button className="button primary" type="button" onClick={() => void installAvailableUpdate()} disabled={running} title={running ? "请先停止当前动作序列" : "下载并安装更新"}>立即更新</button>
              </>
            )}
            {updateNotice.phase === "error" && (
              <button className="button secondary" type="button" onClick={() => void checkForUpdates(true)}>重新检查</button>
            )}
            {["current", "error"].includes(updateNotice.phase) && (
              <button className="text-button" type="button" onClick={() => setUpdateNotice({ phase: "idle" })}>关闭</button>
            )}
          </div>
        </section>
      )}

      <main>
        <nav className="workspace-tabs" aria-label="校准分组">
          {([
            ["display", "显示与灵敏度", "display"],
            ["aim", "瞄点调整", "target"],
            ["timing", "动作时序", "clock"],
          ] as const).map(([id, label, icon]) => (
            <button key={id} className={tab === id ? "active" : ""} onClick={() => setTab(id)} aria-pressed={tab === id}>
              <Icon name={icon} />{label}
            </button>
          ))}
        </nav>

        <div className="calibration-grid">
          <section className={`calibration-column display-column ${tab === "display" ? "mobile-active" : ""}`} aria-labelledby="display-title">
            <div className="column-heading">
              <span className="column-icon"><Icon name="display" /></span>
              <div><h2 id="display-title">显示与灵敏度</h2><p>确认游戏画面尺寸，再换算鼠标移动量。</p></div>
            </div>
            <div className="segment-control" role="group" aria-label="分辨率来源">
              <button type="button" className={config.resolutionMode === "auto" ? "active" : ""} aria-pressed={config.resolutionMode === "auto"} onClick={() => updateConfig("resolutionMode", "auto")}>自动识别</button>
              <button type="button" className={config.resolutionMode === "manual" ? "active" : ""} aria-pressed={config.resolutionMode === "manual"} onClick={() => updateConfig("resolutionMode", "manual")}>手动设置</button>
            </div>
            {config.resolutionMode === "auto" && (
              <div className="resolution-result">
                <div><span>{resolution?.detectedGame ? "已识别 Destiny 2 客户区" : "当前动作分辨率"}</span><strong>{currentResolution}</strong></div>
                <button className="icon-button" onClick={refreshResolution} aria-label="重新检测分辨率" title="重新检测"><Icon name="refresh" /></button>
              </div>
            )}
            {config.resolutionMode === "manual" && (
              <div className="pair-fields">
                <NumberField label="宽度" value={config.manualWidth} min={640} max={7680} onChange={(value) => updateConfig("manualWidth", value)} unit="px" />
                <NumberField label="高度" value={config.manualHeight} min={480} max={4320} onChange={(value) => updateConfig("manualHeight", value)} unit="px" />
              </div>
            )}
            <div className="subsection-title"><span>游戏内设置</span><small>灵敏度参考 15 / 1.00</small></div>
            <div className="game-settings-grid">
              <NumberField label="视角灵敏度" value={config.lookSensitivity} min={1} max={100} step={1} onChange={(value) => updateConfig("lookSensitivity", value)} />
              <NumberField label="瞄准灵敏度" value={config.adsModifier} min={0.5} max={1.5} step={0.1} onChange={(value) => updateConfig("adsModifier", value)} />
              <NumberField label="视野范围 (FOV)" value={config.fieldOfView} min={55} max={105} step={1} unit="°" onChange={(value) => updateConfig("fieldOfView", value)} />
            </div>
            <div className="scale-readout">
              <div><span>ADS 距离系数</span><strong>{applied.adsScale.toFixed(3)}×</strong></div>
              <div><span>腰射距离系数</span><strong>{applied.lookScale.toFixed(3)}×</strong></div>
            </div>
            <p className="info-note">按 15 / 1.00 的参考灵敏度换算相对鼠标计数。FOV 仅用于设置核对，不改变固定世界方向所需的转向量。</p>
          </section>

          <section className={`calibration-column aim-column ${tab === "aim" ? "mobile-active" : ""}`} aria-labelledby="aim-title">
            <div className="column-heading">
              <span className="column-icon"><Icon name="target" /></span>
              <div>
                <div className="heading-title-row">
                  <h2 id="aim-title">瞄点调整</h2>
                  <button className="xy-help" type="button" aria-describedby="xy-help-tooltip">
                    XY 说明
                    <span id="xy-help-tooltip" role="tooltip">X 正值向右、负值向左；Y 正值向下、负值向上。Y 轴采用屏幕坐标，与数学直角坐标系方向相反。</span>
                  </button>
                </div>
                <p>选择首次转向方式，再修正后续落点。</p>
              </div>
            </div>
            <div className="aim-mode-panel">
              <div className="segment-control aim-mode-control" role="group" aria-label="首次转向模式">
                <button type="button" className={usesAds ? "active" : ""} aria-pressed={usesAds} onClick={() => updateConfig("firstAimMode", "ads")}>无礼言论</button>
                <button type="button" className={!usesAds ? "active" : ""} aria-pressed={!usesAds} onClick={() => updateConfig("firstAimMode", "hipfire")}>通用配枪</button>
              </div>
              <p className="mode-note">{usesAds
                ? "按住右键完成首次转向；需要无礼言论的 20 Zoom 作为校准基准。"
                : "全程腰射完成首次转向并直接近战；使用普通视角灵敏度换算。"}</p>
            </div>
            <div className="aim-editor-grid">
              <div className="aim-context">
                <AimPreview label={selectedPreview.label} reference={selectedPreview.reference} value={selectedPreview.value} />
              </div>
              <VectorEditor
                title={firstAimLabel}
                description={usesAds ? "按住右键时的近战前转向" : "腰射状态下转向后直接近战"}
                base={usesAds ? config.firstAdsBase : config.firstHipBase}
                applied={usesAds ? applied.firstAds : applied.firstHip}
                onBaseChange={(value) => updateConfig(usesAds ? "firstAdsBase" : "firstHipBase", value)}
                selected={selectedVector === "first"}
                onSelect={() => setSelectedVector("first")}
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
            </div>
          </section>

          <section className={`calibration-column timing-column ${tab === "timing" ? "mobile-active" : ""}`} aria-labelledby="timing-title">
            <div className="column-heading">
              <span className="column-icon"><Icon name="clock" /></span>
              <div><h2 id="timing-title">动作时序</h2><p>调整每段等待，{formatHotkey(config.hotkeys.stop)} 随时可停。</p></div>
            </div>
            <div className="timing-list">
              <NumberField label="飞升后等待" value={config.timings.ascensionWait} min={0} max={10} step={0.05} unit="秒" onChange={(value) => updateTiming("ascensionWait", value)} hint={`${formatKey(config.gameKeys.ascension)} 飞升至后退定位`} />
              <NumberField label="近战额外等待" value={config.timings.meleeExtraWait} min={0} max={5} step={0.05} unit="秒" onChange={(value) => updateTiming("meleeExtraWait", value)} hint={`${firstAimLabel}到 ${formatKey(config.gameKeys.melee)} 近战前追加`} />
              <NumberField label="首次转向至超能" value={config.timings.adsToSuperWait} min={0} max={10} step={0.05} unit="秒" onChange={(value) => updateTiming("adsToSuperWait", value)} hint="从首次转向阶段开始计算" />
              <NumberField label="超能后等待" value={config.timings.superWait} min={0} max={10} step={0.05} unit="秒" onChange={(value) => updateTiming("superWait", value)} hint={`${formatKey(config.gameKeys.superAbility)} 释放后至冲刺`} />
              <NumberField label="冲刺侧移时间" value={config.timings.sprintATime} min={0} max={3} step={0.01} unit="秒" onChange={(value) => updateTiming("sprintATime", value)} hint="A 先行按下时长" />
              <NumberField label="冲刺至终结" value={config.timings.sprintToFinisher} min={0} max={5} step={0.05} unit="秒" onChange={(value) => updateTiming("sprintToFinisher", value)} hint="镜头微调后附加" />
            </div>
          </section>
        </div>
      </main>

      <footer className="app-footer">
        <div className="footer-actions">
          <button className="guide-button" type="button" onClick={() => setOpenPanel("guide")}><Icon name="help" />使用说明|有问题先看我</button>
          <button type="button" onClick={() => setOpenPanel("keys")}><Icon name="keyboard" />按键设置</button>
          <button type="button" onClick={() => setOpenPanel("diagnostics")} title="后端、热键、输入与环境自检，支持导出诊断包"><Icon name="pulse" />诊断</button>
          <button className="check-update-button" type="button" onClick={() => void checkForUpdates(true)} disabled={updateBusy || updateNotice.phase === "checking"} title={`当前版本 ${formatVersion(appVersion)}`}>
            <Icon name="refresh" />{updateNotice.phase === "checking" ? "检查中" : "检查更新"}<small>{formatVersion(appVersion)}</small>
          </button>
          <button type="button" onClick={resetParameters} title="保留键位与悬浮窗设置"><Icon name="reset" />恢复默认参数</button>
        </div>
        <div className="footer-meta">
          <span className="feedback-channel" title="问题反馈与交流QQ群">
            反馈QQ群 <strong>1104108070</strong>
          </span>
          <div className={`save-indicator ${saveState}`} aria-live="polite">
            <span />
            {saveState === "saving" ? "正在保存" : saveState === "error" ? "保存失败" : "设置已保存"}
          </div>
        </div>
      </footer>

      {openPanel === "guide" && (
        <UsageGuide usesAds={usesAds} startHotkey={formatHotkey(config.hotkeys.start)} onClose={closeGuide} onOpenKeys={openKeysFromGuide} />
      )}

      {openPanel === "keys" && (
        <AppDialog title="按键设置" description="点击键位框，再按下键盘按键、组合键或鼠标侧键；改动会自动保存。" onClose={() => setOpenPanel(null)} wide>
          {running && <div className="settings-warning" role="status">动作运行期间不能修改按键。请先停止当前流程。</div>}
          <section className="key-section" aria-labelledby="hotkey-settings-title">
            <div className="settings-section-heading">
              <div><h3 id="hotkey-settings-title">程序热键</h3><p>直接按下主键或 Ctrl、Shift、Alt、Win 组合；也支持鼠标中键和侧键。</p></div>
              <button className="text-button" type="button" onClick={resetKeyBindings} disabled={running} title="仅恢复程序热键和游戏内操作键"><Icon name="reset" />恢复默认键位</button>
            </div>
            <div className="hotkey-grid">
              <KeyCaptureField label="启动流程" value={config.hotkeys.start} disabled={running} mode="hotkey" onChange={(value) => updateHotkey("start", value)} />
              <KeyCaptureField label="停止流程" value={config.hotkeys.stop} disabled={running} mode="hotkey" onChange={(value) => updateHotkey("stop", value)} />
            </div>
          </section>
          <section className="key-section" aria-labelledby="game-key-settings-title">
            <div className="settings-section-heading"><div><h3 id="game-key-settings-title">游戏内操作</h3><p>支持键盘、鼠标中键与两个侧键；WASD 固定移动，ADS 固定鼠标右键。</p></div></div>
            <div className="game-key-grid">
              {gameKeyRows.map(([key, label, fallback]) => (
                <div className="key-select-row" key={key}>
                  <span><strong>{label}</strong><small>默认 {fallback}</small></span>
                  <KeyCaptureField label={label} value={config.gameKeys[key]} disabled={running} mode="game" onChange={(value) => updateGameKey(key, value)} />
                </div>
              ))}
            </div>
          </section>
          <div className="dialog-actions"><button className="button primary" type="button" onClick={() => setOpenPanel(null)}>完成</button></div>
        </AppDialog>
      )}

      {openPanel === "diagnostics" && (
        <AppDialog
          title="诊断面板"
          description="原生后端、全局热键、SendInput 主动输入与环境自检；支持复制原始错误并导出 ZIP 诊断包。"
          onClose={() => setOpenPanel(null)}
          wide
          className="diag-dialog"
        >
          <DiagnosticsPanel
            backend={backend}
            lastRuntimeError={lastRuntimeError}
            onBackendState={setBackend}
            onClose={() => setOpenPanel(null)}
          />
        </AppDialog>
      )}
    </div>
  );
}
