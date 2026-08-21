export type Theme = "light" | "dark";

export const THEME_STORAGE_KEY = "d2-ogre-kick-theme";

export function resolveTheme(): Theme {
  const preset = document.documentElement.dataset.theme;
  if (preset === "light" || preset === "dark") return preset;

  try {
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {
    // The system preference remains available when storage is blocked.
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(theme: Theme, persist = false) {
  document.documentElement.dataset.theme = theme;
  document.querySelector('meta[name="theme-color"]')?.setAttribute(
    "content",
    theme === "dark" ? "#10141a" : "#eef0f3",
  );
  if (!persist) return;

  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // Theme switching still works for the current session.
  }
}
