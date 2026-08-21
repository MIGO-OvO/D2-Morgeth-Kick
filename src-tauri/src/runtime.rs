use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Emitter};

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeStatus {
    Ready,
    Running,
    Stopping,
    Completed,
    Aborted,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub phase_index: usize,
    pub phase_name: String,
    pub message: String,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            status: RuntimeStatus::Ready,
            phase_index: 0,
            phase_name: "进场准备".into(),
            message: "参数已就绪，按 F8 启动".into(),
        }
    }
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub runtime: Mutex<RuntimeSnapshot>,
    pub running: AtomicBool,
    pub cancel: AtomicBool,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let start_hotkey = config.hotkeys.start.clone();
        Self {
            config: Mutex::new(config),
            runtime: Mutex::new(RuntimeSnapshot {
                message: format!("参数已就绪，按 {start_hotkey} 启动"),
                ..RuntimeSnapshot::default()
            }),
            running: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.runtime
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    pub fn update(
        &self,
        app: &AppHandle,
        update: impl FnOnce(&mut RuntimeSnapshot),
    ) -> RuntimeSnapshot {
        let snapshot = {
            let mut runtime = self
                .runtime
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            update(&mut runtime);
            runtime.clone()
        };
        let _ = app.emit("runtime-state", &snapshot);
        snapshot
    }

    pub fn request_cancel(&self, app: &AppHandle) -> RuntimeSnapshot {
        self.cancel.store(true, Ordering::Release);
        self.update(app, |runtime| {
            if runtime.status == RuntimeStatus::Running {
                runtime.status = RuntimeStatus::Stopping;
                runtime.message = "正在停止并释放按键…".into();
            }
        })
    }
}
