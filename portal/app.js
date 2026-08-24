const pageThemeKey = "d2-morgeth-kick-portal-theme";
const pageThemeButton = document.querySelector(".site-theme-toggle");
const themeColor = document.querySelector('meta[name="theme-color"]');
const preview = document.querySelector(".console-window");
const previewButtons = document.querySelectorAll("[data-preview-theme]");

function setPageTheme(theme, persist = true) {
  document.documentElement.dataset.theme = theme;
  themeColor?.setAttribute("content", theme === "dark" ? "#10141a" : "#eef0f3");
  pageThemeButton?.setAttribute("aria-label", `切换为${theme === "dark" ? "浅色" : "深色"}模式`);
  pageThemeButton?.setAttribute("title", `切换为${theme === "dark" ? "浅色" : "深色"}模式`);
  const label = pageThemeButton?.querySelector("span");
  if (label) label.textContent = theme === "dark" ? "浅色" : "深色";
  if (!persist) return;
  try {
    localStorage.setItem(pageThemeKey, theme);
  } catch {
    // The current page can still switch themes when storage is unavailable.
  }
}

function setPreviewTheme(theme) {
  if (!preview) return;
  preview.dataset.preview = theme;
  previewButtons.forEach((item) => {
    item.setAttribute("aria-pressed", String(item.dataset.previewTheme === theme));
  });
}

pageThemeButton?.addEventListener("click", () => {
  const next = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
  setPageTheme(next);
});

previewButtons.forEach((button) => {
  button.addEventListener("click", () => {
    const next = button.dataset.previewTheme;
    if (!next || !preview) return;
    setPreviewTheme(next);
  });
});

const initialTheme = document.documentElement.dataset.theme === "dark" ? "dark" : "light";
setPageTheme(initialTheme, false);
setPreviewTheme(initialTheme);

fetch("https://api.github.com/repos/MIGO-OvO/D2-Morgeth-Kick/releases/latest", {
  headers: { Accept: "application/vnd.github+json" },
})
  .then((response) => response.ok ? response.json() : Promise.reject(new Error("Release unavailable")))
  .then((release) => {
    const version = document.querySelector("#release-version");
    if (version && release.tag_name) version.textContent = release.tag_name;
    const asset = release.assets?.find((item) => item.name === "D2-Morgeth-Kick-Windows-x64-setup.exe");
    if (asset?.browser_download_url) {
      document.querySelectorAll('a[href*="/releases/latest/download/"]').forEach((link) => {
        link.href = asset.browser_download_url;
      });
    }
  })
  .catch(() => undefined);
