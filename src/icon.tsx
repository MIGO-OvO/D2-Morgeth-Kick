import type { ReactNode } from "react";

export type IconName =
  | "play"
  | "stop"
  | "display"
  | "target"
  | "clock"
  | "refresh"
  | "sun"
  | "moon"
  | "help"
  | "keyboard"
  | "close"
  | "reset"
  | "minimize"
  | "pulse"
  | "download"
  | "copy"
  | "shield";

export function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, ReactNode> = {
    play: <path d="m9 7 8 5-8 5V7Z" />,
    stop: <path d="M8 8h8v8H8z" />,
    display: <><rect x="3" y="5" width="18" height="12" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    target: <><circle cx="12" cy="12" r="7" /><circle cx="12" cy="12" r="2" /><path d="M12 2v3m0 14v3M2 12h3m14 0h3" /></>,
    clock: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
    refresh: <><path d="M20 7v5h-5" /><path d="M18.5 16a7 7 0 1 1 .2-8.2L20 12" /></>,
    sun: <><circle cx="12" cy="12" r="3.5" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>,
    moon: <path d="M20 15.2A8.5 8.5 0 0 1 8.8 4a8.5 8.5 0 1 0 11.2 11.2Z" />,
    help: <><circle cx="12" cy="12" r="9" /><path d="M9.8 9a2.3 2.3 0 0 1 4.4 1c0 1.8-2.2 2-2.2 3.7M12 17.5h.01" /></>,
    keyboard: <><rect x="3" y="6" width="18" height="12" rx="2" /><path d="M7 10h.01M11 10h.01M15 10h.01M18 10h.01M7 14h7m2 0h2" /></>,
    close: <path d="m7 7 10 10M17 7 7 17" />,
    reset: <><path d="M4 4v6h6" /><path d="M5.5 15a7 7 0 1 0 .2-8.2L4 10" /></>,
    minimize: <path d="M6 16h12" />,
    pulse: <><path d="M3 12h4l2.5-6 4 12L16 12h5" /><circle cx="20" cy="7" r="1" /><circle cx="4" cy="17" r="1" /></>,
    download: <><path d="M12 3v12" /><path d="m7 10 5 5 5-5" /><path d="M4 20h16" /></>,
    copy: <><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V6a2 2 0 0 1 2-2h9" /></>,
    shield: <><path d="M12 3 5 6v5c0 4.4 3 8.4 7 10 4-1.6 7-5.6 7-10V6l-7-3Z" /><path d="m9.5 12 1.8 1.8 3.4-3.6" /></>,
  };
  return <svg className="icon" viewBox="0 0 24 24" aria-hidden="true">{paths[name]}</svg>;
}
