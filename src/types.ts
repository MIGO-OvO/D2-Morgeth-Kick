export type ResolutionMode = "auto" | "manual";
export type RuntimeStatus =
  | "ready"
  | "running"
  | "stopping"
  | "completed"
  | "aborted"
  | "error";

export interface TimingConfig {
  ascensionWait: number;
  meleeExtraWait: number;
  adsToSuperWait: number;
  superWait: number;
  sprintATime: number;
  sprintToFinisher: number;
  finisherWait: number;
}

export interface AppConfig {
  resolutionMode: ResolutionMode;
  manualWidth: number;
  manualHeight: number;
  lookSensitivity: number;
  adsModifier: number;
  referenceLookSensitivity: number;
  referenceAdsModifier: number;
  firstAdsBase: [number, number];
  voidArrowBase: [number, number];
  voidArrowTrim: [number, number];
  sprintBase: [number, number];
  sprintTrim: [number, number];
  timings: TimingConfig;
  overlayVisible: boolean;
}

export interface AppliedOffsets {
  adsScale: number;
  lookScale: number;
  firstAds: [number, number];
  voidArrow: [number, number];
  sprint: [number, number];
}

export interface RuntimeSnapshot {
  status: RuntimeStatus;
  phaseIndex: number;
  phaseName: string;
  message: string;
}

export interface ResolutionInfo {
  width: number;
  height: number;
  detectedGame: boolean;
  source: "destiny-window" | "primary-display" | "manual";
  windowTitle: string | null;
  dpi: number | null;
}
