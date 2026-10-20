use crate::PlatformError;
use std::path::{Path, PathBuf};

const MAX_LOCAL_FONT_BYTES: u64 = 32 * 1024 * 1024;

/// Font families Crane may request from the operating system. Crane does not
/// copy, bundle, or redistribute these local font files.
pub const PERMITTED_LOCAL_FONT_FAMILIES: &[&str] =
    &["Georgia", "Times New Roman", "Segoe UI", "Arial"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalFontPaths {
    pub serif: Option<PathBuf>,
    pub interface: Option<PathBuf>,
}

pub fn permitted_local_font_families() -> &'static [&'static str] {
    PERMITTED_LOCAL_FONT_FAMILIES
}

pub fn local_font_paths() -> LocalFontPaths {
    LocalFontPaths {
        serif: first_existing(&serif_candidates()),
        interface: first_existing(&interface_candidates()),
    }
}

pub fn load_permitted_local_font(path: &Path) -> Result<Vec<u8>, PlatformError> {
    let permitted = serif_candidates()
        .into_iter()
        .chain(interface_candidates())
        .any(|candidate| candidate == path);
    if !permitted {
        return Err(PlatformError::new(
            "The requested font is outside Crane's local font allowlist.",
        ));
    }
    let metadata = std::fs::metadata(path).map_err(|error| {
        PlatformError::new(format!("Could not inspect the local font: {error}"))
    })?;
    if metadata.len() > MAX_LOCAL_FONT_BYTES {
        return Err(PlatformError::new("The local font is unexpectedly large."));
    }
    std::fs::read(path)
        .map_err(|error| PlatformError::new(format!("Could not read the local font: {error}")))
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

#[cfg(windows)]
fn fonts_dir() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("Fonts")
}

#[cfg(windows)]
fn serif_candidates() -> Vec<PathBuf> {
    let root = fonts_dir();
    ["georgia.ttf", "times.ttf"]
        .into_iter()
        .map(|name| root.join(name))
        .collect()
}

#[cfg(windows)]
fn interface_candidates() -> Vec<PathBuf> {
    let root = fonts_dir();
    ["segoeui.ttf", "arial.ttf"]
        .into_iter()
        .map(|name| root.join(name))
        .collect()
}

#[cfg(target_os = "macos")]
fn serif_candidates() -> Vec<PathBuf> {
    vec![PathBuf::from("/System/Library/Fonts/NewYork.ttf")]
}

#[cfg(target_os = "macos")]
fn interface_candidates() -> Vec<PathBuf> {
    vec![PathBuf::from("/System/Library/Fonts/SFNS.ttf")]
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn serif_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/liberation2/LiberationSerif-Regular.ttf"),
    ]
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn interface_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf"),
    ]
}

#[cfg(test)]
mod tests {
    use super::load_permitted_local_font;
    use std::path::Path;

    #[test]
    fn arbitrary_font_paths_are_rejected() {
        assert!(load_permitted_local_font(Path::new("not-a-system-font.ttf")).is_err());
    }
}
