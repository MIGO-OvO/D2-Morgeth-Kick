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

pub const KEY_2: u16 = 0x03;
pub const KEY_E: u16 = 0x12;
pub const KEY_W: u16 = 0x11;
pub const KEY_A: u16 = 0x1e;
pub const KEY_S: u16 = 0x1f;
pub const KEY_D: u16 = 0x20;
pub const KEY_F: u16 = 0x21;
pub const KEY_G: u16 = 0x22;
pub const KEY_X: u16 = 0x2d;
pub const KEY_C: u16 = 0x2e;
pub const KEY_LEFT_SHIFT: u16 = 0x2a;
pub const KEY_SPACE: u16 = 0x39;

pub fn release_all() {
    for scan_code in [
        KEY_W,
        KEY_A,
        KEY_S,
        KEY_D,
        KEY_LEFT_SHIFT,
        KEY_SPACE,
        KEY_E,
        KEY_2,
        KEY_X,
        KEY_C,
        KEY_F,
        KEY_G,
    ] {
        let _ = key(scan_code, false);
    }
    let _ = right_mouse(false);
}
