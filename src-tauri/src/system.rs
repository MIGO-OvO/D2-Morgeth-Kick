/// 完整性级别 RID -> 可读标签（TokenIntegrityLevel）。
pub fn integrity_label(rid: u32) -> String {
    let label = match rid & 0xF000 {
        0x0000 => "Untrusted",
        0x1000 => "Low",
        0x2000 => "Medium",
        0x3000 => "High",
        0x4000 => "System",
        0x5000 => "Protected",
        _ => "Unknown",
    };
    format!("{label} (0x{rid:04X})")
}

#[cfg(windows)]
mod platform {
    use super::integrity_label;
    use std::path::PathBuf;
    use windows_sys::core::GUID;
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree, HANDLE, HWND},
        Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
            PSID, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        },
        System::{
            Diagnostics::Debug::{
                FormatMessageW, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
                FORMAT_MESSAGE_IGNORE_INSERTS,
            },
            SystemInformation::OSVERSIONINFOW,
            Threading::{
                OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Shell::SHGetKnownFolderPath,
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    };

    // FOLDERID_Downloads = {374DE290-123F-4565-9164-39C4925E467B}
    const FOLDERID_DOWNLOADS: GUID = GUID {
        data1: 0x374DE290,
        data2: 0x123F,
        data3: 0x4565,
        data4: [0x91, 0x64, 0x39, 0xC4, 0x92, 0x5E, 0x46, 0x7B],
    };

    fn process_id_of_window(hwnd: HWND) -> Option<u32> {
        let mut process_id = 0;
        if unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) } == 0 {
            return None;
        }
        Some(process_id)
    }

    fn process_name_for_pid(process_id: u32) -> Option<String> {
        if process_id == 0 {
            return None;
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0_u16; 32_768];
        let mut length = buffer.len() as u32;
        let queried =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
        unsafe { CloseHandle(process) };
        if queried == 0 || length == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        std::path::Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }

    pub fn foreground_process_name() -> Option<String> {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.is_null() {
            return None;
        }
        process_name_for_pid(process_id_of_window(foreground)?)
    }

    fn token_integrity_rid(token: HANDLE) -> Option<u32> {
        let mut buffer = vec![0_u8; 128];
        let mut returned = 0;
        let queried = unsafe {
            GetTokenInformation(
                token,
                TokenIntegrityLevel,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut returned,
            )
        };
        if queried == 0 || returned == 0 {
            return None;
        }
        let label = unsafe { &*(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
        let sid: PSID = label.Label.Sid;
        if sid.is_null() {
            return None;
        }
        let sub_authority_count = unsafe { *GetSidSubAuthorityCount(sid) } as u32;
        if sub_authority_count == 0 {
            return None;
        }
        Some(unsafe { *GetSidSubAuthority(sid, sub_authority_count - 1) })
    }

    /// 前台进程完整性级别（用于解释 SendInput 是否受 UIPI 阻断；只读诊断信息）。
    pub fn foreground_integrity_level() -> Option<String> {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.is_null() {
            return None;
        }
        let process_id = process_id_of_window(foreground)?;
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return None;
        }
        let mut token: HANDLE = std::ptr::null_mut();
        let opened = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) };
        unsafe { CloseHandle(process) };
        if opened == 0 {
            return None;
        }
        let rid = token_integrity_rid(token);
        unsafe { CloseHandle(token) };
        rid.map(integrity_label)
    }

    /// 通过 RtlGetVersion 读取真实 Windows 版本（GetVersionEx 会受清单影响）。
    pub fn os_version() -> Option<String> {
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            dwMajorVersion: 0,
            dwMinorVersion: 0,
            dwBuildNumber: 0,
            dwPlatformId: 0,
            szCSDVersion: [0; 128],
        };
        let status = unsafe { RtlGetVersion(&mut info) };
        if status < 0 {
            return None;
        }
        Some(format!(
            "{}.{}.{}",
            info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
        ))
    }

    /// Win32 错误码 -> 系统消息（中文系统返回中文，附错误码）。
    pub fn win32_error_message(code: u32) -> String {
        if code == 0 {
            return "无错误 (0)".into();
        }
        let mut buffer: windows_sys::core::PWSTR = std::ptr::null_mut();
        let length = unsafe {
            FormatMessageW(
                FORMAT_MESSAGE_ALLOCATE_BUFFER
                    | FORMAT_MESSAGE_FROM_SYSTEM
                    | FORMAT_MESSAGE_IGNORE_INSERTS,
                std::ptr::null(),
                code,
                0,
                &mut buffer as *mut *mut u16 as windows_sys::core::PWSTR,
                0,
                std::ptr::null(),
            )
        };
        let message = if length > 0 && !buffer.is_null() {
            let text = String::from_utf16_lossy(unsafe {
                std::slice::from_raw_parts(buffer, length as usize)
            });
            let _ = unsafe { LocalFree(buffer as *mut core::ffi::c_void) };
            text.trim_end_matches(['\r', '\n']).to_string()
        } else {
            std::io::Error::from_raw_os_error(code as i32).to_string()
        };
        format!("{message} ({code})")
    }

    /// 用户下载目录（SHGetKnownFolderPath；失败时回退 USERPROFILE\Downloads）。
    pub fn downloads_dir() -> Option<PathBuf> {
        let mut path: windows_sys::core::PWSTR = std::ptr::null_mut();
        let result = unsafe {
            SHGetKnownFolderPath(&FOLDERID_DOWNLOADS, 0, std::ptr::null_mut(), &mut path)
        };
        if result >= 0 && !path.is_null() {
            let text = String::from_utf16_lossy(unsafe {
                let mut length = 0;
                while *path.add(length) != 0 {
                    length += 1;
                }
                std::slice::from_raw_parts(path, length)
            });
            unsafe {
                windows_sys::Win32::System::Com::CoTaskMemFree(path as *const core::ffi::c_void)
            };
            let dir = PathBuf::from(text);
            if dir.is_dir() {
                return Some(dir);
            }
        }
        std::env::var_os("USERPROFILE").map(|profile| PathBuf::from(profile).join("Downloads"))
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::PathBuf;

    pub fn foreground_process_name() -> Option<String> {
        None
    }

    pub fn foreground_integrity_level() -> Option<String> {
        None
    }

    pub fn os_version() -> Option<String> {
        None
    }

    pub fn win32_error_message(_code: u32) -> String {
        format!("Win32 错误 ({_code})")
    }

    pub fn downloads_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads"))
    }
}

pub use platform::{
    downloads_dir, foreground_integrity_level, foreground_process_name, os_version,
    win32_error_message,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_labels_cover_standard_rids() {
        assert_eq!(integrity_label(0x0000), "Untrusted (0x0000)");
        assert_eq!(integrity_label(0x1000), "Low (0x1000)");
        assert_eq!(integrity_label(0x2000), "Medium (0x2000)");
        assert_eq!(integrity_label(0x3000), "High (0x3000)");
        assert_eq!(integrity_label(0x4000), "System (0x4000)");
        assert_eq!(integrity_label(0x5000), "Protected (0x5000)");
        assert_eq!(integrity_label(0x2400), "Medium (0x2400)");
        assert!(integrity_label(0x6A00).starts_with("Unknown"));
    }

    #[test]
    fn error_message_always_contains_the_code() {
        assert_eq!(win32_error_message(0), "无错误 (0)");
        assert!(win32_error_message(5).ends_with("(5)"));
        assert!(win32_error_message(998).ends_with("(998)"));
    }
}
