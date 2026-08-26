import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import type {
  AppConfig,
  AppliedOffsets,
  DiagnosticEvent,
  DiagnosticsExportResult,
  EnvironmentInfo,
  HotkeyStatusEntry,
  InputProbeResult,
  NativePingResult,
  ResolutionInfo,
  RuntimeSnapshot,
} from "./types";

export const PROD_MOCK_DISABLED_MESSAGE =
  "原生后端不可用：正式构建已禁止静默进入前端模拟模式。请使用安装版程序或 npm run tauri dev 启动。";

/** 把任意异常转成完整可读错误文本（保留后端返回的原始错误，不截断）。 */
export function errorText(reason: unknown): string {
  if (typeof reason === "string" && reason.trim()) return reason;
  if (reason instanceof Error && reason.message) return reason.message;
  if (reason && typeof reason === "object") {
    const text = String(reason);
    if (text !== "[object Object]") return text;
  }
  return String(reason ?? "未知错误");
}

export const defaultConfig: AppConfig = {
  resolutionMode: "auto",
  manualWidth: 1920,
  manualHeight: 1080,
  lookSensitivity: 15,
  adsModifier: 1,
  fieldOfView: 100,
  referenceLookSensitivity: 15,
  referenceAdsModifier: 1,
  referenceFieldOfView: 100,
  firstAimMode: "ads",
  firstAdsBase: [-2600, 50],
  firstHipBase: [-1320, 30],
  voidArrowBase: [-300, 81],
  voidArrowTrim: [-50, 0],
  sprintBase: [280, 0],
  sprintTrim: [0, 0],
  timings: {
    ascensionWait: 1.5,
    meleeExtraWait: 0.3,
    adsToSuperWait: 2.5,
    superWait: 1.9,
    sprintATime: 0.1,
    sprintToFinisher: 0,
  },
  hotkeys: { start: "F8", stop: "F10" },
  gameKeys: {
    sprint: "ShiftLeft",
    jump: "Space",
    interact: "KeyE",
    weaponSlot2: "Digit2",
    melee: "KeyC",
    ascension: "KeyX",
    superAbility: "KeyF",
    finisher: "KeyG",
  },
  overlayVisible: true,
  overlayOpacity: 0.88,
  usageGuideSeen: false,
};

export const defaultRuntime: RuntimeSnapshot = {
  status: "ready",
  phaseIndex: 0,
  phaseName: "进场准备",
  message: "参数已就绪，按 F8 启动",
};

const isTauri = () => "__TAURI_INTERNALS__" in window;

/**
 * 正式构建（PROD）禁止静默进入前端 mock：
 * 非 Tauri 环境下所有动作相关 API 直接抛错，UI 据此禁用动作执行。
 * 仅开发模式（npm run dev 浏览器预览）保留模拟数据用于界面调试。
 */
function requireTauri(): void {
  if (isTauri()) return;
  if (import.meta.env.PROD) throw new Error(PROD_MOCK_DISABLED_MESSAGE);
}

export type AppUpdate = Update;
export type AppUpdateDownloadEvent = DownloadEvent;

let mockConfig = structuredClone(defaultConfig);
let mockRuntime = structuredClone(defaultRuntime);
const mockListeners = new Set<(value: RuntimeSnapshot) => void>();
const mockConfigListeners = new Set<(value: AppConfig) => void>();
let mockTimer: number | undefined;

function emitMock(next: RuntimeSnapshot) {
  mockRuntime = next;
  mockListeners.forEach((listener) => listener(next));
}

function startMock() {
  if (mockRuntime.status === "running") return;
  emitMock({
    status: "running",
    phaseIndex: 0,
    phaseName: "进场准备",
    message: "动作序列正在执行",
  });
  let phase = 0;
  mockTimer = window.setInterval(() => {
    phase += 1;
    const firstAimPhase = mockConfig.firstAimMode === "ads" ? "ADS 近战" : "腰射近战";
    const phases = ["进场准备", "飞升", "后退定位", firstAimPhase, "虚空箭", "冲刺", "终结"];
    if (phase >= phases.length) {
      if (mockTimer) window.clearInterval(mockTimer);
      emitMock({
        status: "completed",
        phaseIndex: 6,
        phaseName: "终结",
        message: "动作序列已完成",
      });
      return;
    }
    emitMock({
      status: "running",
      phaseIndex: phase,
      phaseName: phases[phase],
      message: `正在执行：${phases[phase]}`,
    });
  }, 1700);
}

export function calculateAppliedOffsets(config: AppConfig): AppliedOffsets {
  const adsScale =
    (config.referenceLookSensitivity * config.referenceAdsModifier) /
    (config.lookSensitivity * config.adsModifier);
  const lookScale =
    config.referenceLookSensitivity / config.lookSensitivity;
  const apply = (base: [number, number], scale: number, trim: [number, number] = [0, 0]) =>
    base.map((value, index) => Math.round(value * scale + trim[index])) as [number, number];

  return {
    adsScale,
    lookScale,
    firstAds: apply(config.firstAdsBase, adsScale),
    firstHip: apply(config.firstHipBase, lookScale),
    voidArrow: apply(config.voidArrowBase, lookScale, config.voidArrowTrim),
    sprint: apply(config.sprintBase, lookScale, config.sprintTrim),
  };
}

export async function getConfig(): Promise<AppConfig> {
  requireTauri();
  return isTauri() ? invoke<AppConfig>("get_config") : structuredClone(mockConfig);
}

export async function saveConfig(config: AppConfig): Promise<AppConfig> {
  requireTauri();
  if (isTauri()) return invoke<AppConfig>("save_config", { config });
  mockConfig = structuredClone(config);
  mockConfigListeners.forEach((listener) => listener(structuredClone(mockConfig)));
  return structuredClone(mockConfig);
}

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  requireTauri();
  return isTauri() ? invoke<RuntimeSnapshot>("get_runtime_snapshot") : structuredClone(mockRuntime);
}

export async function detectResolution(): Promise<ResolutionInfo> {
  requireTauri();
  if (isTauri()) return invoke<ResolutionInfo>("detect_resolution");
  return {
    width: 1920,
    height: 1080,
    detectedGame: true,
    source: "destiny-window",
    windowTitle: "Destiny 2",
    dpi: 96,
  };
}

export async function startSequence(): Promise<RuntimeSnapshot> {
  requireTauri();
  if (isTauri()) return invoke<RuntimeSnapshot>("start_sequence");
  startMock();
  return structuredClone(mockRuntime);
}

export async function stopSequence(): Promise<RuntimeSnapshot> {
  requireTauri();
  if (isTauri()) return invoke<RuntimeSnapshot>("stop_sequence");
  if (mockTimer) window.clearInterval(mockTimer);
  emitMock({ ...mockRuntime, status: "aborted", message: "已由 F10 安全中止" });
  return structuredClone(mockRuntime);
}

export async function setHotkeyCaptureActive(active: boolean): Promise<void> {
  requireTauri();
  if (isTauri()) await invoke("set_hotkey_capture_active", { active });
}

export async function setOverlayVisible(visible: boolean): Promise<boolean> {
  requireTauri();
  if (isTauri()) return invoke<boolean>("set_overlay_visible", { visible });
  mockConfig.overlayVisible = visible;
  return visible;
}

export async function checkForAppUpdate(): Promise<AppUpdate | null> {
  if (!isTauri()) {
    if (!import.meta.env.DEV || !new URLSearchParams(window.location.search).has("mockUpdate")) return null;
    return {
      available: true,
      currentVersion: "0.3.5",
      version: "0.3.6",
      date: new Date().toISOString(),
      body: "改进更新提醒，并修复长时间运行时的状态同步。",
      rawJson: {},
      close: async () => undefined,
    } as AppUpdate;
  }
  return check({ timeout: 15_000 });
}

export async function getAppVersion(): Promise<string> {
  return isTauri() ? getVersion() : "0.3.5";
}

export async function installAppUpdate(
  update: AppUpdate,
  onEvent: (event: AppUpdateDownloadEvent) => void,
): Promise<void> {
  if (!isTauri()) {
    const total = 4 * 1024 * 1024;
    const chunkLength = total / 4;
    onEvent({ event: "Started", data: { contentLength: total } });
    for (let index = 0; index < 4; index += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 120));
      onEvent({ event: "Progress", data: { chunkLength } });
    }
    onEvent({ event: "Finished" });
    return;
  }
  await invoke("prepare_for_update");
  try {
    await update.downloadAndInstall(onEvent, { timeout: 120_000 });
    await invoke("restart_after_update");
  } catch (reason) {
    await invoke("cancel_update_preparation").catch(() => undefined);
    throw reason;
  }
}

export async function minimizeWindow(): Promise<void> {
  if (isTauri()) await getCurrentWindow().minimize();
}

export async function closeWindow(): Promise<void> {
  if (isTauri()) await getCurrentWindow().close();
}

export async function onRuntimeState(
  listener: (value: RuntimeSnapshot) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<RuntimeSnapshot>("runtime-state", (event) => listener(event.payload));
  }
  mockListeners.add(listener);
  return () => mockListeners.delete(listener);
}

export async function onConfigState(
  listener: (value: AppConfig) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    return listen<AppConfig>("config-state", (event) => listener(event.payload));
  }
  mockConfigListeners.add(listener);
  return () => mockConfigListeners.delete(listener);
}

// ---------- 诊断 ----------

/** 原生后端握手：正式构建中失败即禁用动作执行。 */
export async function nativePing(): Promise<NativePingResult> {
  if (isTauri()) return invoke<NativePingResult>("native_ping");
  if (import.meta.env.PROD) throw new Error(PROD_MOCK_DISABLED_MESSAGE);
  return {
    ok: true,
    mock: true,
    version: "0.3.5",
    commitSha: "dev-preview",
    commitShort: "dev",
    buildProfile: "dev",
    os: "browser",
    arch: "web",
    timestamp: new Date().toISOString(),
  };
}

export async function getHotkeyStatus(): Promise<HotkeyStatusEntry[]> {
  requireTauri();
  if (isTauri()) return invoke<HotkeyStatusEntry[]>("get_hotkey_status");
  return [
    {
      role: "start",
      label: "启动热键",
      configured: "F8",
      parsed: true,
      registered: true,
      isRegistered: true,
      events: [],
      updatedAt: new Date().toISOString(),
    },
    {
      role: "stop",
      label: "停止热键",
      configured: "F10",
      parsed: true,
      registered: true,
      isRegistered: true,
      events: [],
      updatedAt: new Date().toISOString(),
    },
  ];
}

/** 主动输入自检（会注入一次极短 W 按键与 ±1 像素鼠标移动）。 */
export async function runInputProbes(): Promise<InputProbeResult[]> {
  requireTauri();
  if (isTauri()) return invoke<InputProbeResult[]>("run_input_probes");
  const now = new Date().toISOString();
  const okProbe = (probe: string, label: string, description: string): InputProbeResult => ({
    probe,
    label,
    description,
    ok: true,
    requested: 2,
    calls: 2,
    sent: 2,
    lastErrorCode: null,
    lastError: null,
    foregroundProcess: "dev-preview",
    integrityLevel: "Medium (0x2000)",
    observedAsyncDown: probe !== "mouse-relative" ? true : null,
    durationMs: 1,
    timestamp: now,
  });
  return [
    okProbe("scan-code-w", "扫描码 W", "开发预览模拟探针"),
    okProbe("virtual-key-w", "虚拟键 W", "开发预览模拟探针"),
    okProbe("mouse-relative", "相对鼠标移动", "开发预览模拟探针"),
  ];
}

export async function getEnvironmentInfo(): Promise<EnvironmentInfo> {
  requireTauri();
  if (isTauri()) return invoke<EnvironmentInfo>("get_environment_info");
  return {
    os: "browser",
    osVersion: null,
    arch: "web",
    appVersion: "0.3.5",
    commitSha: "dev-preview",
    buildProfile: "dev",
    configPath: "（浏览器预览无配置目录）",
    logPath: "（浏览器预览仅内存）",
    downloadsPath: "（浏览器预览）",
    foregroundProcess: null,
    foregroundIntegrity: null,
    sessionUptimeS: 0,
    generatedAt: new Date().toISOString(),
  };
}

export async function getDiagnosticEvents(limit = 300): Promise<DiagnosticEvent[]> {
  requireTauri();
  if (isTauri()) return invoke<DiagnosticEvent[]>("get_diagnostic_events", { limit });
  return [];
}

export async function exportDiagnosticsPackage(): Promise<DiagnosticsExportResult> {
  requireTauri();
  if (isTauri()) return invoke<DiagnosticsExportResult>("export_diagnostics_package");
  throw new Error("浏览器预览无法导出诊断包：请在桌面端使用该功能。");
}
