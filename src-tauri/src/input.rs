#[cfg(windows)]
mod platform {
    use std::mem::size_of;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEINPUT,
    };

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

    pub fn key(scan_code: u16, down: bool) -> Result<(), String> {
        let flags = KEYEVENTF_SCANCODE | if down { 0 } else { KEYEVENTF_KEYUP };
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: scan_code,
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
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: if down {
                        MOUSEEVENTF_RIGHTDOWN
                    } else {
                        MOUSEEVENTF_RIGHTUP
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        send(&input)
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn key(_scan_code: u16, _down: bool) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
    pub fn mouse_move(_dx: i32, _dy: i32) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
    pub fn right_mouse(_down: bool) -> Result<(), String> {
        Err("键鼠执行仅支持 Windows".into())
    }
}

pub use platform::{key, mouse_move, right_mouse};

pub const KEY_W: u16 = 0x11;
pub const KEY_A: u16 = 0x1e;
pub const KEY_S: u16 = 0x1f;
pub const KEY_D: u16 = 0x20;

pub fn scan_code(code: &str) -> Option<u16> {
    Some(match code {
        "Escape" => 0x01,
        "Digit1" => 0x02,
        "Digit2" => 0x03,
        "Digit3" => 0x04,
        "Digit4" => 0x05,
        "Digit5" => 0x06,
        "Digit6" => 0x07,
        "Digit7" => 0x08,
        "Digit8" => 0x09,
        "Digit9" => 0x0a,
        "Digit0" => 0x0b,
        "Minus" => 0x0c,
        "Equal" => 0x0d,
        "Backspace" => 0x0e,
        "Tab" => 0x0f,
        "KeyQ" => 0x10,
        "KeyW" => KEY_W,
        "KeyE" => 0x12,
        "KeyR" => 0x13,
        "KeyT" => 0x14,
        "KeyY" => 0x15,
        "KeyU" => 0x16,
        "KeyI" => 0x17,
        "KeyO" => 0x18,
        "KeyP" => 0x19,
        "BracketLeft" => 0x1a,
        "BracketRight" => 0x1b,
        "Enter" => 0x1c,
        "ControlLeft" => 0x1d,
        "KeyA" => KEY_A,
        "KeyS" => KEY_S,
        "KeyD" => KEY_D,
        "KeyF" => 0x21,
        "KeyG" => 0x22,
        "KeyH" => 0x23,
        "KeyJ" => 0x24,
        "KeyK" => 0x25,
        "KeyL" => 0x26,
        "Semicolon" => 0x27,
        "Quote" => 0x28,
        "Backquote" => 0x29,
        "ShiftLeft" => 0x2a,
        "Backslash" => 0x2b,
        "KeyZ" => 0x2c,
        "KeyX" => 0x2d,
        "KeyC" => 0x2e,
        "KeyV" => 0x2f,
        "KeyB" => 0x30,
        "KeyN" => 0x31,
        "KeyM" => 0x32,
        "Comma" => 0x33,
        "Period" => 0x34,
        "Slash" => 0x35,
        "ShiftRight" => 0x36,
        "AltLeft" => 0x38,
        "Space" => 0x39,
        "CapsLock" => 0x3a,
        "F1" => 0x3b,
        "F2" => 0x3c,
        "F3" => 0x3d,
        "F4" => 0x3e,
        "F5" => 0x3f,
        "F6" => 0x40,
        "F7" => 0x41,
        "F8" => 0x42,
        "F9" => 0x43,
        "F10" => 0x44,
        "F11" => 0x57,
        "F12" => 0x58,
        _ => return None,
    })
}

pub fn release_all(configured_keys: &[u16]) {
    for scan_code in [KEY_W, KEY_A, KEY_S, KEY_D]
        .into_iter()
        .chain(configured_keys.iter().copied())
    {
        let _ = key(scan_code, false);
    }
    let _ = right_mouse(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_key_codes_map_to_windows_set_one_scan_codes() {
        assert_eq!(scan_code("KeyE"), Some(0x12));
        assert_eq!(scan_code("Digit2"), Some(0x03));
        assert_eq!(scan_code("ShiftLeft"), Some(0x2a));
        assert_eq!(scan_code("ArrowUp"), None);
    }
}
