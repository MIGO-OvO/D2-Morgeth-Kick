mod config;
mod engine;
mod input;
mod resolution;
mod runtime;

use std::sync::{atomic::Ordering, Arc};
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use config::AppConfig;
use runtime::{AppState, RuntimeSnapshot, RuntimeStatus};

fn register_shortcuts(app: &tauri::AppHandle, shortcuts: [Shortcut; 2]) -> Result<(), String> {
    let manager = app.global_shortcut();
    let mut registered = Vec::new();
    for shortcut in shortcuts {
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

fn restore_shortcuts(app: &tauri::AppHandle, shortcuts: [Shortcut; 2]) {
    let manager = app.global_shortcut();
    for shortcut in shortcuts {
        if !manager.is_registered(shortcut) {
            let _ = manager.register(shortcut);
        }
    }
}

fn replace_shortcuts(
    app: &tauri::AppHandle,
    old_shortcuts: [Shortcut; 2],
    new_shortcuts: [Shortcut; 2],
) -> Result<(), String> {
    if old_shortcuts == new_shortcuts {
        return Ok(());
    }
    let manager = app.global_shortcut();
    for shortcut in old_shortcuts {
        if manager.is_registered(shortcut) {
            if let Err(error) = manager.unregister(shortcut) {
                restore_shortcuts(app, old_shortcuts);
                return Err(format!("无法注销原有全局热键 {shortcut}：{error}"));
            }
        }
    }
    if let Err(error) = register_shortcuts(app, new_shortcuts) {
        restore_shortcuts(app, old_shortcuts);
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
    let old_shortcuts = old_config.hotkeys.shortcuts()?;
    let new_shortcuts = config.hotkeys.shortcuts()?;
    replace_shortcuts(&app, old_shortcuts, new_shortcuts)?;
    if let Err(error) = config::save(&app, &config) {
        let _ = replace_shortcuts(&app, new_shortcuts, old_shortcuts);
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
    state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "动作序列已经在运行".to_string())?;
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
fn set_overlay_visible(app: tauri::AppHandle, visible: bool) -> Result<bool, String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Overlay 窗口不存在".to_string())?;
    if visible {
        overlay.show().map_err(|error| error.to_string())?;
    } else {
        overlay.hide().map_err(|error| error.to_string())?;
    }
    Ok(visible)
}

pub fn run() {
    let global_shortcut = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let state = app.state::<Arc<AppState>>().inner().clone();
            let shortcuts = state
                .config
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .hotkeys
                .shortcuts();
            let Ok([start_shortcut, stop_shortcut]) = shortcuts else {
                return;
            };
            if *shortcut == start_shortcut {
                let _ = start_internal(app.clone(), state);
            } else if *shortcut == stop_shortcut {
                stop_internal(app, &state);
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(global_shortcut)
        .setup(|app| {
            let config = config::load(app.handle()).unwrap_or_else(|_| AppConfig::default());
            let overlay_visible = config.overlay_visible;
            let shortcuts = config.hotkeys.shortcuts()?;
            let state = Arc::new(AppState::new(config));
            app.manage(Arc::clone(&state));
            if let Err(error) = register_shortcuts(app.handle(), shortcuts) {
                state.update(app.handle(), |runtime| {
                    runtime.status = RuntimeStatus::Error;
                    runtime.message = error;
                });
            }
            if let Some(overlay) = app.get_webview_window("overlay") {
                overlay.set_ignore_cursor_events(true)?;
                if !overlay_visible {
                    overlay.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                let state = window.state::<Arc<AppState>>().inner().clone();
                state.cancel.store(true, Ordering::Release);
                let configured_keys = state
                    .config
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .game_keys
                    .applied()
                    .map(|keys| keys.all())
                    .unwrap_or_default();
                input::release_all(&configured_keys);
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
            set_overlay_visible,
        ])
        .run(tauri::generate_context!())
        .expect("启动 D2 Morgath Kick 失败");
}
