import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
  referenceLookSensitivity: 15,
  referenceAdsModifier: 1,
  firstAdsBase: [-2600, 50],
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
    finisherWait: 3,
  },
  overlayVisible: true,
};

export const defaultRuntime: RuntimeSnapshot = {
  status: "ready",
  phaseIndex: 0,
  phaseName: "进场准备",
  message: "参数已就绪，按 F8 启动",
};

const isTauri = () => "__TAURI_INTERNALS__" in window;

let mockConfig = structuredClone(defaultConfig);
let mockRuntime = structuredClone(defaultRuntime);
const mockListeners = new Set<(value: RuntimeSnapshot) => void>();
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
    const phases = ["进场准备", "飞升", "后退定位", "ADS 近战", "虚空箭", "冲刺", "终结"];
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
  const lookScale = config.referenceLookSensitivity / config.lookSensitivity;
  const apply = (base: [number, number], scale: number, trim: [number, number] = [0, 0]) =>
    base.map((value, index) => Math.round(value * scale + trim[index])) as [number, number];

  return {
    adsScale,
    lookScale,
    firstAds: apply(config.firstAdsBase, adsScale),
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

export async function setOverlayVisible(visible: boolean): Promise<boolean> {
  if (isTauri()) return invoke<boolean>("set_overlay_visible", { visible });
  mockConfig.overlayVisible = visible;
  return visible;
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
