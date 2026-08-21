import { useEffect, useState } from "react";
import { defaultConfig, defaultRuntime, getConfig, getRuntimeSnapshot, onConfigState, onRuntimeState } from "./api";
import { formatHotkey } from "./keyboard";
import type { AppConfig, RuntimeSnapshot, RuntimeStatus } from "./types";

const statusLabels: Record<RuntimeStatus, string> = {
  ready: "就绪",
  running: "运行中",
  stopping: "正在停止",
  completed: "已完成",
  aborted: "已中止",
  error: "错误",
};

export default function OverlayApp() {
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot>(defaultRuntime);
  const [config, setConfig] = useState<AppConfig>(defaultConfig);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let disposeConfig: (() => void) | undefined;
    getRuntimeSnapshot().then(setSnapshot).catch(() => undefined);
    getConfig().then(setConfig).catch(() => undefined);
    onRuntimeState(setSnapshot).then((unlisten) => { dispose = unlisten; });
    onConfigState(setConfig).then((unlisten) => { disposeConfig = unlisten; });
    return () => {
      dispose?.();
      disposeConfig?.();
    };
  }, []);

  const progress = snapshot.status === "completed" ? 100 : Math.max(0, Math.min(100, ((snapshot.phaseIndex + 0.25) / 7) * 100));

  return (
    <div className={`overlay-shell ${snapshot.status}`}>
      <span className="sr-only" role="status" aria-live="polite">{statusLabels[snapshot.status]}，{snapshot.phaseName}</span>
      <div className="overlay-status">
        <span className={`status-dot ${snapshot.status}`} />
        <div><small>{statusLabels[snapshot.status]}</small><strong>{snapshot.phaseName}</strong></div>
      </div>
      <div className="overlay-progress" aria-label={`阶段 ${snapshot.phaseIndex + 1} / 7`}>
        <div className="overlay-track"><span style={{ width: `${progress}%` }} /></div>
        <span>{String(snapshot.phaseIndex + 1).padStart(2, "0")} / 07</span>
      </div>
      <div className="overlay-hotkeys"><span><kbd>{formatHotkey(config.hotkeys.start)}</kbd> 启动</span><span><kbd>{formatHotkey(config.hotkeys.stop)}</kbd> 停止</span></div>
    </div>
  );
}
