import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import {
  errorText,
  exportDiagnosticsPackage,
  getDiagnosticEvents,
  getEnvironmentInfo,
  getHotkeyStatus,
  nativePing,
  runInputProbes,
} from "./api";
import { Icon } from "./icon";
import type {
  BackendHandshakeState,
  DiagnosticEvent,
  DiagnosticsExportResult,
  EnvironmentInfo,
  HotkeyStatusEntry,
  InputProbeResult,
} from "./types";

interface DiagnosticsPanelProps {
  backend: BackendHandshakeState;
  lastRuntimeError: string | null;
  onBackendState: (state: BackendHandshakeState) => void;
  onClose: () => void;
}

const statusLabels: Record<BackendHandshakeState["status"], string> = {
  checking: "握手中",
  ok: "可用",
  unavailable: "不可用",
};

function formatLocalTime(timestamp: string | null | undefined): string {
  if (!timestamp) return "—";
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return timestamp;
  return date.toLocaleTimeString("zh-CN", { hour12: false });
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function Badge({ ok, text }: { ok: boolean; text?: string }) {
  return <span className={`diag-badge ${ok ? "ok" : "bad"}`}>{text ?? (ok ? "正常" : "异常")}</span>;
}

function Row({ label, children, mono = false }: { label: string; children: ReactNode; mono?: boolean }) {
  return (
    <div className="diag-row">
      <span className="diag-row-label">{label}</span>
      <span className={`diag-row-value${mono ? " mono" : ""}`}>{children}</span>
    </div>
  );
}

function ErrorBlock({ error }: { error: string | null | undefined }) {
  if (!error) return null;
  return <pre className="diag-error-block">{error}</pre>;
}

function HotkeyCard({ entry }: { entry: HotkeyStatusEntry }) {
  const [expanded, setExpanded] = useState(false);
  const recent = entry.events.slice(-6).reverse();
  return (
    <article className="diag-card">
      <header>
        <div>
          <strong>{entry.label}</strong>
          <kbd>{entry.configured}</kbd>
        </div>
        <Badge ok={entry.parsed && entry.registered && entry.isRegistered !== false} />
      </header>
      <Row label="解析">{entry.parsed ? "成功" : "失败"}</Row>
      <ErrorBlock error={entry.parsedError} />
      <Row label="注册">{entry.registered ? "成功" : "失败"}</Row>
      <ErrorBlock error={entry.registerError} />
      <Row label="is_registered">{entry.isRegistered === undefined ? "未检查" : entry.isRegistered ? "true" : "false"}</Row>
      {recent.length > 0 && (
        <>
          <button className="text-button diag-events-toggle" type="button" onClick={() => setExpanded((current) => !current)} aria-expanded={expanded}>
            {expanded ? "收起事件轨迹" : `查看事件轨迹（最近 ${recent.length} 条）`}
          </button>
          {expanded && (
            <ul className="diag-event-list">
              {recent.map((event, index) => (
                <li key={`${event.timestamp}-${index}`} className={event.ok ? "" : "failed"}>
                  <time>{formatLocalTime(event.timestamp)}</time>
                  <code>{event.action}</code>
                  <span>{event.message}</span>
                  {event.error && <pre>{event.error}</pre>}
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </article>
  );
}

function ProbeCard({ probe }: { probe: InputProbeResult }) {
  return (
    <article className="diag-card">
      <header>
        <div>
          <strong>{probe.label}</strong>
          <small>{probe.description}</small>
        </div>
        <Badge ok={probe.ok} text={probe.ok ? "成功" : "失败"} />
      </header>
      <Row label="SendInput 请求数" mono>{probe.requested}</Row>
      <Row label="SendInput 返回数" mono>{probe.sent}（调用 {probe.calls} 次）</Row>
      <Row label="LastError" mono>{probe.lastError ?? "无错误"}</Row>
      <ErrorBlock error={probe.lastError} />
      <Row label="前台进程" mono>{probe.foregroundProcess ?? "未知"}</Row>
      <Row label="完整性级别" mono>{probe.integrityLevel ?? "未知"}</Row>
      {probe.observedAsyncDown !== undefined && probe.observedAsyncDown !== null && (
        <Row label="观察到的按下状态">{probe.observedAsyncDown ? "已按下" : "未观察到"}</Row>
      )}
      <Row label="耗时" mono>{probe.durationMs} ms · {formatLocalTime(probe.timestamp)}</Row>
    </article>
  );
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const area = document.createElement("textarea");
      area.value = text;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      const copied = document.execCommand("copy");
      area.remove();
      return copied;
    } catch {
      return false;
    }
  }
}

export default function DiagnosticsPanel({ backend, lastRuntimeError, onBackendState, onClose }: DiagnosticsPanelProps) {
  const [hotkeys, setHotkeys] = useState<HotkeyStatusEntry[] | null>(null);
  const [probes, setProbes] = useState<InputProbeResult[] | null>(null);
  const [environment, setEnvironment] = useState<EnvironmentInfo | null>(null);
  const [events, setEvents] = useState<DiagnosticEvent[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [exportResult, setExportResult] = useState<DiagnosticsExportResult | null>(null);
  const [copyState, setCopyState] = useState<"idle" | "done" | "failed">("idle");
  const [showEvents, setShowEvents] = useState(false);
  const mountedRef = useRef(false);

  const detect = useCallback(async (withProbes: boolean) => {
    if (busy) return;
    setBusy(true);
    setFeedback(null);
    setExportResult(null);
    const started = performance.now();
    try {
      const ping = await nativePing();
      onBackendState({
        status: "ok",
        ping,
        latencyMs: Math.round(performance.now() - started),
      });
      const [nextHotkeys, nextProbes, nextEnvironment, nextEvents] = await Promise.all([
        getHotkeyStatus(),
        withProbes ? runInputProbes() : Promise.resolve(probes ?? []),
        getEnvironmentInfo(),
        getDiagnosticEvents(300),
      ]);
      setHotkeys(nextHotkeys);
      if (withProbes) setProbes(nextProbes);
      setEnvironment(nextEnvironment);
      setEvents(nextEvents);
    } catch (reason) {
      const message = errorText(reason);
      setFeedback(message);
      onBackendState({ status: "unavailable", message });
    } finally {
      setBusy(false);
    }
    // 仅首次挂载与显式点击触发；probes 保持上次结果作为回退值。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (mountedRef.current) return;
    mountedRef.current = true;
    void detect(false);
  }, [detect]);

  const collectErrors = (): string => {
    const lines: string[] = [];
    if (backend.status === "unavailable" && backend.message) {
      lines.push(`[后端] ${backend.message}`);
    }
    for (const entry of hotkeys ?? []) {
      if (entry.parsedError) lines.push(`[${entry.label} 解析] ${entry.parsedError}`);
      if (entry.registerError) lines.push(`[${entry.label} 注册] ${entry.registerError}`);
      for (const event of entry.events) {
        if (event.error) lines.push(`[${entry.label} ${event.action}] ${event.error}`);
      }
    }
    for (const probe of probes ?? []) {
      if (probe.lastError) lines.push(`[输入探针 ${probe.label}] ${probe.lastError}`);
    }
    if (lastRuntimeError) lines.push(`[运行时] ${lastRuntimeError}`);
    for (const event of events ?? []) {
      if (event.level === "error") {
        lines.push(`[事件 ${event.event}] ${event.error ?? event.message}`);
      }
    }
    return lines.length > 0 ? lines.join("\n") : "未发现错误记录。";
  };

  const copyErrors = async () => {
    const copied = await copyText(collectErrors());
    setCopyState(copied ? "done" : "failed");
    window.setTimeout(() => setCopyState("idle"), 2500);
  };

  const exportPackage = async () => {
    if (busy) return;
    setBusy(true);
    setFeedback(null);
    setExportResult(null);
    try {
      const nextProbes = await runInputProbes();
      setProbes(nextProbes);
      setExportResult(await exportDiagnosticsPackage());
    } catch (reason) {
      setFeedback(errorText(reason));
    } finally {
      setBusy(false);
    }
  };

  const backendLabel = statusLabels[backend.status];
  const recentEvents = events ? events.slice(-30).reverse() : [];

  return (
    <div className="diag-shell">
      <div className="diag-body">
        {feedback && <div className="diag-feedback" role="alert"><strong>操作未完成</strong><span>{feedback}</span></div>}

        <section className="diag-group" aria-labelledby="diag-backend-title">
          <header className="diag-group-heading">
            <h3 id="diag-backend-title">后端</h3>
            <Badge ok={backend.status === "ok"} text={backendLabel} />
          </header>
          <Row label="握手状态">{backend.status === "ok" ? "native_ping 成功" : backend.status === "checking" ? "正在握手…" : "native_ping 失败"}</Row>
          {backend.status === "ok" && backend.ping && (
            <>
              <Row label="版本" mono>v{backend.ping.version}{backend.ping.mock ? "（开发预览）" : ""}</Row>
              <Row label="Commit SHA" mono>{backend.ping.commitSha}</Row>
              <Row label="构建配置" mono>{backend.ping.buildProfile}</Row>
              <Row label="平台" mono>{backend.ping.os} / {backend.ping.arch}{backend.ping.pid ? ` / PID ${backend.ping.pid}` : ""}</Row>
              <Row label="握手延迟" mono>{backend.latencyMs ?? "—"} ms</Row>
            </>
          )}
          {backend.status === "unavailable" && <ErrorBlock error={backend.message} />}
          {backend.status === "unavailable" && (
            <p className="diag-note">正式构建已禁止前端 mock：后端握手失败时启动与停止按钮会被禁用，避免在无后端环境下执行模拟动作。</p>
          )}
        </section>

        <section className="diag-group" aria-labelledby="diag-hotkeys-title">
          <header className="diag-group-heading">
            <h3 id="diag-hotkeys-title">热键</h3>
            {hotkeys && <span className="diag-group-meta">解析 / 注册 / 失败 / 回滚 / is_registered 全轨迹</span>}
          </header>
          <div className="diag-card-grid">
            {(hotkeys ?? []).map((entry) => <HotkeyCard key={entry.role} entry={entry} />)}
            {!hotkeys && <p className="diag-note">正在读取热键状态…</p>}
          </div>
        </section>

        <section className="diag-group" aria-labelledby="diag-input-title">
          <header className="diag-group-heading">
            <h3 id="diag-input-title">输入</h3>
            {probes && <span className="diag-group-meta">SendInput 主动探针（扫描码 W / 虚拟键 W / 相对鼠标）</span>}
          </header>
          <p className="diag-note">
            探针会向前台窗口注入一次极短的 W 按键与 ±1 像素的相对鼠标移动（净位移为零），并记录 SendInput
            请求数、返回数、LastError、前台进程与完整性级别。仅在你点击“重新检测”或“导出诊断包”时运行。
          </p>
          <div className="diag-card-grid three">
            {(probes ?? []).map((probe) => <ProbeCard key={probe.probe} probe={probe} />)}
            {!probes && <p className="diag-note">尚未运行输入自检，点击“重新检测”。</p>}
          </div>
        </section>

        <section className="diag-group" aria-labelledby="diag-env-title">
          <header className="diag-group-heading">
            <h3 id="diag-env-title">环境</h3>
          </header>
          {environment && (
            <>
              <Row label="操作系统" mono>{environment.os}{environment.osVersion ? ` ${environment.osVersion}` : ""}（{environment.arch}）</Row>
              <Row label="应用版本" mono>v{environment.appVersion} · commit {environment.commitSha} · {environment.buildProfile}</Row>
              <Row label="配置目录" mono>{environment.configPath}</Row>
              <Row label="日志文件" mono>{environment.logPath}</Row>
              <Row label="下载目录" mono>{environment.downloadsPath}</Row>
              <Row label="前台进程" mono>{environment.foregroundProcess ?? "未知"}</Row>
              <Row label="前台完整性级别" mono>{environment.foregroundIntegrity ?? "未知"}</Row>
              <Row label="会话时长" mono>{environment.sessionUptimeS} 秒</Row>
            </>
          )}
          {!environment && <p className="diag-note">正在读取环境信息…</p>}
          <div className="diag-runtime-error">
            <strong>最近运行时错误</strong>
            {lastRuntimeError
              ? <pre className="diag-error-block">{lastRuntimeError}</pre>
              : <span className="diag-row-value">无</span>}
          </div>
          {recentEvents.length > 0 && (
            <>
              <button className="text-button diag-events-toggle" type="button" onClick={() => setShowEvents((current) => !current)} aria-expanded={showEvents}>
                {showEvents ? "收起运行事件" : `查看运行事件（最近 ${recentEvents.length} 条）`}
              </button>
              {showEvents && (
                <ul className="diag-event-list">
                  {recentEvents.map((event, index) => (
                    <li key={`${event.timestamp}-${index}`} className={event.level === "error" ? "failed" : ""}>
                      <time>{formatLocalTime(event.timestamp)}</time>
                      <code>{event.level}</code>
                      <code>{event.category}/{event.event}</code>
                      <span>{event.message}</span>
                      {event.error && <pre>{event.error}</pre>}
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </section>
      </div>

      <div className="dialog-actions diag-actions">
        {exportResult && (
          <span className="diag-export-result">
            已导出 {exportResult.fileCount} 个文件（{formatBytes(exportResult.sizeBytes)}）：{exportResult.path}
          </span>
        )}
        <button className="button secondary" type="button" onClick={() => void detect(true)} disabled={busy}>
          <Icon name="pulse" />{busy ? "检测中…" : "重新检测"}
        </button>
        <button className="button secondary" type="button" onClick={() => void copyErrors()}>
          <Icon name="copy" />{copyState === "done" ? "已复制" : copyState === "failed" ? "复制失败" : "复制错误"}
        </button>
        <button className="button secondary" type="button" onClick={() => void exportPackage()} disabled={busy}>
          <Icon name="download" />{busy ? "导出中…" : "导出诊断包"}
        </button>
        <button className="button primary" type="button" onClick={onClose}>完成</button>
      </div>
    </div>
  );
}
