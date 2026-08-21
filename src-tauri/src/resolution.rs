use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionInfo {
    pub width: u32,
    pub height: u32,
    pub detected_game: bool,
    pub source: &'static str,
    pub window_title: Option<String>,
    pub dpi: Option<u32>,
}

#[cfg(windows)]
mod platform {
    use super::ResolutionInfo;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::{
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                EnumWindows, GetClientRect, GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW,
                IsWindowVisible, SM_CXSCREEN, SM_CYSCREEN,
            },
        },
    };

    #[derive(Debug)]
    struct WindowMatch {
        hwnd: HWND,
        title: String,
    }

    unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let length = GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return 1;
        }
        let mut buffer = vec![0_u16; length as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        if copied <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&buffer[..copied as usize]);
        if title.to_ascii_lowercase().contains("destiny 2") {
            let slot = &mut *(lparam as *mut Option<WindowMatch>);
            *slot = Some(WindowMatch { hwnd, title });
            return 0;
        }
        1
    }

    pub fn detect() -> ResolutionInfo {
        let mut found: Option<WindowMatch> = None;
        unsafe {
            EnumWindows(Some(collect_window), &mut found as *mut _ as LPARAM);
        }
        if let Some(window) = found {
            let mut rect = RECT::default();
            if unsafe { GetClientRect(window.hwnd, &mut rect) } != 0 {
                let width = (rect.right - rect.left).max(0) as u32;
                let height = (rect.bottom - rect.top).max(0) as u32;
                if width > 0 && height > 0 {
                    let dpi = unsafe { GetDpiForWindow(window.hwnd) };
                    return ResolutionInfo {
                        width,
                        height,
                        detected_game: true,
                        source: "destiny-window",
                        window_title: Some(window.title),
                        dpi: (dpi > 0).then_some(dpi),
                    };
                }
            }
        }
        ResolutionInfo {
            width: unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(0) as u32,
            height: unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(0) as u32,
            detected_game: false,
            source: "primary-display",
            window_title: None,
            dpi: None,
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::ResolutionInfo;

    pub fn detect() -> ResolutionInfo {
        ResolutionInfo {
            width: 1920,
            height: 1080,
            detected_game: false,
            source: "primary-display",
            window_title: None,
            dpi: None,
        }
    }
}

pub use platform::detect;
