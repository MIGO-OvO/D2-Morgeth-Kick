export type HotkeyModifier = "Control" | "Shift" | "Alt" | "Super";

export interface ParsedHotkey {
  primary: string;
  modifiers: Set<HotkeyModifier>;
}

const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((letter) => `Key${letter}`);
const digits = Array.from({ length: 10 }, (_, index) => `Digit${index}`);
const functionKeys = Array.from({ length: 24 }, (_, index) => `F${index + 1}`);
const numpadKeys = [
  ...Array.from({ length: 10 }, (_, index) => `Numpad${index}`),
  "NumpadAdd",
  "NumpadDecimal",
  "NumpadDivide",
  "NumpadEnter",
  "NumpadEqual",
  "NumpadMultiply",
  "NumpadSubtract",
];
const punctuationKeys = [
  "Backquote",
  "Minus",
  "Equal",
  "BracketLeft",
  "BracketRight",
  "Backslash",
  "Semicolon",
  "Quote",
  "Comma",
  "Period",
  "Slash",
];
const navigationKeys = [
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "Insert",
  "Delete",
];
const commonKeys = ["Space", "Tab", "Enter", "Backspace", "Escape", "CapsLock"];

const hotkeyPrimaryCodes = new Set([
  ...letters,
  ...digits,
  ...functionKeys,
  ...numpadKeys,
  ...punctuationKeys,
  ...navigationKeys,
  ...commonKeys,
  "Pause",
  "PrintScreen",
  "ScrollLock",
  "NumLock",
  "AudioVolumeDown",
  "AudioVolumeUp",
  "AudioVolumeMute",
  "MediaPlay",
  "MediaPause",
  "MediaPlayPause",
  "MediaStop",
  "MediaTrackNext",
  "MediaTrackPrevious",
]);

const reservedMovementCodes = new Set(["KeyW", "KeyA", "KeyS", "KeyD"]);
const gameKeyCodes = new Set([
  ...letters.filter((code) => !reservedMovementCodes.has(code)),
  ...digits,
  ...functionKeys.slice(0, 12),
  ...numpadKeys,
  ...punctuationKeys,
  ...navigationKeys,
  ...commonKeys,
  "ShiftLeft",
  "ShiftRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "NumLock",
  "MouseMiddle",
  "Mouse4",
  "Mouse5",
]);

const keyLabels: Record<string, string> = {
  Space: "Space",
  Tab: "Tab",
  Enter: "Enter",
  Backspace: "Backspace",
  Escape: "Esc",
  CapsLock: "Caps Lock",
  ShiftLeft: "左 Shift",
  ShiftRight: "右 Shift",
  ControlLeft: "左 Ctrl",
  ControlRight: "右 Ctrl",
  AltLeft: "左 Alt",
  AltRight: "右 Alt",
  MetaLeft: "左 Win",
  MetaRight: "右 Win",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Home: "Home",
  End: "End",
  PageUp: "Page Up",
  PageDown: "Page Down",
  Insert: "Insert",
  Delete: "Delete",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Pause: "Pause",
  PrintScreen: "Print Screen",
  ScrollLock: "Scroll Lock",
  NumLock: "Num Lock",
  NumpadAdd: "数字键盘 +",
  NumpadDecimal: "数字键盘 .",
  NumpadDivide: "数字键盘 /",
  NumpadEnter: "数字键盘 Enter",
  NumpadEqual: "数字键盘 =",
  NumpadMultiply: "数字键盘 *",
  NumpadSubtract: "数字键盘 -",
  AudioVolumeDown: "音量减",
  AudioVolumeUp: "音量加",
  AudioVolumeMute: "静音",
  MediaPlay: "媒体播放",
  MediaPause: "媒体暂停",
  MediaPlayPause: "媒体播放 / 暂停",
  MediaStop: "媒体停止",
  MediaTrackNext: "下一曲",
  MediaTrackPrevious: "上一曲",
  MouseMiddle: "鼠标中键",
  Mouse4: "鼠标侧键 1",
  Mouse5: "鼠标侧键 2",
};

for (const code of numpadKeys.slice(0, 10)) {
  keyLabels[code] = `数字键盘 ${code.replace("Numpad", "")}`;
}

export function parseHotkey(binding: string): ParsedHotkey {
  const parts = binding.split("+");
  const primary = parts.pop() || "F8";
  return {
    primary,
    modifiers: new Set(parts.filter((part): part is HotkeyModifier =>
      ["Control", "Shift", "Alt", "Super"].includes(part))),
  };
}

export function buildHotkey(primary: string, modifiers: Set<HotkeyModifier>): string {
  return ["Control", "Shift", "Alt", "Super"]
    .filter((modifier) => modifiers.has(modifier as HotkeyModifier))
    .concat(primary)
    .join("+");
}

export function hotkeyModifiersFromEvent(
  event: Pick<KeyboardEvent | MouseEvent, "ctrlKey" | "shiftKey" | "altKey" | "metaKey">,
): Set<HotkeyModifier> {
  const modifiers = new Set<HotkeyModifier>();
  if (event.ctrlKey) modifiers.add("Control");
  if (event.shiftKey) modifiers.add("Shift");
  if (event.altKey) modifiers.add("Alt");
  if (event.metaKey) modifiers.add("Super");
  return modifiers;
}

export function isModifierCode(code: string): boolean {
  return [
    "ControlLeft",
    "ControlRight",
    "ShiftLeft",
    "ShiftRight",
    "AltLeft",
    "AltRight",
    "MetaLeft",
    "MetaRight",
  ].includes(code);
}

export function isHotkeyPrimaryCode(code: string): boolean {
  return hotkeyPrimaryCodes.has(code);
}

export function isGameKeyCode(code: string): boolean {
  return gameKeyCodes.has(code);
}

export function isReservedMovementCode(code: string): boolean {
  return reservedMovementCodes.has(code);
}

export function mouseBindingFromButton(button: number): string | null {
  if (button === 1) return "MouseMiddle";
  if (button === 3) return "Mouse4";
  if (button === 4) return "Mouse5";
  return null;
}

export function formatKey(code: string): string {
  return keyLabels[code] ?? code.replace(/^Key/, "").replace(/^Digit/, "");
}

export function formatHotkey(binding: string): string {
  const labels: Record<HotkeyModifier, string> = {
    Control: "Ctrl",
    Shift: "Shift",
    Alt: "Alt",
    Super: "Win",
  };
  const { primary, modifiers } = parseHotkey(binding);
  return [...["Control", "Shift", "Alt", "Super"]
    .filter((modifier) => modifiers.has(modifier as HotkeyModifier))
    .map((modifier) => labels[modifier as HotkeyModifier]), formatKey(primary)].join(" + ");
}
