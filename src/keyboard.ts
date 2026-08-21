export interface KeyOption {
  value: string;
  label: string;
}

const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((letter) => ({
  value: `Key${letter}`,
  label: letter,
}));
const digits = Array.from({ length: 10 }, (_, index) => ({
  value: `Digit${index}`,
  label: String(index),
}));
const functionKeys = Array.from({ length: 12 }, (_, index) => ({
  value: `F${index + 1}`,
  label: `F${index + 1}`,
}));

export const hotkeyOptions: KeyOption[] = [
  ...functionKeys,
  ...letters,
  ...digits,
  { value: "Space", label: "Space" },
  { value: "Tab", label: "Tab" },
  { value: "Enter", label: "Enter" },
  { value: "Escape", label: "Esc" },
  { value: "ArrowUp", label: "↑" },
  { value: "ArrowDown", label: "↓" },
  { value: "ArrowLeft", label: "←" },
  { value: "ArrowRight", label: "→" },
  { value: "Home", label: "Home" },
  { value: "End", label: "End" },
  { value: "PageUp", label: "Page Up" },
  { value: "PageDown", label: "Page Down" },
  { value: "Insert", label: "Insert" },
  { value: "Delete", label: "Delete" },
];

export const gameKeyOptions: KeyOption[] = [
  ...letters.filter(({ label }) => !["W", "A", "S", "D"].includes(label)),
  ...digits,
  ...functionKeys,
  { value: "Space", label: "Space" },
  { value: "ShiftLeft", label: "左 Shift" },
  { value: "ShiftRight", label: "右 Shift" },
  { value: "ControlLeft", label: "左 Ctrl" },
  { value: "AltLeft", label: "左 Alt" },
  { value: "Tab", label: "Tab" },
  { value: "CapsLock", label: "Caps Lock" },
  { value: "Enter", label: "Enter" },
  { value: "Backspace", label: "Backspace" },
  { value: "Escape", label: "Esc" },
  { value: "Backquote", label: "`" },
  { value: "Minus", label: "-" },
  { value: "Equal", label: "=" },
  { value: "BracketLeft", label: "[" },
  { value: "BracketRight", label: "]" },
  { value: "Backslash", label: "\\" },
  { value: "Semicolon", label: ";" },
  { value: "Quote", label: "'" },
  { value: "Comma", label: "," },
  { value: "Period", label: "." },
  { value: "Slash", label: "/" },
];

const optionLabels = new Map([...hotkeyOptions, ...gameKeyOptions].map(({ value, label }) => [value, label]));

export type HotkeyModifier = "Control" | "Shift" | "Alt" | "Super";

export interface ParsedHotkey {
  primary: string;
  modifiers: Set<HotkeyModifier>;
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

export function formatKey(code: string): string {
  return optionLabels.get(code) ?? code.replace(/^Key/, "").replace(/^Digit/, "");
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
