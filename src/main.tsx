import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import OverlayApp from "./OverlayApp";
import { applyTheme, resolveTheme } from "./theme";
import "./styles.css";

const params = new URLSearchParams(window.location.search);
const isOverlayView = params.get("view") === "overlay";
document.documentElement.dataset.view = isOverlayView ? "overlay" : "main";
applyTheme(resolveTheme());
const RootComponent = isOverlayView ? OverlayApp : App;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <RootComponent />
  </StrictMode>,
);
