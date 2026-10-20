use crate::PlatformError;

pub fn open_http_link(url: &str) -> Result<(), PlatformError> {
    validate_http_link(url)?;
    open(url)
}

pub fn validate_http_link(url: &str) -> Result<(), PlatformError> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(PlatformError::new(
            "Crane only opens explicit HTTP or HTTPS links.",
        ));
    }
    if url.chars().any(char::is_control) {
        return Err(PlatformError::new("The link contains control characters."));
    }
    Ok(())
}

#[cfg(windows)]
fn open(url: &str) -> Result<(), PlatformError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = OsStr::new(url).encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as usize <= 32 {
        return Err(PlatformError::new("Windows could not open the link."));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn open(url: &str) -> Result<(), PlatformError> {
    open_with("open", url)
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn open(url: &str) -> Result<(), PlatformError> {
    open_with("xdg-open", url)
}

#[cfg(not(windows))]
fn open_with(program: &str, url: &str) -> Result<(), PlatformError> {
    std::process::Command::new(program)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| PlatformError::new(format!("Could not open the link: {error}")))
}

#[cfg(test)]
mod tests {
    use super::validate_http_link;

    #[test]
    fn only_http_schemes_are_accepted() {
        assert!(validate_http_link("https://example.com").is_ok());
        assert!(validate_http_link("HTTP://example.com").is_ok());
        assert!(validate_http_link("file:///tmp/note").is_err());
        assert!(validate_http_link("javascript:alert(1)").is_err());
    }
}
