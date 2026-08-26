mod config;
mod diagnostics;
mod engine;
mod input;
mod resolution;
mod runtime;
mod system;

use std::sync::{atomic::Ordering, Arc};
use tauri::{Emitter, Manager, PhysicalPosition, Position, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use config::{AppConfig, HotkeyBinding, HotkeyConfig, HotkeyRole, MouseShortcut};
use diagnostics::{DiagnosticEvent, DiagnosticsExportResult, EnvironmentInfo, HotkeyStatusEntry};
use input::InputProbeResult;
use runtime::{AppState, RuntimeSnapshot, RuntimeStatus};

const HOTKEY_ROLES: [(HotkeyRole, &str, &str); 2] = [
    (HotkeyRole::Start, "start", "启动热键"),
    (HotkeyRole::Stop, "stop", "停止热键"),
];

fn hotkey_configured(hotkeys: &HotkeyConfig, role: HotkeyRole) -> String {
    match role {
        HotkeyRole::Start => hotkeys.start.clone(),
        HotkeyRole::Stop => hotkeys.stop.clone(),
    }
}

fn mouse_shortcut_display(shortcut: &MouseShortcut) -> String {
    let mut parts = Vec::new();
    let modifiers = shortcut.modifiers;
    if modifiers & input::MOD_CONTROL != 0 {
        parts.push("Control");
    }
    if modifiers & input::MOD_SHIFT != 0 {
        parts.push("Shift");
    }
    if modifiers & input::MOD_ALT != 0 {
        parts.push("Alt");
    }
    if modifiers & input::MOD_SUPER != 0 {
        parts.push("Super");
    }
    let button = match shortcut.button {
        input::MouseButton::Middle => "MouseMiddle",
        input::MouseButton::Back => "Mouse4",
        input::MouseButton::Forward => "Mouse5",
    };
    parts.push(button);
    parts.join("+")
}

#[derive(Debug, Clone)]
struct KeyboardBinding {
    id: &'static str,
    label: &'static str,
    configured: String,
    shortcut: Shortcut,
}

/// 解析每个角色的热键并记录 parse / parse.failed 事件（含原始错误），
/// 返回其中的键盘热键集合。
fn record_and_collect(state: &AppState, hotkeys: &HotkeyConfig) -> Vec<KeyboardBinding> {
    let mut collected = Vec::new();
    for (role, id, label) in HOTKEY_ROLES {
        let configured = hotkey_configured(hotkeys, role);
        match hotkeys.role_binding(role) {
            Ok(HotkeyBinding::Keyboard(shortcut)) => {
                state
                    .hotkeys
                    .record_parse(id, label, &configured, Ok(shortcut.to_string()));
                collected.push(KeyboardBinding {
                    id,
                    label,
                    configured,
                    shortcut,
                });
            }
            Ok(HotkeyBinding::Mouse(shortcut)) => {
                state.hotkeys.record_parse(
                    id,
                    label,
                    &configured,
                    Ok(mouse_shortcut_display(&shortcut)),
                );
            }
            Err(error) => {
                state
                    .hotkeys
                    .record_parse(id, label, &configured, Err(error));
            }
        }
    }
    collected
}

fn register_shortcuts(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    hotkeys: &HotkeyConfig,
) -> Result<(), String> {
    let bindings = record_and_collect(state, hotkeys);
    let manager = app.global_shortcut();
    let mut registered: Vec<KeyboardBinding> = Vec::new();
    for binding in &bindings {
        if let Err(error) = manager.register(binding.shortcut) {
            for done in &registered {
                let _ = manager.unregister(done.shortcut);
                state.hotkeys.record_rollback(
                    done.id,
                    done.label,
                    &done.configured,
                    format!("{} 注册失败，回滚注销：{}", binding.label, done.shortcut),
                );
            }
            let message = format!("无法注册全局热键 {}：{error}", binding.shortcut);
            state.hotkeys.record_register(
                binding.id,
                binding.label,
                &binding.configured,
                &binding.shortcut.to_string(),
                Err(message.clone()),
            );
            return Err(message);
        }
        state.hotkeys.record_register(
            binding.id,
            binding.label,
            &binding.configured,
            &binding.shortcut.to_string(),
            Ok(()),
        );
        registered.push(binding.clone());
    }
    for binding in &registered {
        state.hotkeys.record_is_registered(
            binding.id,
            binding.label,
            &binding.configured,
            manager.is_registered(binding.shortcut),
        );
    }
    Ok(())
}

fn restore_shortcuts(app: &tauri::AppHandle, state: &Arc<AppState>, hotkeys: &HotkeyConfig) {
    let bindings = record_and_collect(state, hotkeys);
    let manager = app.global_shortcut();
    for binding in &bindings {
        if !manager.is_registered(binding.shortcut) {
            let result = manager
                .register(binding.shortcut)
                .map_err(|error| format!("回滚后重新注册失败：{error}"));
            state.hotkeys.record_restore(
                binding.id,
                binding.label,
                &binding.configured,
                &binding.shortcut.to_string(),
                result,
            );
        }
        state.hotkeys.record_is_registered(
            binding.id,
            binding.label,
            &binding.configured,
            manager.is_registered(binding.shortcut),
        );
    }
}

fn replace_shortcuts(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    old_hotkeys: &HotkeyConfig,
    new_hotkeys: &HotkeyConfig,
) -> Result<(), String> {
    if old_hotkeys == new_hotkeys {
        return Ok(());
    }
    let manager = app.global_shortcut();
    let old_bindings = record_and_collect(state, old_hotkeys);
    for binding in &old_bindings {
        if manager.is_registered(binding.shortcut) {
            let result = manager
                .unregister(binding.shortcut)
                .map_err(|error| format!("无法注销原有全局热键 {}：{error}", binding.shortcut));
            state.hotkeys.record_unregister(
                binding.id,
                binding.label,
                &binding.configured,
                &binding.shortcut.to_string(),
                result.clone(),
            );
            if let Err(error) = result {
                restore_shortcuts(app, state, old_hotkeys);
                state.hotkeys.record_rollback_failed(
                    binding.id,
                    binding.label,
                    &binding.configured,
                    format!("注销 {} 失败，已尝试回滚原有热键", binding.shortcut),
                    error.clone(),
                );
                return Err(error);
            }
        }
    }
    if let Err(error) = register_shortcuts(app, state, new_hotkeys) {
        restore_shortcuts(app, state, old_hotkeys);
        for binding in &old_bindings {
            state.hotkeys.record_rollback(
                binding.id,
                binding.label,
                &binding.configured,
                format!("新热键注册失败，已回滚并重新注册：{}", binding.shortcut),
            );
        }
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
fn get_config(state: State<'_, Arc<AppState>>) -> AppConfig {
    state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone()
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    config: AppConfig,
) -> Result<AppConfig, String> {
    config.validate()?;
    let old_config = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    let old_hotkeys = old_config.hotkeys.clone();
    let new_hotkeys = config.hotkeys.clone();
    replace_shortcuts(&app, state.inner(), &old_hotkeys, &new_hotkeys)?;
    if let Err(error) = config::save(&app, &config) {
        state.diagnostics.error(
            "config",
            "config.save.failed",
            "设置保存失败",
            Some(error.clone()),
        );
        let _ = replace_shortcuts(&app, state.inner(), &new_hotkeys, &old_hotkeys);
        return Err(error);
    }
    *state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = config.clone();
    state
        .diagnostics
        .info("config", "config.save", "设置保存成功");
    let _ = app.emit("config-state", &config);
    Ok(config)
}

#[tauri::command]
fn get_runtime_snapshot(state: State<'_, Arc<AppState>>) -> RuntimeSnapshot {
    state.snapshot()
}

#[tauri::command]
fn detect_resolution() -> resolution::ResolutionInfo {
    resolution::detect()
}

fn start_internal(app: tauri::AppHandle, state: Arc<AppState>) -> Result<RuntimeSnapshot, String> {
    state.diagnostics.info(
        "sequence",
        "sequence.start.requested",
        "收到动作序列启动请求",
    );
    state.try_begin_sequence().map_err(str::to_string)?;
    let config = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    if let Err(error) = config.validate() {
        state.running.store(false, Ordering::Release);
        state.diagnostics.error(
            "sequence",
            "sequence.start.rejected",
            "配置校验失败，动作序列未启动",
            Some(error.clone()),
        );
        return Err(error);
    }
    state.cancel.store(false, Ordering::Release);
    let snapshot = state.update(&app, |runtime| {
        runtime.status = RuntimeStatus::Running;
        runtime.phase_index = 0;
        runtime.phase_name = "进场准备".into();
        runtime.message = "动作序列正在执行".into();
    });

    std::thread::spawn(move || {
        let result = engine::execute(&app, &state, &config);
        state.running.store(false, Ordering::Release);
        state.update(&app, |runtime| match &result {
            Ok(()) => {
                runtime.status = RuntimeStatus::Completed;
                runtime.message = "动作序列已完成".into();
            }
            Err(engine::SequenceError::Cancelled) => {
                runtime.status = RuntimeStatus::Aborted;
                runtime.message = "已安全中止，所有输入均已释放".into();
            }
            Err(engine::SequenceError::Input(error)) => {
                runtime.status = RuntimeStatus::Error;
                runtime.message = error.clone();
            }
        });
        match &result {
            Ok(()) => state
                .diagnostics
                .info("sequence", "sequence.completed", "动作序列已完成"),
            Err(engine::SequenceError::Cancelled) => state.diagnostics.info(
                "sequence",
                "sequence.cancelled",
                "动作序列已中止，输入已释放",
            ),
            Err(engine::SequenceError::Input(error)) => state.diagnostics.error(
                "sequence",
                "sequence.failed",
                "动作序列执行失败",
                Some(error.clone()),
            ),
        }
    });

    Ok(snapshot)
}

fn stop_internal(app: &tauri::AppHandle, state: &Arc<AppState>) -> RuntimeSnapshot {
    state
        .diagnostics
        .info("sequence", "sequence.stop.requested", "收到停止请求");
    if state.running.load(Ordering::Acquire) {
        state.request_cancel(app)
    } else {
        state.snapshot()
    }
}

#[tauri::command]
fn set_hotkey_capture_active(state: State<'_, Arc<AppState>>, active: bool) {
    state.hotkey_capture_active.store(active, Ordering::Release);
}

fn release_configured_inputs(state: &Arc<AppState>) {
    let configured_inputs = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .game_keys
        .applied()
        .map(|keys| keys.all());
    match configured_inputs {
        Ok(inputs) => input::release_all(&inputs),
        Err(_) => input::release_all(&[]),
    }
}

fn sync_overlay_to_game(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let enabled = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .overlay_visible;
    let Some(area) = enabled.then(resolution::active_game_client_area).flatten() else {
        if overlay.is_visible().unwrap_or(false) {
            let _ = overlay.hide();
        }
        return;
    };
    let Ok(size) = overlay.outer_size() else {
        return;
    };
    let available_x = i64::from(area.width).saturating_sub(i64::from(size.width));
    let x = i64::from(area.left) + available_x.max(0) / 2;
    let y = i64::from(area.top) + 20;
    let position = PhysicalPosition::new(
        x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
    );
    let _ = overlay.set_position(Position::Physical(position));
    if !overlay.is_visible().unwrap_or(false) {
        let _ = overlay.show();
    }
}

fn start_overlay_monitor(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || loop {
        sync_overlay_to_game(&app, &state);
        std::thread::sleep(std::time::Duration::from_millis(180));
    });
}

#[tauri::command]
fn start_sequence(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<RuntimeSnapshot, String> {
    start_internal(app, state.inner().clone())
}

#[tauri::command]
fn stop_sequence(app: tauri::AppHandle, state: State<'_, Arc<AppState>>) -> RuntimeSnapshot {
    stop_internal(&app, state.inner())
}

#[tauri::command]
fn set_overlay_visible(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    visible: bool,
) -> Result<bool, String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Overlay 窗口不存在".to_string())?;
    state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .overlay_visible = visible;
    if !visible {
        overlay.hide().map_err(|error| error.to_string())?;
    } else {
        sync_overlay_to_game(&app, state.inner());
    }
    Ok(visible)
}

#[tauri::command]
fn prepare_for_update(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = state.inner().clone();
    let was_running = state.lock_for_update().map_err(str::to_string)?;
    if was_running {
        state.request_cancel(&app);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while state.running.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    release_configured_inputs(&state);
    if state.running.load(Ordering::Acquire) {
        state.unlock_update();
        return Err("动作序列尚未安全停止，更新已取消".into());
    }
    Ok(())
}

#[tauri::command]
fn cancel_update_preparation(state: State<'_, Arc<AppState>>) {
    release_configured_inputs(state.inner());
    state.unlock_update();
}

#[tauri::command]
fn restart_after_update(app: tauri::AppHandle, state: State<'_, Arc<AppState>>) {
    state.cancel.store(true, Ordering::Release);
    release_configured_inputs(state.inner());
    app.restart();
}

fn spawn_mouse_hotkey_monitor(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut previous = input::polled_mouse_buttons();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let current = input::polled_mouse_buttons();
            if !state.hotkey_capture_active.load(Ordering::Acquire) {
                for (index, (&pressed, &was_pressed)) in
                    current.iter().zip(previous.iter()).enumerate()
                {
                    if !pressed || was_pressed {
                        continue;
                    }
                    let button = match index {
                        0 => input::MouseButton::Middle,
                        1 => input::MouseButton::Back,
                        _ => input::MouseButton::Forward,
                    };
                    let bindings = state
                        .config
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .hotkeys
                        .bindings();
                    let Ok([start_binding, stop_binding]) = bindings else {
                        continue;
                    };
                    let pressed_shortcut = MouseShortcut {
                        button,
                        modifiers: input::modifier_mask(),
                    };
                    if start_binding == HotkeyBinding::Mouse(pressed_shortcut) {
                        let _ = start_internal(app.clone(), Arc::clone(&state));
                    } else if stop_binding == HotkeyBinding::Mouse(pressed_shortcut) {
                        stop_internal(&app, &state);
                    }
                }
            }
            previous = current;
        }
    });
}

// ---------- 诊断命令 ----------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePingResult {
    ok: bool,
    version: String,
    commit_sha: String,
    commit_short: String,
    build_profile: String,
    os: &'static str,
    arch: &'static str,
    pid: u32,
    timestamp: String,
}

/// 原生后端握手：正式构建中前端以该命令判定后端可用性，
/// 握手失败时禁用动作执行（前端不再静默进入 mock）。
#[tauri::command]
fn native_ping(state: State<'_, Arc<AppState>>) -> NativePingResult {
    let info = diagnostics::build_info();
    state.diagnostics.info(
        "backend",
        "backend.ping",
        format!("native_ping 握手成功（v{}）", info.version),
    );
    NativePingResult {
        ok: true,
        version: info.version,
        commit_sha: info.commit_sha.clone(),
        commit_short: info.commit_short.clone(),
        build_profile: info.build_profile.clone(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        pid: std::process::id(),
        timestamp: diagnostics::iso8601_now(),
    }
}

/// 重新检查当前热键注册状态并返回完整状态报告。
#[tauri::command]
fn get_hotkey_status(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Vec<HotkeyStatusEntry> {
    let manager = app.global_shortcut();
    let hotkeys = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .hotkeys
        .clone();
    for binding in record_and_collect(state.inner(), &hotkeys) {
        state.hotkeys.record_is_registered(
            binding.id,
            binding.label,
            &binding.configured,
            manager.is_registered(binding.shortcut),
        );
    }
    state.hotkeys.report()
}

/// 主动输入自检：扫描码 W、虚拟键 W、相对鼠标移动。
/// 探针会向前台窗口注入一次极短 W 按键与 ±1 像素鼠标移动，仅由用户显式触发。
#[tauri::command]
fn run_input_probes(state: State<'_, Arc<AppState>>) -> Vec<InputProbeResult> {
    state.diagnostics.info(
        "input",
        "input.probes.start",
        "开始主动输入自检（3 项探针）",
    );
    let probes = input::run_probes();
    for probe in &probes {
        let event = format!(
            "input.probe.{}.{}",
            probe.probe,
            if probe.ok { "ok" } else { "failed" }
        );
        let message = format!(
            "{}：请求 {} 次 / 返回 {} 次 / 前台进程 {} / 完整性 {}",
            probe.label,
            probe.requested,
            probe.sent,
            probe.foreground_process.as_deref().unwrap_or("未知"),
            probe.integrity_level.as_deref().unwrap_or("未知"),
        );
        if probe.ok {
            state.diagnostics.info("input", &event, message.clone());
            if probe.observed_async_down == Some(false) {
                state.diagnostics.warn(
                    "input",
                    "input.probe.observed-down-missing",
                    format!(
                        "{}：SendInput 成功但 GetAsyncKeyState 未观察到按下状态（可能被 UIPI 过滤）",
                        probe.label
                    ),
                    None,
                );
            }
        } else {
            state
                .diagnostics
                .error("input", &event, message, probe.last_error.clone());
        }
    }
    *state
        .probes
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = probes.clone();
    probes
}

fn environment_info(app: &tauri::AppHandle, state: &AppState) -> EnvironmentInfo {
    let info = diagnostics::build_info();
    let config_dir = app.path().app_config_dir().ok();
    let downloads = system::downloads_dir();
    EnvironmentInfo {
        os: std::env::consts::OS.to_string(),
        os_version: system::os_version(),
        arch: std::env::consts::ARCH.to_string(),
        app_version: info.version.clone(),
        commit_sha: info.commit_sha.clone(),
        build_profile: info.build_profile.clone(),
        config_path: config_dir
            .as_deref()
            .map(diagnostics::sanitize_path)
            .unwrap_or_else(|| "未知".into()),
        log_path: state
            .diagnostics
            .log_path()
            .map(diagnostics::sanitize_path)
            .unwrap_or_else(|| "仅内存（未写盘）".into()),
        downloads_path: downloads
            .as_deref()
            .map(diagnostics::sanitize_path)
            .unwrap_or_else(|| "未知".into()),
        foreground_process: system::foreground_process_name(),
        foreground_integrity: system::foreground_integrity_level(),
        session_uptime_s: state.diagnostics.uptime(),
        generated_at: diagnostics::iso8601_now(),
    }
}

#[tauri::command]
fn get_environment_info(app: tauri::AppHandle, state: State<'_, Arc<AppState>>) -> EnvironmentInfo {
    environment_info(&app, state.inner())
}

#[tauri::command]
fn get_diagnostic_events(state: State<'_, Arc<AppState>>, limit: usize) -> Vec<DiagnosticEvent> {
    state.diagnostics.events(limit)
}

/// 把诊断包导出为 ZIP 到用户下载目录。
#[tauri::command]
fn export_diagnostics_package(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<DiagnosticsExportResult, String> {
    let state = state.inner().clone();
    let target_dir = system::downloads_dir().ok_or_else(|| "无法确定用户下载目录".to_string())?;
    let build = diagnostics::build_info();
    let environment = environment_info(&app, &state);
    let config = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    let hotkeys = state.hotkeys.report();
    let probes = state
        .probes
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    let events = state.diagnostics.all_events();
    let bundle = diagnostics::assemble_export_bundle(
        &build,
        &environment,
        &hotkeys,
        &probes,
        &events,
        &config,
    );
    let result = diagnostics::write_export_zip(&target_dir, &build, &bundle)?;
    state.diagnostics.info(
        "export",
        "export.zip.written",
        format!(
            "诊断包已导出：{}（{} 个文件，{} 字节）",
            diagnostics::sanitize_path(std::path::Path::new(&result.path)),
            result.file_count,
            result.size_bytes,
        ),
    );
    Ok(result)
}

pub fn run() {
    let global_shortcut = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let state = app.state::<Arc<AppState>>().inner().clone();
            if state.hotkey_capture_active.load(Ordering::Acquire) {
                return;
            }
            let bindings = state
                .config
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .hotkeys
                .bindings();
            let Ok([start_binding, stop_binding]) = bindings else {
                return;
            };
            if start_binding == HotkeyBinding::Keyboard(*shortcut) {
                let _ = start_internal(app.clone(), state);
            } else if stop_binding == HotkeyBinding::Keyboard(*shortcut) {
                stop_internal(app, &state);
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(global_shortcut)
        .setup(|app| {
            let hub = diagnostics::Hub::new(
                app.path()
                    .app_log_dir()
                    .ok()
                    .map(|dir| dir.join("diagnostics.jsonl")),
            );
            let info = diagnostics::build_info();
            hub.info(
                "backend",
                "backend.boot",
                format!("后端启动 v{} (commit {})", info.version, info.commit_short),
            );
            let config = match config::load(app.handle()) {
                Ok(config) => {
                    hub.info("config", "config.load", "设置加载成功");
                    config
                }
                Err(error) => {
                    hub.error(
                        "config",
                        "config.load.failed",
                        "设置加载失败，使用默认设置",
                        Some(error),
                    );
                    AppConfig::default()
                }
            };
            let overlay_visible = config.overlay_visible;
            let hotkeys = config.hotkeys.clone();
            let state = Arc::new(AppState::with_hub(config, hub));
            app.manage(Arc::clone(&state));
            spawn_mouse_hotkey_monitor(app.handle().clone(), Arc::clone(&state));
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            if let Err(error) = register_shortcuts(app.handle(), &state, &hotkeys) {
                state.diagnostics.error(
                    "hotkey",
                    "hotkey.startup.register.failed",
                    "启动时全局热键注册失败",
                    Some(error.clone()),
                );
                state.update(app.handle(), |runtime| {
                    runtime.status = RuntimeStatus::Error;
                    runtime.message = error;
                });
            }
            if let Some(overlay) = app.get_webview_window("overlay") {
                overlay.set_ignore_cursor_events(true)?;
                overlay.hide()?;
            }
            if overlay_visible {
                sync_overlay_to_game(app.handle(), &state);
            }
            start_overlay_monitor(app.handle().clone(), Arc::clone(&state));
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                let state = window.state::<Arc<AppState>>().inner().clone();
                state.cancel.store(true, Ordering::Release);
                release_configured_inputs(&state);
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_runtime_snapshot,
            detect_resolution,
            start_sequence,
            stop_sequence,
            set_hotkey_capture_active,
            set_overlay_visible,
            prepare_for_update,
            cancel_update_preparation,
            restart_after_update,
            native_ping,
            get_hotkey_status,
            run_input_probes,
            get_environment_info,
            get_diagnostic_events,
            export_diagnostics_package,
        ])
        .run(tauri::generate_context!())
        .expect("启动 D2 Morgeth Kick 失败");
}
