export type ResolutionMode = "auto" | "manual";
export type FirstAimMode = "ads" | "hipfire";
export type RuntimeStatus =
  | "ready"
  | "running"
  | "stopping"
  | "completed"
  | "aborted"
  | "error";

export interface TimingConfig {
  strafeToFlagWait: number;
  flagToClaimWait: number;
  claimToWeaponWait: number;
  weaponToMoveWait: number;
  positionToAimWait: number;
  aimToMeleeWait: number;
  meleeToSuperWait: number;
  superToSprintWait: number;
  sprintToFinisher: number;
}

export interface HotkeyConfig {
  start: string;
  stop: string;
}

export interface GameKeyConfig {
  sprint: string;
  jump: string;
  interact: string;
  weaponSlot2: string;
  melee: string;
  ascension: string;
  superAbility: string;
  finisher: string;
}

export interface AppConfig {
  resolutionMode: ResolutionMode;
  manualWidth: number;
  manualHeight: number;
  lookSensitivity: number;
  adsModifier: number;
  fieldOfView: number;
  referenceLookSensitivity: number;
  referenceAdsModifier: number;
  referenceFieldOfView: number;
  firstAimMode: FirstAimMode;
  firstAdsBase: [number, number];
  firstHipBase: [number, number];
  voidArrowBase: [number, number];
  voidArrowTrim: [number, number];
  sprintBase: [number, number];
  sprintTrim: [number, number];
  timings: TimingConfig;
  hotkeys: HotkeyConfig;
  gameKeys: GameKeyConfig;
  overlayVisible: boolean;
  overlayOpacity: number;
  usageGuideSeen: boolean;
}

export interface AppliedOffsets {
  adsScale: number;
  lookScale: number;
  firstAds: [number, number];
  firstHip: [number, number];
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

export interface NativePingResult {
  ok: boolean;
  mock?: boolean;
  version: string;
  commitSha: string;
  commitShort: string;
  buildProfile: string;
  os: string;
  arch: string;
  pid?: number;
  timestamp: string;
}

export interface HotkeyEventEntry {
  timestamp: string;
  action: string;
  ok: boolean;
  message: string;
  error?: string | null;
  shortcut?: string | null;
}

export interface HotkeyStatusEntry {
  role: string;
  label: string;
  configured: string;
  parsed: boolean;
  parsedError?: string | null;
  registered: boolean;
  registerError?: string | null;
  isRegistered?: boolean | null;
  events: HotkeyEventEntry[];
  updatedAt: string;
}

export interface InputProbeResult {
  probe: string;
  label: string;
  description: string;
  ok: boolean;
  requested: number;
  calls: number;
  sent: number;
  lastErrorCode?: number | null;
  lastError?: string | null;
  foregroundProcess?: string | null;
  integrityLevel?: string | null;
  observedAsyncDown?: boolean | null;
  durationMs: number;
  timestamp: string;
}

export interface EnvironmentInfo {
  os: string;
  osVersion?: string | null;
  arch: string;
  appVersion: string;
  commitSha: string;
  buildProfile: string;
  configPath: string;
  logPath: string;
  downloadsPath: string;
  foregroundProcess?: string | null;
  foregroundIntegrity?: string | null;
  sessionUptimeS: number;
  generatedAt: string;
}

export interface DiagnosticEvent {
  timestamp: string;
  level: string;
  category: string;
  event: string;
  message: string;
  error?: string | null;
  details?: unknown;
}

export interface DiagnosticsExportResult {
  path: string;
  fileCount: number;
  sizeBytes: number;
  exportedAt: string;
}

export interface BackendHandshakeState {
  status: "checking" | "ok" | "unavailable";
  message?: string;
  ping?: NativePingResult;
  latencyMs?: number;
}
