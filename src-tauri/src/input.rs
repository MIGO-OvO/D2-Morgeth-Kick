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

#[cfg(windows)]
mod platform {
    use std::mem::size_of;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_MIDDLEDOWN,
        MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, VK_CONTROL, VK_LWIN, VK_MBUTTON, VK_MENU,
        VK_RWIN, VK_SHIFT, VK_XBUTTON1, VK_XBUTTON2,
    };

    use super::{KeyboardInput, MouseButton, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_SUPER};

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
}
