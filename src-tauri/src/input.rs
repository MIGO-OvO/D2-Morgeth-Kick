use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInput {
    pub scan_code: u16,
    pub extended: bool,
}

impl KeyboardInput {
    pub const fn standard(scan_code: u16) -> Self {
        Self {
            scan_code,
            extended: false,
        }
    }

    pub const fn extended(scan_code: u16) -> Self {
        Self {
            scan_code,
            extended: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Middle,
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBinding {
    Keyboard(KeyboardInput),
    Mouse(MouseButton),
}

pub const MOD_CONTROL: u8 = 1 << 0;
pub const MOD_SHIFT: u8 = 1 << 1;
pub const MOD_ALT: u8 = 1 << 2;
pub const MOD_SUPER: u8 = 1 << 3;

pub const KEY_W: InputBinding = InputBinding::Keyboard(KeyboardInput::standard(0x11));
pub const KEY_A: InputBinding = InputBinding::Keyboard(KeyboardInput::standard(0x1e));
pub const KEY_S: InputBinding = InputBinding::Keyboard(KeyboardInput::standard(0x1f));
pub const KEY_D: InputBinding = InputBinding::Keyboard(KeyboardInput::standard(0x20));

/// 单次主动输入自检（SendInput 探针）的结果。
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputProbeResult {
    /// scan-code-w | virtual-key-w | mouse-relative
    pub probe: String,
    pub label: String,
    pub description: String,
    pub ok: bool,
    /// 计划发起的 SendInput 请求数（事件数）。
    pub requested: u32,
    /// 实际调用 SendInput 的次数。
    pub calls: u32,
    /// SendInput 返回值之和（成功注入的事件数）。
    pub sent: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_code: Option<u32>,
    /// 原始 LastError（系统消息 + 错误码）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground_process: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity_level: Option<String>,
    /// W 类探针：按下后观察到的 GetAsyncKeyState 状态（仅供参考，不参与判定）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_async_down: Option<bool>,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// SendInput 调用累计器：与平台无关，便于单元测试。
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SendOutcome {
    pub requested: u32,
    pub calls: u32,
    pub sent: u32,
    pub last_error_code: Option<u32>,
}

impl SendOutcome {
    pub fn record_call(&mut self, sent: u32, error_code: u32) {
        self.calls += 1;
        self.sent += sent;
        if sent != 1 && self.last_error_code.is_none() && error_code != 0 {
            self.last_error_code = Some(error_code);
        }
    }

    pub fn ok(&self) -> bool {
        self.calls == self.requested
            && self.sent == self.requested
            && self.last_error_code.is_none()
    }
}

/// 依次运行三种主动输入自检：扫描码 W、虚拟键 W、相对鼠标移动。
/// 注意：探针会向前台窗口注入一次极短 W 按键与 ±1 像素的鼠标移动，
/// 因此仅由用户在诊断面板中显式触发。
pub fn run_probes() -> Vec<InputProbeResult> {
    platform::run_probes()
}

#[cfg(windows)]
mod platform {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_MIDDLEDOWN,
        MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, VK_CONTROL, VK_LWIN, VK_MBUTTON, VK_MENU,
        VK_RWIN, VK_SHIFT, VK_W, VK_XBUTTON1, VK_XBUTTON2,
    };

    use super::{
        KeyboardInput, MouseButton, SendOutcome, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_SUPER,
    };
    use crate::system;

    fn send(input: &INPUT) -> Result<(), String> {
        let sent = unsafe { SendInput(1, input, size_of::<INPUT>() as i32) };
        if sent == 1 {
            Ok(())
        } else {
            Err(format!(
                "SendInput 失败：{}",
                std::io::Error::last_os_error()
            ))
        }
    }

    fn send_raw(input: &INPUT) -> (u32, u32) {
        let sent = unsafe { SendInput(1, input, size_of::<INPUT>() as i32) };
        (sent, unsafe { GetLastError() })
    }

    fn keyboard_input(scan_code: u16, virtual_key: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: virtual_key,
                    wScan: scan_code,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn async_down(virtual_key: u16) -> bool {
        unsafe { GetAsyncKeyState(i32::from(virtual_key)) as u16 & 0x8000 != 0 }
    }

    fn probe_result(
        probe: &str,
        label: &str,
        description: &str,
        outcome: SendOutcome,
        started: std::time::Instant,
        observed: Option<bool>,
    ) -> super::InputProbeResult {
        super::InputProbeResult {
            probe: probe.into(),
            label: label.into(),
            description: description.into(),
            ok: outcome.ok(),
            requested: outcome.requested,
            calls: outcome.calls,
            sent: outcome.sent,
            last_error_code: outcome.last_error_code,
            last_error: outcome.last_error_code.map(system::win32_error_message),
            foreground_process: system::foreground_process_name(),
            integrity_level: system::foreground_integrity_level(),
            observed_async_down: observed,
            duration_ms: started.elapsed().as_millis() as u64,
            timestamp: crate::diagnostics::iso8601_now(),
        }
    }

    /// 探针 1：扫描码 W（KEYEVENTF_SCANCODE + 0x11）。
    fn probe_scan_code_w() -> super::InputProbeResult {
        let started = std::time::Instant::now();
        let mut outcome = SendOutcome {
            requested: 2,
            ..SendOutcome::default()
        };
        let down = keyboard_input(0x11, 0, KEYEVENTF_SCANCODE);
        let (sent, error_code) = send_raw(&down);
        outcome.record_call(sent, error_code);
        std::thread::sleep(std::time::Duration::from_millis(40));
        let observed = async_down(VK_W);
        let up = keyboard_input(0x11, 0, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
        let (sent, error_code) = send_raw(&up);
        outcome.record_call(sent, error_code);
        probe_result(
            "scan-code-w",
            "扫描码 W",
            "KEYEVENTF_SCANCODE 注入 W（0x11）按下与抬起",
            outcome,
            started,
            Some(observed),
        )
    }

    /// 探针 2：虚拟键 W（wVk=0x57，不带扫描码标志）。
    fn probe_virtual_key_w() -> super::InputProbeResult {
        let started = std::time::Instant::now();
        let mut outcome = SendOutcome {
            requested: 2,
            ..SendOutcome::default()
        };
        let down = keyboard_input(0, VK_W, 0);
        let (sent, error_code) = send_raw(&down);
        outcome.record_call(sent, error_code);
        std::thread::sleep(std::time::Duration::from_millis(40));
        let observed = async_down(VK_W);
        let up = keyboard_input(0, VK_W, KEYEVENTF_KEYUP);
        let (sent, error_code) = send_raw(&up);
        outcome.record_call(sent, error_code);
        probe_result(
            "virtual-key-w",
            "虚拟键 W",
            "wVk=0x57 虚拟键注入 W 按下与抬起",
            outcome,
            started,
            Some(observed),
        )
    }

    /// 探针 3：相对鼠标移动（+1,0 再 -1,0，净位移为零）。
    fn probe_mouse_relative() -> super::InputProbeResult {
        let started = std::time::Instant::now();
        let mut outcome = SendOutcome {
            requested: 2,
            ..SendOutcome::default()
        };
        let input = |dx: i32| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let (sent, error_code) = send_raw(&input(1));
        outcome.record_call(sent, error_code);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let (sent, error_code) = send_raw(&input(-1));
        outcome.record_call(sent, error_code);
        probe_result(
            "mouse-relative",
            "相对鼠标移动",
            "MOUSEEVENTF_MOVE 相对移动 (+1,0) 与 (-1,0)，净位移为零",
            outcome,
            started,
            None,
        )
    }

    pub fn run_probes() -> Vec<super::InputProbeResult> {
        vec![
            probe_scan_code_w(),
            probe_virtual_key_w(),
            probe_mouse_relative(),
        ]
    }

    fn mouse(flags: u32, data: u32) -> Result<(), String> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        send(&input)
    }

    pub fn keyboard(key: KeyboardInput, down: bool) -> Result<(), String> {
        let mut flags = KEYEVENTF_SCANCODE | if down { 0 } else { KEYEVENTF_KEYUP };
        if key.extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: key.scan_code,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        send(&input)
    }

    pub fn mouse_move(dx: i32, dy: i32) -> Result<(), String> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        send(&input)
    }

    pub fn right_mouse(down: bool) -> Result<(), String> {
        mouse(
            if down {
                MOUSEEVENTF_RIGHTDOWN
            } else {
                MOUSEEVENTF_RIGHTUP
            },
            0,
        )
    }

    pub fn mouse_button(button: MouseButton, down: bool) -> Result<(), String> {
        let (flags, data) = match (button, down) {
            (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
            (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
            (MouseButton::Back, true) => (MOUSEEVENTF_XDOWN, 1),
            (MouseButton::Back, false) => (MOUSEEVENTF_XUP, 1),
            (MouseButton::Forward, true) => (MOUSEEVENTF_XDOWN, 2),
            (MouseButton::Forward, false) => (MOUSEEVENTF_XUP, 2),
        };
        mouse(flags, data)
    }

    fn pressed(virtual_key: u16) -> bool {
        unsafe { GetAsyncKeyState(i32::from(virtual_key)) as u16 & 0x8000 != 0 }
    }

    pub fn polled_mouse_buttons() -> [bool; 3] {
        [
            pressed(VK_MBUTTON),
            pressed(VK_XBUTTON1),
            pressed(VK_XBUTTON2),
        ]
    }

    pub fn modifier_mask() -> u8 {
        let mut modifiers = 0;
        if pressed(VK_CONTROL) {
            modifiers |= MOD_CONTROL;
        }
        if pressed(VK_SHIFT) {
            modifiers |= MOD_SHIFT;
        }
        if pressed(VK_MENU) {
            modifiers |= MOD_ALT;
        }
        if pressed(VK_LWIN) || pressed(VK_RWIN) {
            modifiers |= MOD_SUPER;
        }
        modifiers
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{KeyboardInput, MouseButton};

    pub fn keyboard(_key: KeyboardInput, _down: bool) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
    pub fn mouse_move(_dx: i32, _dy: i32) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
    pub fn right_mouse(_down: bool) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
    pub fn mouse_button(_button: MouseButton, _down: bool) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
    pub fn polled_mouse_buttons() -> [bool; 3] {
        [false; 3]
    }
    pub fn modifier_mask() -> u8 {
        0
    }
    pub fn run_probes() -> Vec<super::InputProbeResult> {
        let probe = |probe: &str, label: &str| super::InputProbeResult {
            probe: probe.into(),
            label: label.into(),
            description: "键鼠输入自检仅支持 Windows".into(),
            ok: false,
            requested: 2,
            calls: 0,
            sent: 0,
            last_error_code: None,
            last_error: Some("键鼠输入自检仅支持 Windows".into()),
            foreground_process: None,
            integrity_level: None,
            observed_async_down: None,
            duration_ms: 0,
            timestamp: crate::diagnostics::iso8601_now(),
        };
        vec![
            probe("scan-code-w", "扫描码 W"),
            probe("virtual-key-w", "虚拟键 W"),
            probe("mouse-relative", "相对鼠标移动"),
        ]
    }
}

pub use platform::{modifier_mask, mouse_move, polled_mouse_buttons, right_mouse};

pub fn set(binding: InputBinding, down: bool) -> Result<(), String> {
    match binding {
        InputBinding::Keyboard(key) => platform::keyboard(key, down),
        InputBinding::Mouse(button) => platform::mouse_button(button, down),
    }
}

pub fn binding(code: &str) -> Option<InputBinding> {
    let keyboard = |scan_code| InputBinding::Keyboard(KeyboardInput::standard(scan_code));
    let extended = |scan_code| InputBinding::Keyboard(KeyboardInput::extended(scan_code));
    Some(match code {
        "MouseMiddle" => InputBinding::Mouse(MouseButton::Middle),
        "Mouse4" => InputBinding::Mouse(MouseButton::Back),
        "Mouse5" => InputBinding::Mouse(MouseButton::Forward),
        "Escape" => keyboard(0x01),
        "Digit1" => keyboard(0x02),
        "Digit2" => keyboard(0x03),
        "Digit3" => keyboard(0x04),
        "Digit4" => keyboard(0x05),
        "Digit5" => keyboard(0x06),
        "Digit6" => keyboard(0x07),
        "Digit7" => keyboard(0x08),
        "Digit8" => keyboard(0x09),
        "Digit9" => keyboard(0x0a),
        "Digit0" => keyboard(0x0b),
        "Minus" => keyboard(0x0c),
        "Equal" => keyboard(0x0d),
        "Backspace" => keyboard(0x0e),
        "Tab" => keyboard(0x0f),
        "KeyQ" => keyboard(0x10),
        "KeyW" => KEY_W,
        "KeyE" => keyboard(0x12),
        "KeyR" => keyboard(0x13),
        "KeyT" => keyboard(0x14),
        "KeyY" => keyboard(0x15),
        "KeyU" => keyboard(0x16),
        "KeyI" => keyboard(0x17),
        "KeyO" => keyboard(0x18),
        "KeyP" => keyboard(0x19),
        "BracketLeft" => keyboard(0x1a),
        "BracketRight" => keyboard(0x1b),
        "Enter" => keyboard(0x1c),
        "ControlLeft" => keyboard(0x1d),
        "ControlRight" => extended(0x1d),
        "KeyA" => KEY_A,
        "KeyS" => KEY_S,
        "KeyD" => KEY_D,
        "KeyF" => keyboard(0x21),
        "KeyG" => keyboard(0x22),
        "KeyH" => keyboard(0x23),
        "KeyJ" => keyboard(0x24),
        "KeyK" => keyboard(0x25),
        "KeyL" => keyboard(0x26),
        "Semicolon" => keyboard(0x27),
        "Quote" => keyboard(0x28),
        "Backquote" => keyboard(0x29),
        "ShiftLeft" => keyboard(0x2a),
        "Backslash" => keyboard(0x2b),
        "KeyZ" => keyboard(0x2c),
        "KeyX" => keyboard(0x2d),
        "KeyC" => keyboard(0x2e),
        "KeyV" => keyboard(0x2f),
        "KeyB" => keyboard(0x30),
        "KeyN" => keyboard(0x31),
        "KeyM" => keyboard(0x32),
        "Comma" => keyboard(0x33),
        "Period" => keyboard(0x34),
        "Slash" => keyboard(0x35),
        "NumpadDivide" => extended(0x35),
        "ShiftRight" => keyboard(0x36),
        "NumpadMultiply" => keyboard(0x37),
        "AltLeft" => keyboard(0x38),
        "AltRight" => extended(0x38),
        "Space" => keyboard(0x39),
        "CapsLock" => keyboard(0x3a),
        "F1" => keyboard(0x3b),
        "F2" => keyboard(0x3c),
        "F3" => keyboard(0x3d),
        "F4" => keyboard(0x3e),
        "F5" => keyboard(0x3f),
        "F6" => keyboard(0x40),
        "F7" => keyboard(0x41),
        "F8" => keyboard(0x42),
        "F9" => keyboard(0x43),
        "F10" => keyboard(0x44),
        "NumLock" => extended(0x45),
        "Numpad7" => keyboard(0x47),
        "Home" => extended(0x47),
        "Numpad8" => keyboard(0x48),
        "ArrowUp" => extended(0x48),
        "Numpad9" => keyboard(0x49),
        "PageUp" => extended(0x49),
        "NumpadSubtract" => keyboard(0x4a),
        "Numpad4" => keyboard(0x4b),
        "ArrowLeft" => extended(0x4b),
        "Numpad5" => keyboard(0x4c),
        "Numpad6" => keyboard(0x4d),
        "ArrowRight" => extended(0x4d),
        "NumpadAdd" => keyboard(0x4e),
        "Numpad1" => keyboard(0x4f),
        "End" => extended(0x4f),
        "Numpad2" => keyboard(0x50),
        "ArrowDown" => extended(0x50),
        "Numpad3" => keyboard(0x51),
        "PageDown" => extended(0x51),
        "Numpad0" => keyboard(0x52),
        "Insert" => extended(0x52),
        "NumpadDecimal" => keyboard(0x53),
        "Delete" => extended(0x53),
        "F11" => keyboard(0x57),
        "F12" => keyboard(0x58),
        "NumpadEqual" => keyboard(0x59),
        "NumpadEnter" => extended(0x1c),
        "MetaLeft" => extended(0x5b),
        "MetaRight" => extended(0x5c),
        _ => return None,
    })
}

pub fn release_all(configured_inputs: &[InputBinding]) {
    for binding in [KEY_W, KEY_A, KEY_S, KEY_D]
        .into_iter()
        .chain(configured_inputs.iter().copied())
    {
        let _ = set(binding, false);
    }
    let _ = right_mouse(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_codes_map_to_keyboard_and_mouse_inputs() {
        assert_eq!(
            binding("KeyE"),
            Some(InputBinding::Keyboard(KeyboardInput::standard(0x12)))
        );
        assert_eq!(
            binding("Digit2"),
            Some(InputBinding::Keyboard(KeyboardInput::standard(0x03)))
        );
        assert_eq!(
            binding("ArrowUp"),
            Some(InputBinding::Keyboard(KeyboardInput::extended(0x48)))
        );
        assert_eq!(
            binding("Mouse4"),
            Some(InputBinding::Mouse(MouseButton::Back))
        );
    }

    #[test]
    fn unknown_browser_codes_are_rejected() {
        assert_eq!(binding("BrowserBack"), None);
    }

    #[test]
    fn send_outcome_is_ok_only_when_every_request_succeeds() {
        let mut outcome = SendOutcome {
            requested: 2,
            ..SendOutcome::default()
        };
        outcome.record_call(1, 0);
        outcome.record_call(1, 0);
        assert_eq!(outcome.calls, 2);
        assert_eq!(outcome.sent, 2);
        assert_eq!(outcome.last_error_code, None);
        assert!(outcome.ok());
    }

    #[test]
    fn send_outcome_keeps_first_error_code_and_fails() {
        let mut outcome = SendOutcome {
            requested: 2,
            ..SendOutcome::default()
        };
        outcome.record_call(1, 0);
        outcome.record_call(0, 5);
        outcome.record_call(0, 998);
        assert_eq!(outcome.calls, 3);
        assert_eq!(outcome.sent, 1);
        assert_eq!(outcome.last_error_code, Some(5));
        assert!(!outcome.ok());
    }

    #[test]
    fn send_outcome_with_zero_error_code_is_not_a_failure() {
        let mut outcome = SendOutcome {
            requested: 2,
            ..SendOutcome::default()
        };
        outcome.record_call(1, 0);
        outcome.record_call(1, 0);
        assert!(outcome.ok());
    }

    #[test]
    fn probe_results_serialize_to_camel_case() {
        let probe = InputProbeResult {
            probe: "scan-code-w".into(),
            label: "扫描码 W".into(),
            description: "测试".into(),
            ok: true,
            requested: 2,
            calls: 2,
            sent: 2,
            last_error_code: None,
            last_error: None,
            foreground_process: Some("destiny2.exe".into()),
            integrity_level: Some("High (0x3000)".into()),
            observed_async_down: Some(true),
            duration_ms: 2,
            timestamp: "2026-01-01T00:00:00.000Z".into(),
        };
        let raw = serde_json::to_string(&probe).unwrap();
        assert!(!raw.contains("last_error_code"));
        assert!(raw.contains("\"foregroundProcess\":\"destiny2.exe\""));
        assert!(raw.contains("\"integrityLevel\":\"High (0x3000)\""));
        assert!(raw.contains("\"observedAsyncDown\":true"));
        assert!(raw.contains("\"durationMs\":2"));
    }

    /// 真实注入冒烟测试（仅 Windows，注入一次极短 W 与 ±1 像素鼠标移动）。
    /// CI 中默认跳过；本地手动验收：cargo test -- --ignored
    #[test]
    #[ignore = "注入真实键鼠输入，仅本地手动验收运行"]
    #[cfg(windows)]
    fn probes_execute_on_windows_without_crashing() {
        let probes = run_probes();
        assert_eq!(probes.len(), 3);
        assert_eq!(
            probes.iter().map(|probe| probe.probe.as_str()).collect::<Vec<_>>(),
            ["scan-code-w", "virtual-key-w", "mouse-relative"]
        );
        for probe in &probes {
            assert_eq!(probe.requested, 2, "{} 的请求数应为 2", probe.label);
            assert_eq!(probe.calls, 2, "{} 的调用次数应为 2", probe.label);
            assert!(!probe.label.is_empty());
            assert!(!probe.timestamp.is_empty());
        }
    }
}
