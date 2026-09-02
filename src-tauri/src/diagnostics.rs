use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{config::AppConfig, input::InputProbeResult};

/// 内存环形事件缓冲区容量（runtime-events.jsonl 导出时使用同一份数据）。
pub const RING_CAPACITY: usize = 1024;
/// 单个热键角色保留的事件轨迹上限。
pub const HOTKEY_EVENT_CAPACITY: usize = 64;
/// JSONL 日志文件超过该大小后滚动为 .old（保留最近一份）。
const MAX_LOG_FILE_BYTES: u64 = 4 * 1024 * 1024;

pub const LEVEL_INFO: &str = "info";
pub const LEVEL_WARN: &str = "warn";
pub const LEVEL_ERROR: &str = "error";

/// 结构化诊断事件：同时写入内存环形缓冲区和 JSONL 日志文件。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEvent {
    pub timestamp: String,
    pub level: String,
    pub category: String,
    pub event: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

pub fn iso8601_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let millis = now.subsec_millis();
    let days = secs.div_euclid(86_400);
    let seconds_of_day = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        seconds_of_day / 3_600,
        (seconds_of_day % 3_600) / 60,
        seconds_of_day % 60,
    )
}

/// Howard Hinnant 的 civil-from-days 算法：天数 -> 公历日期。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// 事件中枢：内存环形缓冲 + 可选 JSONL 文件追加。
#[derive(Clone)]
pub struct Hub {
    inner: Arc<HubInner>,
}

struct HubInner {
    events: Mutex<VecDeque<DiagnosticEvent>>,
    log_path: Option<PathBuf>,
    log_file: Mutex<Option<File>>,
    started_at: Instant,
}

impl Hub {
    pub fn new(log_path: Option<PathBuf>) -> Self {
        let log_file = log_path.as_deref().and_then(open_log_file);
        Self {
            inner: Arc::new(HubInner {
                events: Mutex::new(VecDeque::with_capacity(RING_CAPACITY)),
                log_path,
                log_file: Mutex::new(log_file),
                started_at: Instant::now(),
            }),
        }
    }

    /// 仅内存（单元测试 / 无文件系统场景）。
    #[cfg(test)]
    pub fn memory() -> Self {
        Self::new(None)
    }

    pub fn log_path(&self) -> Option<&Path> {
        self.inner.log_path.as_deref()
    }

    pub fn uptime(&self) -> u64 {
        self.inner.started_at.elapsed().as_secs()
    }

    pub fn record(
        &self,
        level: &str,
        category: &str,
        event: &str,
        message: impl Into<String>,
        error: Option<String>,
        details: Option<Value>,
    ) {
        let entry = DiagnosticEvent {
            timestamp: iso8601_now(),
            level: level.to_string(),
            category: category.to_string(),
            event: event.to_string(),
            message: message.into(),
            error,
            details,
        };
        {
            let mut events = self
                .inner
                .events
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if events.len() >= RING_CAPACITY {
                events.pop_front();
            }
            events.push_back(entry.clone());
        }
        if let Some(file) = self
            .inner
            .log_file
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .as_mut()
        {
            if let Ok(line) = serde_json::to_string(&entry) {
                let write = file
                    .write_all(line.as_bytes())
                    .and_then(|_| file.write_all(b"\n"))
                    .and_then(|_| file.flush());
                if write.is_err() {
                    // 日志文件写入失败时保留环形缓冲区，避免后续反复失败。
                    *self
                        .inner
                        .log_file
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner()) = None;
                }
            }
        }
    }

    pub fn info(&self, category: &str, event: &str, message: impl Into<String>) {
        self.record(LEVEL_INFO, category, event, message, None, None);
    }

    pub fn warn(
        &self,
        category: &str,
        event: &str,
        message: impl Into<String>,
        error: Option<String>,
    ) {
        self.record(LEVEL_WARN, category, event, message, error, None);
    }

    pub fn error(
        &self,
        category: &str,
        event: &str,
        message: impl Into<String>,
        error: Option<String>,
    ) {
        self.record(LEVEL_ERROR, category, event, message, error, None);
    }

    /// 按时间正序返回最近 `limit` 条事件。
    pub fn events(&self, limit: usize) -> Vec<DiagnosticEvent> {
        let events = self
            .inner
            .events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let skip = events.len().saturating_sub(limit.max(1));
        events.iter().skip(skip).cloned().collect()
    }

    pub fn all_events(&self) -> Vec<DiagnosticEvent> {
        self.events(RING_CAPACITY)
    }
}

fn open_log_file(path: &Path) -> Option<File> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > MAX_LOG_FILE_BYTES {
            let backup = path.with_extension("old.jsonl");
            let _ = fs::remove_file(&backup);
            let _ = fs::rename(path, &backup);
        }
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// 构建信息：版本 + Git commit SHA + 构建配置。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: String,
    pub commit_sha: String,
    pub commit_short: String,
    pub build_profile: String,
    pub target: String,
    pub built_at: String,
}

pub fn build_info() -> BuildInfo {
    let commit_sha = env!("GIT_COMMIT");
    let commit_short = if commit_sha.len() >= 8 && commit_sha != "unknown" {
        &commit_sha[..8]
    } else {
        commit_sha
    };
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit_sha: commit_sha.to_string(),
        commit_short: commit_short.to_string(),
        build_profile: env!("BUILD_PROFILE").to_string(),
        target: env!("TARGET").to_string(),
        built_at: env!("BUILD_TIMESTAMP").to_string(),
    }
}

/// 路径脱敏：将用户主目录前缀替换为 `<USER>`，避免诊断包泄露用户名。
pub fn sanitize_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from));
    if let Some(profile) = profile {
        let prefix = profile.to_string_lossy();
        if text.starts_with(prefix.as_ref()) {
            return format!("<USER>{}", &text[prefix.len()..]);
        }
    }
    text.into_owned()
}

/// 单个全局热键的完整生命周期轨迹（解析 / 注册 / 失败 / 回滚 / is_registered）。
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyEventEntry {
    pub timestamp: String,
    /// parse | parse.failed | register | register.failed | unregister |
    /// unregister.failed | rollback | rollback.failed | restore | restore.failed |
    /// is_registered
    pub action: String,
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatusEntry {
    /// start | stop
    pub role: String,
    pub label: String,
    pub configured: String,
    pub parsed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_error: Option<String>,
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub register_error: Option<String>,
    /// 最近一次观察到的 manager.is_registered 状态。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_registered: Option<bool>,
    pub events: Vec<HotkeyEventEntry>,
    pub updated_at: String,
}

pub struct HotkeyTracker {
    entries: Mutex<Vec<HotkeyStatusEntry>>,
    hub: Hub,
}

impl HotkeyTracker {
    pub fn new(hub: Hub) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            hub,
        }
    }

    fn entry(&self, role: &str, label: &str, configured: &str) -> HotkeyStatusEntry {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(entry) = entries.iter_mut().find(|entry| entry.role == role) {
            if entry.label != label || entry.configured != configured {
                // 配置发生变化时保留轨迹，但重置当前状态并同步新配置。
                entry.label = label.to_string();
                entry.configured = configured.to_string();
                entry.parsed = false;
                entry.parsed_error = None;
                entry.registered = false;
                entry.register_error = None;
            }
            return entry.clone();
        }
        let entry = HotkeyStatusEntry {
            role: role.to_string(),
            label: label.to_string(),
            configured: configured.to_string(),
            parsed: false,
            parsed_error: None,
            registered: false,
            register_error: None,
            is_registered: None,
            events: Vec::new(),
            updated_at: iso8601_now(),
        };
        entries.push(entry.clone());
        entry
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        action: &str,
        ok: bool,
        message: String,
        error: Option<String>,
        shortcut: Option<String>,
    ) {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let entry = match entries.iter_mut().find(|entry| entry.role == role) {
            Some(entry) => entry,
            None => {
                entries.push(HotkeyStatusEntry {
                    role: role.to_string(),
                    label: label.to_string(),
                    configured: configured.to_string(),
                    parsed: false,
                    parsed_error: None,
                    registered: false,
                    register_error: None,
                    is_registered: None,
                    events: Vec::new(),
                    updated_at: iso8601_now(),
                });
                entries.last_mut().expect("entry was just pushed")
            }
        };
        if entry.events.len() >= HOTKEY_EVENT_CAPACITY {
            entry.events.remove(0);
        }
        entry.events.push(HotkeyEventEntry {
            timestamp: iso8601_now(),
            action: action.to_string(),
            ok,
            message: message.clone(),
            error: error.clone(),
            shortcut: shortcut.clone(),
        });
        match action {
            "parse" | "parse.failed" => {
                entry.parsed = ok;
                entry.parsed_error = if ok { None } else { error.clone() };
            }
            "register" | "register.failed" | "restore" | "restore.failed" => {
                entry.registered = ok;
                entry.register_error = if ok { None } else { error.clone() };
            }
            "unregister" if ok => entry.registered = false,
            "is_registered" => entry.is_registered = Some(ok),
            _ => {}
        }
        entry.updated_at = iso8601_now();
        let level = if ok { LEVEL_INFO } else { LEVEL_ERROR };
        self.hub.record(
            level,
            "hotkey",
            &format!("hotkey.{action}"),
            format!("[{label}] {message}"),
            error,
            Some(json!({ "role": role, "configured": configured, "shortcut": shortcut })),
        );
    }

    pub fn record_parse(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        result: Result<String, String>,
    ) {
        self.entry(role, label, configured);
        match result {
            Ok(shortcut) => self.push(
                role,
                label,
                configured,
                "parse",
                true,
                format!("解析成功：{shortcut}"),
                None,
                Some(shortcut),
            ),
            Err(error) => self.push(
                role,
                label,
                configured,
                "parse.failed",
                false,
                format!("解析失败：{configured}"),
                Some(error),
                None,
            ),
        }
    }

    pub fn record_register(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        shortcut: &str,
        result: Result<(), String>,
    ) {
        match result {
            Ok(()) => self.push(
                role,
                label,
                configured,
                "register",
                true,
                format!("注册成功：{shortcut}"),
                None,
                Some(shortcut.to_string()),
            ),
            Err(error) => self.push(
                role,
                label,
                configured,
                "register.failed",
                false,
                format!("注册失败：{shortcut}"),
                Some(error),
                Some(shortcut.to_string()),
            ),
        }
    }

    pub fn record_unregister(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        shortcut: &str,
        result: Result<(), String>,
    ) {
        match result {
            Ok(()) => self.push(
                role,
                label,
                configured,
                "unregister",
                true,
                format!("注销成功：{shortcut}"),
                None,
                Some(shortcut.to_string()),
            ),
            Err(error) => self.push(
                role,
                label,
                configured,
                "unregister.failed",
                false,
                format!("注销失败：{shortcut}"),
                Some(error),
                Some(shortcut.to_string()),
            ),
        }
    }

    pub fn record_rollback(&self, role: &str, label: &str, configured: &str, message: String) {
        self.push(
            role, label, configured, "rollback", true, message, None, None,
        );
    }

    pub fn record_rollback_failed(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        message: String,
        error: String,
    ) {
        self.push(
            role,
            label,
            configured,
            "rollback.failed",
            false,
            message,
            Some(error),
            None,
        );
    }

    pub fn record_restore(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        shortcut: &str,
        result: Result<(), String>,
    ) {
        match result {
            Ok(()) => self.push(
                role,
                label,
                configured,
                "restore",
                true,
                format!("回滚后重新注册成功：{shortcut}"),
                None,
                Some(shortcut.to_string()),
            ),
            Err(error) => self.push(
                role,
                label,
                configured,
                "restore.failed",
                false,
                format!("回滚后重新注册失败：{shortcut}"),
                Some(error),
                Some(shortcut.to_string()),
            ),
        }
    }

    pub fn record_is_registered(
        &self,
        role: &str,
        label: &str,
        configured: &str,
        registered: bool,
    ) {
        self.push(
            role,
            label,
            configured,
            "is_registered",
            true,
            format!(
                "is_registered 检查：{}",
                if registered { "已注册" } else { "未注册" }
            ),
            None,
            None,
        );
        if !registered {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if let Some(entry) = entries.iter_mut().find(|entry| entry.role == role) {
                entry.registered = false;
            }
        }
    }

    pub fn report(&self) -> Vec<HotkeyStatusEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

/// 环境信息（路径均脱敏）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    pub arch: String,
    pub app_version: String,
    pub commit_sha: String,
    pub build_profile: String,
    pub config_path: String,
    pub log_path: String,
    pub downloads_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground_process: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground_integrity: Option<String>,
    pub session_uptime_s: u64,
    pub generated_at: String,
}

/// 诊断导出包内容（写 ZIP 前先在内存中组装，便于单元测试）。
pub struct ExportBundle {
    pub summary: String,
    pub system: Value,
    pub build: Value,
    pub hotkeys: Value,
    pub probes: Value,
    pub events: Vec<DiagnosticEvent>,
    pub config_sanitized: Value,
}

impl ExportBundle {
    pub fn files(&self) -> Vec<(&'static str, String)> {
        vec![
            ("summary.txt", self.summary.clone()),
            (
                "system.json",
                serde_json::to_string_pretty(&self.system).unwrap_or_default(),
            ),
            (
                "build.json",
                serde_json::to_string_pretty(&self.build).unwrap_or_default(),
            ),
            (
                "hotkeys.json",
                serde_json::to_string_pretty(&self.hotkeys).unwrap_or_default(),
            ),
            (
                "input-probes.json",
                serde_json::to_string_pretty(&self.probes).unwrap_or_default(),
            ),
            ("runtime-events.jsonl", events_to_jsonl(&self.events)),
            (
                "config-sanitized.json",
                serde_json::to_string_pretty(&self.config_sanitized).unwrap_or_default(),
            ),
        ]
    }
}

fn events_to_jsonl(events: &[DiagnosticEvent]) -> String {
    let mut output = String::new();
    for event in events {
        if let Ok(line) = serde_json::to_string(event) {
            output.push_str(&line);
            output.push('\n');
        }
    }
    output
}

fn hotkey_summary_lines(hotkeys: &[HotkeyStatusEntry]) -> Vec<String> {
    let mut lines = Vec::new();
    for entry in hotkeys {
        let mut error_lines = Vec::new();
        if let Some(error) = &entry.parsed_error {
            error_lines.push(format!("解析原始错误：{error}"));
        }
        if let Some(error) = &entry.register_error {
            error_lines.push(format!("注册原始错误：{error}"));
        }
        for event in entry.events.iter().rev().take(8).rev() {
            if let Some(error) = &event.error {
                error_lines.push(format!(
                    "[{action}] 原始错误：{error}",
                    action = event.action
                ));
            }
        }
        let error_text = if error_lines.is_empty() {
            "（无）".to_string()
        } else {
            error_lines.join("\n    ")
        };
        lines.push(format!(
            "{label}（{configured}）: 解析={parsed} 注册={registered} is_registered={is_registered:?}\n    原始错误: {error_text}",
            label = entry.label,
            configured = entry.configured,
            parsed = if entry.parsed { "成功" } else { "失败" },
            registered = if entry.registered { "成功" } else { "失败" },
            is_registered = entry.is_registered,
            error_text = error_text,
        ));
    }
    lines
}

pub fn assemble_export_bundle(
    build: &BuildInfo,
    environment: &EnvironmentInfo,
    hotkeys: &[HotkeyStatusEntry],
    probes: &[InputProbeResult],
    events: &[DiagnosticEvent],
    config: &AppConfig,
) -> ExportBundle {
    let mut raw_errors: Vec<String> = Vec::new();
    for entry in hotkeys {
        if let Some(error) = &entry.parsed_error {
            raw_errors.push(format!("[热键 {label}] 解析：{error}", label = entry.label));
        }
        if let Some(error) = &entry.register_error {
            raw_errors.push(format!("[热键 {label}] 注册：{error}", label = entry.label));
        }
    }
    for probe in probes {
        if let Some(error) = &probe.last_error {
            raw_errors.push(format!("[输入探针 {}] {error}", probe.label));
        }
    }
    for event in events.iter().rev() {
        if event.level == LEVEL_ERROR {
            if let Some(error) = &event.error {
                raw_errors.push(format!("[{}] {}", event.event, error));
            } else {
                raw_errors.push(format!("[{}] {}", event.event, event.message));
            }
        }
    }
    let raw_errors_text = if raw_errors.is_empty() {
        "（无）".to_string()
    } else {
        raw_errors
            .into_iter()
            .rev()
            .map(|error| format!("- {error}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let summary = format!(
        "D2 Morgeth Kick 诊断报告\n\
         ============================\n\
         生成时间: {generated}\n\
         版本: v{version} (commit {commit}, {profile})\n\n\
         [后端]\n\
         原生后端: 可用（本报告由原生后端生成）\n\
         系统: {os} {os_version} ({arch})\n\n\
         [热键]\n\
         {hotkeys}\n\n\
         [输入探针]\n\
         {probes}\n\n\
         [环境]\n\
         配置目录: {config_path}\n\
         日志文件: {log_path}\n\
         下载目录: {downloads_path}\n\
         前台进程: {foreground_process}\n\
         完整性级别: {foreground_integrity}\n\
         会话时长: {uptime} 秒\n\n\
         [原始错误]\n\
         {raw_errors}\n\n\
         [运行事件]（最近 {event_count} 条，完整记录见 runtime-events.jsonl）\n\
         {events}\n",
        generated = environment.generated_at,
        version = build.version,
        commit = build.commit_sha,
        profile = build.build_profile,
        os = environment.os,
        os_version = environment.os_version.as_deref().unwrap_or(""),
        arch = environment.arch,
        hotkeys = if hotkeys.is_empty() {
            "（无热键状态记录）".to_string()
        } else {
            hotkey_summary_lines(hotkeys).join("\n")
        },
        probes = if probes.is_empty() {
            "（尚未运行自检，点击“重新检测”后再导出）".to_string()
        } else {
            probes
                .iter()
                .map(|probe| {
                    format!(
                        "{label}: {ok}（请求 {requested} 次 / 返回 {sent} 次，LastError: {last_error}，前台进程: {foreground}，完整性: {integrity}，耗时 {duration_ms} ms）",
                        label = probe.label,
                        ok = if probe.ok { "成功" } else { "失败" },
                        requested = probe.requested,
                        sent = probe.sent,
                        last_error = probe.last_error.as_deref().unwrap_or("无"),
                        foreground = probe.foreground_process.as_deref().unwrap_or("未知"),
                        integrity = probe.integrity_level.as_deref().unwrap_or("未知"),
                        duration_ms = probe.duration_ms,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        },
        config_path = environment.config_path,
        log_path = environment.log_path,
        downloads_path = environment.downloads_path,
        foreground_process = environment.foreground_process.as_deref().unwrap_or("未知"),
        foreground_integrity = environment
            .foreground_integrity
            .as_deref()
            .unwrap_or("未知"),
        uptime = environment.session_uptime_s,
        raw_errors = raw_errors_text,
        event_count = events.len(),
        events = events
            .iter()
            .rev()
            .take(20)
            .map(|event| {
                let error = event
                    .error
                    .as_ref()
                    .map(|error| format!(" error=\"{error}\""))
                    .unwrap_or_default();
                format!(
                    "{ts} [{level}] {category}/{name} {message}{error}",
                    ts = event.timestamp,
                    level = event.level,
                    category = event.category,
                    name = event.event,
                    message = event.message,
                    error = error,
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );

    ExportBundle {
        summary,
        system: serde_json::to_value(environment).unwrap_or(Value::Null),
        build: serde_json::to_value(build).unwrap_or(Value::Null),
        hotkeys: serde_json::to_value(hotkeys).unwrap_or(Value::Null),
        probes: serde_json::to_value(probes).unwrap_or(Value::Null),
        events: events.to_vec(),
        config_sanitized: json!({
            "sanitized": true,
            "note": "校准参数与按键绑定不包含个人信息，因此整体保留；日志与路径中的用户目录前缀已脱敏为 <USER>。",
            "redactedFields": [],
            "config": config,
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsExportResult {
    pub path: String,
    pub file_count: usize,
    pub size_bytes: u64,
    pub exported_at: String,
}

pub fn zip_file_name(build: &BuildInfo) -> String {
    let stamp = iso8601_now()
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    format!(
        "d2-morgeth-kick-diagnostics-v{}-{}.zip",
        build.version,
        &stamp[..stamp.len().min(14)]
    )
}

/// 把诊断包写入 `target_dir`（下载目录），返回导出结果。
pub fn write_export_zip(
    target_dir: &Path,
    build: &BuildInfo,
    bundle: &ExportBundle,
) -> Result<DiagnosticsExportResult, String> {
    fs::create_dir_all(target_dir).map_err(|error| format!("无法创建下载目录：{error}"))?;
    let path = target_dir.join(zip_file_name(build));
    let file = File::create(&path).map_err(|error| format!("无法创建诊断包：{error}"))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let files = bundle.files();
    for (name, content) in &files {
        writer
            .start_file(*name, options)
            .map_err(|error| format!("无法写入诊断包条目 {name}：{error}"))?;
        writer
            .write_all(content.as_bytes())
            .map_err(|error| format!("无法写入诊断包条目 {name}：{error}"))?;
    }
    writer
        .finish()
        .map_err(|error| format!("无法完成诊断包：{error}"))?;
    let size_bytes = fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Ok(DiagnosticsExportResult {
        path: path.to_string_lossy().into_owned(),
        file_count: files.len(),
        size_bytes,
        exported_at: iso8601_now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputProbeResult;

    fn probe(probe: &str, ok: bool, last_error: Option<&str>) -> InputProbeResult {
        InputProbeResult {
            probe: probe.to_string(),
            label: probe.to_string(),
            description: "test probe".into(),
            ok,
            requested: 2,
            sent: if ok { 2 } else { 1 },
            calls: 2,
            last_error_code: if ok { None } else { Some(5) },
            last_error: last_error.map(str::to_string),
            foreground_process: Some("destiny2.exe".into()),
            integrity_level: Some("Medium (0x2000)".into()),
            observed_async_down: None,
            duration_ms: 3,
            timestamp: "2026-01-01T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(20_675), (2026, 8, 10));
    }

    #[test]
    fn ring_buffer_evicts_oldest_entries_beyond_capacity() {
        let hub = Hub::memory();
        for index in 0..(RING_CAPACITY + 50) {
            hub.info("test", "ring.push", format!("entry-{index}"));
        }
        let events = hub.all_events();
        assert_eq!(events.len(), RING_CAPACITY);
        assert_eq!(events[0].message, "entry-50");
        assert_eq!(
            events.last().unwrap().message,
            format!("entry-{}", RING_CAPACITY + 49)
        );
    }

    #[test]
    fn events_are_returned_in_chronological_order() {
        let hub = Hub::memory();
        hub.info("test", "a", "first");
        hub.warn("test", "b", "second", Some("oops".into()));
        let events = hub.events(10);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event, "a");
        assert_eq!(events[1].level, LEVEL_WARN);
        assert_eq!(events[1].error.as_deref(), Some("oops"));
    }

    #[test]
    fn jsonl_events_serialize_with_camel_case_fields() {
        let hub = Hub::memory();
        hub.error(
            "hotkey",
            "hotkey.register.failed",
            "注册失败：F8",
            Some("被其他程序占用".into()),
        );
        let line = serde_json::to_string(&hub.all_events()[0]).unwrap();
        assert!(line.contains("\"event\":\"hotkey.register.failed\""));
        assert!(line.contains("\"error\":\"被其他程序占用\""));
        assert!(line.contains("\"level\":\"error\""));
    }

    #[test]
    fn hotkey_tracker_records_full_lifecycle_per_role() {
        let tracker = HotkeyTracker::new(Hub::memory());
        tracker.record_parse("start", "启动热键", "F8", Ok("F8".into()));
        tracker.record_parse("stop", "停止热键", "F10", Ok("F10".into()));
        tracker.record_register("start", "启动热键", "F8", "F8", Ok(()));
        tracker.record_register(
            "stop",
            "停止热键",
            "F10",
            "F10",
            Err("HotKey already registered".into()),
        );
        tracker.record_is_registered("start", "启动热键", "F8", true);
        tracker.record_rollback("stop", "停止热键", "F10", "回滚：重新注册 F10".into());

        let report = tracker.report();
        assert_eq!(report.len(), 2);
        let start = report.iter().find(|entry| entry.role == "start").unwrap();
        let stop = report.iter().find(|entry| entry.role == "stop").unwrap();
        assert!(start.parsed && start.registered && start.is_registered == Some(true));
        assert_eq!(start.register_error, None);
        assert!(!stop.registered);
        assert_eq!(
            stop.register_error.as_deref(),
            Some("HotKey already registered")
        );
        assert_eq!(
            stop.events
                .iter()
                .map(|event| event.action.as_str())
                .collect::<Vec<_>>(),
            ["parse", "register.failed", "rollback"]
        );
    }

    #[test]
    fn parse_failure_keeps_the_raw_error() {
        let tracker = HotkeyTracker::new(Hub::memory());
        tracker.record_parse(
            "start",
            "启动热键",
            "NotAKey",
            Err("热键解析失败：unknown key".into()),
        );
        let entry = &tracker.report()[0];
        assert!(!entry.parsed);
        assert_eq!(
            entry.parsed_error.as_deref(),
            Some("热键解析失败：unknown key")
        );
    }

    #[test]
    fn hotkey_events_also_reach_the_hub_ring_buffer() {
        let hub = Hub::memory();
        let tracker = HotkeyTracker::new(hub.clone());
        tracker.record_register("stop", "停止热键", "F10", "F10", Err("冲突".into()));
        let events = hub.all_events();
        assert!(events
            .iter()
            .any(|event| event.event == "hotkey.register.failed"));
        assert!(events.iter().any(|event| event.category == "hotkey"));
    }

    #[test]
    fn sanitize_path_redacts_the_user_profile_prefix() {
        let profile = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .expect("user profile env var");
        let path = PathBuf::from(&profile).join("Downloads").join("report.zip");
        let sanitized = sanitize_path(&path);
        assert!(sanitized.starts_with("<USER>"));
        assert!(sanitized.ends_with("report.zip"));
        assert!(!sanitized.contains(&profile.to_string_lossy().to_string()));
    }

    #[test]
    fn export_zip_contains_all_required_files() {
        let build = BuildInfo {
            version: "0.3.5".into(),
            commit_sha: "0123456789abcdef".into(),
            commit_short: "01234567".into(),
            build_profile: "debug".into(),
            target: "x86_64-pc-windows-msvc".into(),
            built_at: "0".into(),
        };
        let environment = EnvironmentInfo {
            os: "windows".into(),
            os_version: Some("10.0 (build 19045)".into()),
            arch: "x86_64".into(),
            app_version: "0.3.5".into(),
            commit_sha: "0123456789abcdef".into(),
            build_profile: "debug".into(),
            config_path: "<USER>/AppData/Roaming/settings.json".into(),
            log_path: "<USER>/AppData/Roaming/logs/diagnostics.jsonl".into(),
            downloads_path: "<USER>/Downloads".into(),
            foreground_process: Some("destiny2.exe".into()),
            foreground_integrity: Some("Medium (0x2000)".into()),
            session_uptime_s: 12,
            generated_at: "2026-01-01T00:00:00.000Z".into(),
        };
        let tracker = HotkeyTracker::new(Hub::memory());
        tracker.record_parse("start", "启动热键", "F8", Ok("F8".into()));
        tracker.record_register("start", "启动热键", "F8", "F8", Ok(()));
        tracker.record_register("stop", "停止热键", "F10", "F10", Err("已注册".into()));
        let hub = Hub::memory();
        hub.error(
            "input",
            "probe.failed",
            "SendInput 失败",
            Some("拒绝访问 (5)".into()),
        );
        let bundle = assemble_export_bundle(
            &build,
            &environment,
            &tracker.report(),
            &[
                probe("scan-code-w", true, None),
                probe("mouse-relative", false, Some("拒绝访问 (5)")),
            ],
            &hub.all_events(),
            &AppConfig::default(),
        );

        let target = std::env::temp_dir().join(format!("d2mk-diag-test-{}", std::process::id()));
        fs::create_dir_all(&target).unwrap();
        let result = write_export_zip(&target, &build, &bundle).unwrap();
        assert_eq!(result.file_count, 7);

        let archive = File::open(&result.path).unwrap();
        let mut zip = zip::ZipArchive::new(archive).unwrap();
        let expected = [
            "summary.txt",
            "system.json",
            "build.json",
            "hotkeys.json",
            "input-probes.json",
            "runtime-events.jsonl",
            "config-sanitized.json",
        ];
        for name in expected {
            assert!(zip.by_name(name).is_ok(), "缺少条目 {name}");
        }
        let summary = {
            let mut entry = zip.by_name("summary.txt").unwrap();
            let mut text = String::new();
            std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
            text
        };
        assert!(summary.contains("D2 Morgeth Kick 诊断报告"));
        assert!(summary.contains("拒绝访问 (5)"));
        assert!(summary.contains("0123456789abcdef"));
        let config = {
            let mut entry = zip.by_name("config-sanitized.json").unwrap();
            let mut text = String::new();
            std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
            text
        };
        assert!(config.contains("\"sanitized\": true"));
        assert!(config.contains("\"start\": \"F8\""));
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn build_info_embeds_version_and_commit() {
        let info = build_info();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.version, "0.3.7");
        assert!(!info.commit_sha.is_empty());
        let expected_short = if info.commit_sha == "unknown" {
            "unknown"
        } else {
            &info.commit_sha[..8]
        };
        assert_eq!(info.commit_short, expected_short);
    }
}
