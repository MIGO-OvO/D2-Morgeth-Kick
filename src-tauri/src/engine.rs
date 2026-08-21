use std::{
    sync::{atomic::Ordering, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

use crate::{
    config::{AppConfig, AppliedGameKeys},
    input::{self, KEY_A, KEY_D, KEY_S, KEY_W},
    runtime::AppState,
};

#[derive(Debug)]
pub enum SequenceError {
    Cancelled,
    Input(String),
}

impl std::fmt::Display for SequenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(formatter, "用户已中止"),
            Self::Input(message) => formatter.write_str(message),
        }
    }
}

type SequenceResult<T = ()> = Result<T, SequenceError>;

fn check_cancel(state: &AppState) -> SequenceResult {
    if state.cancel.load(Ordering::Acquire) {
        Err(SequenceError::Cancelled)
    } else {
        Ok(())
    }
}

fn wait(state: &AppState, duration: Duration) -> SequenceResult {
    let deadline = Instant::now() + duration;
    loop {
        check_cancel(state)?;
        let now = Instant::now();
        if now >= deadline {
            return Ok(());
        }
        thread::sleep((deadline - now).min(Duration::from_millis(10)));
    }
}

fn wait_seconds(state: &AppState, seconds: f64) -> SequenceResult {
    wait(state, Duration::from_secs_f64(seconds.max(0.0)))
}

fn wait_until(state: &AppState, start: Instant, seconds: f64) -> SequenceResult {
    let target = start + Duration::from_secs_f64(seconds.max(0.0));
    let now = Instant::now();
    if target > now {
        wait(state, target - now)
    } else {
        check_cancel(state)
    }
}

fn key(state: &AppState, scan_code: u16, down: bool) -> SequenceResult {
    check_cancel(state)?;
    input::key(scan_code, down).map_err(SequenceError::Input)
}

fn tap(state: &AppState, scan_code: u16, hold_ms: u64) -> SequenceResult {
    key(state, scan_code, true)?;
    wait(state, Duration::from_millis(hold_ms))?;
    key(state, scan_code, false)
}

fn right_mouse(state: &AppState, down: bool) -> SequenceResult {
    check_cancel(state)?;
    input::right_mouse(down).map_err(SequenceError::Input)
}

fn mouse_step_count(offset: [i32; 2], reference_offset: [i32; 2], step_px: i32) -> i32 {
    // Sensitivity changes the distance per step, while the reference calibration fixes duration.
    let reference_distance = reference_offset[0].abs().max(reference_offset[1].abs());
    let distance = if reference_distance == 0 {
        offset[0].abs().max(offset[1].abs())
    } else {
        reference_distance
    };
    (distance / step_px.max(1)).max(1)
}

fn turn_camera(
    state: &AppState,
    offset: [i32; 2],
    reference_offset: [i32; 2],
    step_px: i32,
) -> SequenceResult {
    let [dx, dy] = offset;
    if dx == 0 && dy == 0 {
        return Ok(());
    }
    let steps = mouse_step_count(offset, reference_offset, step_px);
    for index in 0..steps {
        check_cancel(state)?;
        let current_x = ((index + 1) * dx / steps) - (index * dx / steps);
        let current_y = ((index + 1) * dy / steps) - (index * dy / steps);
        if current_x != 0 || current_y != 0 {
            input::mouse_move(current_x, current_y).map_err(SequenceError::Input)?;
        }
        wait(state, Duration::from_millis(5))?;
    }
    wait(state, Duration::from_millis(100))
}

fn set_phase(app: &AppHandle, state: &AppState, index: usize, name: &str) {
    state.update(app, |runtime| {
        runtime.phase_index = index;
        runtime.phase_name = name.into();
        runtime.message = format!("正在执行：{name}");
    });
}

// The decimal values below are recorded replay timestamps, not mathematical constants.
#[allow(clippy::approx_constant)]
fn pre_ascension(app: &AppHandle, state: &AppState, keys: AppliedGameKeys) -> SequenceResult {
    let start = Instant::now();
    key(state, KEY_W, true)?;
    wait_until(state, start, 0.359)?;
    key(state, KEY_W, false)?;

    wait_until(state, start, 0.718)?;
    key(state, KEY_A, true)?;
    wait_until(state, start, 1.172)?;
    key(state, KEY_A, false)?;

    wait_until(state, start, 1.859)?;
    key(state, keys.interact, true)?;
    wait_until(state, start, 2.718)?;
    key(state, keys.interact, false)?;

    wait_until(state, start, 4.703)?;
    key(state, keys.interact, true)?;
    wait_until(state, start, 5.265)?;
    key(state, keys.interact, false)?;

    wait_until(state, start, 5.672)?;
    tap(state, keys.weapon_slot_2, 28)?;
    wait_until(state, start, 5.953)?;
    tap(state, keys.weapon_slot_2, 28)?;

    wait_until(state, start, 7.656)?;
    key(state, KEY_W, true)?;
    wait_until(state, start, 8.343)?;
    key(state, keys.sprint, true)?;
    wait_until(state, start, 8.500)?;
    key(state, keys.sprint, false)?;

    wait_until(state, start, 10.109)?;
    key(state, KEY_D, true)?;
    wait_until(state, start, 12.125)?;
    key(state, KEY_D, false)?;

    wait_until(state, start, 13.015)?;
    key(state, KEY_D, true)?;
    wait_until(state, start, 13.312)?;
    key(state, KEY_D, false)?;

    wait_until(state, start, 15.406)?;
    key(state, KEY_A, true)?;
    wait_until(state, start, 16.218)?;
    key(state, KEY_A, false)?;

    wait_until(state, start, 16.422)?;
    key(state, KEY_A, true)?;
    wait_until(state, start, 16.609)?;
    key(state, KEY_A, false)?;

    wait_until(state, start, 17.422)?;
    key(state, KEY_W, false)?;
    set_phase(app, state, 1, "飞升");
    wait_until(state, start, 17.625)?;
    key(state, keys.jump, true)?;
    wait_until(state, start, 17.781)?;
    key(state, keys.jump, false)?;
    wait_until(state, start, 18.031)
}

fn post_ascension(
    app: &AppHandle,
    state: &AppState,
    config: &AppConfig,
    keys: AppliedGameKeys,
) -> SequenceResult {
    tap(state, keys.ascension, 35)?;
    wait_seconds(state, config.timings.ascension_wait)?;

    set_phase(app, state, 2, "后退定位");
    let start = Instant::now();
    key(state, KEY_S, true)?;
    wait_until(state, start, 0.188)?;
    key(state, KEY_S, false)?;
    wait_until(state, start, 0.469)?;
    key(state, KEY_S, true)?;
    wait_until(state, start, 0.594)?;
    key(state, KEY_S, false)?;

    wait_until(state, start, 2.109)?;
    set_phase(app, state, 3, "ADS 近战");
    right_mouse(state, true)?;
    wait_until(state, start, 2.125)?;
    turn_camera(state, config.first_ads_offset(), config.first_ads_base, 10)?;

    wait_until(state, start, 4.047)?;
    wait_seconds(state, config.timings.melee_extra_wait)?;
    key(state, keys.melee, true)?;
    wait(state, Duration::from_millis(47))?;
    right_mouse(state, false)?;
    wait(state, Duration::from_millis(31))?;
    key(state, keys.melee, false)?;

    wait_until(state, start, 2.109 + config.timings.ads_to_super_wait)?;
    set_phase(app, state, 4, "虚空箭");
    turn_camera(
        state,
        config.void_arrow_offset(),
        config.void_arrow_base,
        10,
    )?;
    tap(state, keys.super_ability, 125)?;
    wait_seconds(state, config.timings.super_wait)?;

    set_phase(app, state, 5, "冲刺");
    key(state, KEY_A, true)?;
    wait_seconds(state, config.timings.sprint_a_time)?;
    key(state, KEY_W, true)?;
    wait(state, Duration::from_millis(78))?;
    key(state, keys.sprint, true)?;
    turn_camera(state, config.sprint_offset(), config.sprint_base, 5)?;
    wait_seconds(state, config.timings.sprint_to_finisher)?;

    set_phase(app, state, 6, "终结");
    for index in 0..4 {
        key(state, keys.finisher, true)?;
        wait(state, Duration::from_millis(140))?;
        key(state, keys.finisher, false)?;
        if index < 3 {
            wait(state, Duration::from_millis(60))?;
        }
    }
    key(state, KEY_A, false)?;
    key(state, KEY_W, false)?;
    key(state, keys.sprint, false)
}

pub fn execute(app: &AppHandle, state: &Arc<AppState>, config: &AppConfig) -> SequenceResult {
    let keys = config.game_keys.applied().map_err(SequenceError::Input)?;
    let result = (|| {
        set_phase(app, state, 0, "进场准备");
        pre_ascension(app, state, keys)?;
        post_ascension(app, state, config, keys)
    })();
    input::release_all(&keys.all());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn mouse_step_count_preserves_python_floor_behavior() {
        assert_eq!(mouse_step_count([-2600, 50], [-2600, 50], 10), 260);
        assert_eq!(mouse_step_count([9, 0], [9, 0], 10), 1);
    }

    #[test]
    fn sensitivity_scaling_keeps_reference_mouse_move_duration() {
        let reference_offset = [-2600, 50];
        let reference_steps = mouse_step_count(reference_offset, reference_offset, 10);
        let low_sensitivity_offset = [-5200, 100];

        assert_eq!(
            mouse_step_count(low_sensitivity_offset, reference_offset, 10),
            reference_steps
        );
    }

    #[test]
    fn missing_reference_offset_falls_back_to_actual_distance() {
        assert_eq!(mouse_step_count([300, 0], [0, 0], 10), 30);
    }

    #[test]
    fn cancellation_interrupts_an_eighteen_second_wait() {
        let state = Arc::new(AppState::new(AppConfig::default()));
        let cancel_state = Arc::clone(&state);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            cancel_state.cancel.store(true, Ordering::Release);
        });
        let started = Instant::now();
        assert!(matches!(
            wait(&state, Duration::from_secs(18)),
            Err(SequenceError::Cancelled)
        ));
        assert!(started.elapsed() < Duration::from_millis(250));
    }
}
