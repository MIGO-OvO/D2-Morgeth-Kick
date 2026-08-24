mod config;
mod engine;
mod input;
mod resolution;
mod runtime;

use std::sync::{atomic::Ordering, Arc};
use tauri::{Emitter, Manager, PhysicalPosition, Position, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use config::{AppConfig, HotkeyBinding, MouseShortcut};
use runtime::{AppState, RuntimeSnapshot, RuntimeStatus};

fn keyboard_shortcuts(bindings: [HotkeyBinding; 2]) -> Vec<Shortcut> {
    bindings
        .into_iter()
        .filter_map(HotkeyBinding::keyboard)
        .collect()
}

fn register_shortcuts(app: &tauri::AppHandle, bindings: [HotkeyBinding; 2]) -> Result<(), String> {
    let manager = app.global_shortcut();
    let mut registered = Vec::new();
    for shortcut in keyboard_shortcuts(bindings) {
        if let Err(error) = manager.register(shortcut) {
            for registered_shortcut in registered {
                let _ = manager.unregister(registered_shortcut);
            }
            return Err(format!("无法注册全局热键 {shortcut}：{error}"));
        }
        registered.push(shortcut);
    }
    Ok(())
}

fn restore_shortcuts(app: &tauri::AppHandle, bindings: [HotkeyBinding; 2]) {
    let manager = app.global_shortcut();
    for shortcut in keyboard_shortcuts(bindings) {
        if !manager.is_registered(shortcut) {
            let _ = manager.register(shortcut);
        }
    }
}

fn replace_shortcuts(
    app: &tauri::AppHandle,
    old_bindings: [HotkeyBinding; 2],
    new_bindings: [HotkeyBinding; 2],
) -> Result<(), String> {
    if old_bindings == new_bindings {
        return Ok(());
    }
    let manager = app.global_shortcut();
    for shortcut in keyboard_shortcuts(old_bindings) {
        if manager.is_registered(shortcut) {
            if let Err(error) = manager.unregister(shortcut) {
                restore_shortcuts(app, old_bindings);
                return Err(format!("无法注销原有全局热键 {shortcut}：{error}"));
            }
        }
    }
    if let Err(error) = register_shortcuts(app, new_bindings) {
        restore_shortcuts(app, old_bindings);
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
    let old_bindings = old_config.hotkeys.bindings()?;
    let new_bindings = config.hotkeys.bindings()?;
    replace_shortcuts(&app, old_bindings, new_bindings)?;
    if let Err(error) = config::save(&app, &config) {
        let _ = replace_shortcuts(&app, new_bindings, old_bindings);
        return Err(error);
    }
    *state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = config.clone();
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
    state.try_begin_sequence().map_err(str::to_string)?;
    let config = state
        .config
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    if let Err(error) = config.validate() {
        state.running.store(false, Ordering::Release);
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
        state.update(&app, |runtime| match result {
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
                runtime.message = error;
            }
        });
    });

    Ok(snapshot)
}

fn stop_internal(app: &tauri::AppHandle, state: &Arc<AppState>) -> RuntimeSnapshot {
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
            let config = config::load(app.handle()).unwrap_or_else(|_| AppConfig::default());
            let overlay_visible = config.overlay_visible;
            let bindings = config.hotkeys.bindings()?;
            let state = Arc::new(AppState::new(config));
            app.manage(Arc::clone(&state));
            spawn_mouse_hotkey_monitor(app.handle().clone(), Arc::clone(&state));
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            if let Err(error) = register_shortcuts(app.handle(), bindings) {
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
        ])
        .run(tauri::generate_context!())
        .expect("启动 D2 Morgeth Kick 失败");
}
