use std::path::Path;

/// Moves explicitly dropped regular files to the platform trash.
///
/// The paths are used only for this operation. They are not persisted or logged.
pub fn trash_paths(paths: &[String]) -> Result<u32, String> {
    if paths.is_empty() {
        return Err("드롭된 파일이 없습니다.".to_string());
    }

    for path in paths {
        let candidate = Path::new(path);
        if !candidate.is_file() {
            return Err("파일만 휴지통으로 이동할 수 있습니다.".to_string());
        }
    }

    platform::trash_paths(paths)
}

#[cfg(windows)]
mod platform {
    use std::iter;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    pub fn trash_paths(paths: &[String]) -> Result<u32, String> {
        let mut from = paths
            .iter()
            .flat_map(|path| {
                Path::new(path)
                    .as_os_str()
                    .encode_wide()
                    .chain(iter::once(0))
            })
            .collect::<Vec<_>>();
        from.push(0);

        let mut operation = SHFILEOPSTRUCTW {
            hwnd: Default::default(),
            wFunc: FO_DELETE,
            pFrom: PCWSTR(from.as_ptr()),
            pTo: PCWSTR::null(),
            fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
            fAnyOperationsAborted: false.into(),
            hNameMappings: std::ptr::null_mut(),
            lpszProgressTitle: PCWSTR::null(),
        };

        let result = unsafe { SHFileOperationW(&mut operation) };
        if result != 0 {
            return Err("파일을 휴지통으로 이동하지 못했습니다.".to_string());
        }
        if operation.fAnyOperationsAborted.as_bool() {
            return Err("파일 이동이 취소되었습니다.".to_string());
        }

        Ok(paths.len() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::trash_paths;

    #[test]
    fn rejects_empty_drop() {
        assert!(trash_paths(&[]).is_err());
    }

    #[test]
    fn rejects_missing_or_directory_paths_before_platform_call() {
        assert!(trash_paths(&["C:\\path-that-does-not-exist\\file.txt".to_string()]).is_err());
        assert!(trash_paths(&[".".to_string()]).is_err());
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2_foundation::{NSFileManager, NSString, NSURL};

    pub fn trash_paths(paths: &[String]) -> Result<u32, String> {
        let manager = NSFileManager::defaultManager();
        let mut moved = 0;

        for path in paths {
            let path = NSString::from_str(path);
            let url = NSURL::fileURLWithPath(&path);
            let mut resulting_url = None;
            manager
                .trashItemAtURL_resultingItemURL_error(&url, Some(&mut resulting_url))
                .map_err(|_| "파일을 휴지통으로 이동하지 못했습니다.".to_string())?;
            moved += 1;
        }

        Ok(moved)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    pub fn trash_paths(_paths: &[String]) -> Result<u32, String> {
        Err("현재 운영체제에서는 휴지통 이동을 지원하지 않습니다.".to_string())
    }
}
