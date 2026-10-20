mod atomic_replace;
mod dialogs;
mod environment;
mod fonts;
mod instance;
mod links;
mod paths;

use std::fmt::{Display, Formatter};

pub use atomic_replace::{replace_atomic, validate_destructive_path};
pub use dialogs::{DialogProvider, DialogSelection, NativeDialogs};
pub use environment::SystemEnvironment;
pub use fonts::{
    load_permitted_local_font, local_font_paths, permitted_local_font_families, LocalFontPaths,
    PERMITTED_LOCAL_FONT_FAMILIES,
};
pub use instance::LibraryGuard;
pub use links::{open_http_link, validate_http_link};
pub use paths::resolve_legacy_data_dir;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformError {
    message: String,
}

impl PlatformError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for PlatformError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PlatformError {}
