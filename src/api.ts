import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import type {
  AppConfig,
  AppliedOffsets,
  ResolutionInfo,
  RuntimeSnapshot,
} from "./types";

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
  firstHipBase: [-1300, 25],
  voidArrowBase: [-300, 81],
  voidArrowTrim: [0, 0],
  sprintBase: [280, 0],
  sprintTrim: [0, 0],
  timings: {
    ascensionWait: 1.6,
    meleeExtraWait: 0.5,
    adsToSuperWait: 2.5,
    superWait: 1.8,
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
  return isTauri() ? invoke<AppConfig>("get_config") : structuredClone(mockConfig);
}

export async function saveConfig(config: AppConfig): Promise<AppConfig> {
  if (isTauri()) return invoke<AppConfig>("save_config", { config });
  mockConfig = structuredClone(config);
  mockConfigListeners.forEach((listener) => listener(structuredClone(mockConfig)));
  return structuredClone(mockConfig);
}

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  return isTauri() ? invoke<RuntimeSnapshot>("get_runtime_snapshot") : structuredClone(mockRuntime);
}

export async function detectResolution(): Promise<ResolutionInfo> {
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
  if (isTauri()) return invoke<RuntimeSnapshot>("start_sequence");
  startMock();
  return structuredClone(mockRuntime);
}

export async function stopSequence(): Promise<RuntimeSnapshot> {
  if (isTauri()) return invoke<RuntimeSnapshot>("stop_sequence");
  if (mockTimer) window.clearInterval(mockTimer);
  emitMock({ ...mockRuntime, status: "aborted", message: "已由 F10 安全中止" });
  return structuredClone(mockRuntime);
}

export async function setHotkeyCaptureActive(active: boolean): Promise<void> {
  if (isTauri()) await invoke("set_hotkey_capture_active", { active });
}

export async function setOverlayVisible(visible: boolean): Promise<boolean> {
  if (isTauri()) return invoke<boolean>("set_overlay_visible", { visible });
  mockConfig.overlayVisible = visible;
  return visible;
}

export async function checkForAppUpdate(): Promise<AppUpdate | null> {
  if (!isTauri()) {
    if (!import.meta.env.DEV || !new URLSearchParams(window.location.search).has("mockUpdate")) return null;
    return {
      available: true,
      currentVersion: "0.3.2",
      version: "0.3.3",
      date: new Date().toISOString(),
      body: "改进更新提醒，并修复长时间运行时的状态同步。",
      rawJson: {},
      close: async () => undefined,
    } as AppUpdate;
  }
  return check({ timeout: 15_000 });
}

export async function getAppVersion(): Promise<string> {
  return isTauri() ? getVersion() : "0.3.2";
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

export async function toggleMaximizeWindow(): Promise<void> {
  if (isTauri()) await getCurrentWindow().toggleMaximize();
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
