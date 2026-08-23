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
    use std::path::Path;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::{
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                EnumWindows, GetClientRect, GetForegroundWindow, GetSystemMetrics,
                GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
                IsWindowVisible, SM_CXSCREEN, SM_CYSCREEN,
            },
        },
    };

    #[derive(Debug, Clone, Copy)]
    pub struct GameClientArea {
        pub left: i32,
        pub top: i32,
        pub width: u32,
        pub height: u32,
    }

    #[derive(Debug)]
    struct WindowMatch {
        hwnd: HWND,
        title: String,
    }

    unsafe fn process_name(hwnd: HWND) -> Option<String> {
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0_u16; 32_768];
        let mut length = buffer.len() as u32;
        let queried = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
        CloseHandle(process);
        if queried == 0 || length == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }

    unsafe fn is_destiny_process(hwnd: HWND) -> bool {
        process_name(hwnd).is_some_and(|name| name.eq_ignore_ascii_case("destiny2.exe"))
    }

    unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        if IsIconic(hwnd) != 0 || !is_destiny_process(hwnd) {
            return 1;
        }
        let length = GetWindowTextLengthW(hwnd);
        let mut buffer = vec![0_u16; length as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        let title = (copied > 0)
            .then(|| String::from_utf16_lossy(&buffer[..copied as usize]))
            .unwrap_or_else(|| "Destiny 2".into());
        let slot = &mut *(lparam as *mut Option<WindowMatch>);
        *slot = Some(WindowMatch { hwnd, title });
        0
    }

    fn find_game_window() -> Option<WindowMatch> {
        let mut found: Option<WindowMatch> = None;
        unsafe {
            EnumWindows(Some(collect_window), &mut found as *mut _ as LPARAM);
        }
        found
    }

    fn client_area(hwnd: HWND) -> Option<GameClientArea> {
        let mut rect = RECT::default();
        let mut origin = POINT { x: 0, y: 0 };
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0
            || unsafe { ClientToScreen(hwnd, &mut origin) } == 0
        {
            return None;
        }
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;
        (width > 0 && height > 0).then_some(GameClientArea {
            left: origin.x,
            top: origin.y,
            width,
            height,
        })
    }

    pub fn active_game_client_area() -> Option<GameClientArea> {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.is_null() || unsafe { !is_destiny_process(foreground) } {
            return None;
        }
        client_area(foreground)
    }

    pub fn detect() -> ResolutionInfo {
        if let Some(window) = find_game_window() {
            if let Some(area) = client_area(window.hwnd) {
                let dpi = unsafe { GetDpiForWindow(window.hwnd) };
                return ResolutionInfo {
                    width: area.width,
                    height: area.height,
                    detected_game: true,
                    source: "destiny-process",
                    window_title: Some(window.title),
                    dpi: (dpi > 0).then_some(dpi),
                };
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

    #[derive(Debug, Clone, Copy)]
    pub struct GameClientArea {
        pub left: i32,
        pub top: i32,
        pub width: u32,
        pub height: u32,
    }

    pub fn active_game_client_area() -> Option<GameClientArea> {
        None
    }

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

pub use platform::{active_game_client_area, detect};
