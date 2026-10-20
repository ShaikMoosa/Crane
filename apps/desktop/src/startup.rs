use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupMode {
    Production,
    Preview(PathBuf),
}

pub fn startup_mode_from_args(
    mut args: impl Iterator<Item = String>,
) -> Result<StartupMode, String> {
    match (args.next(), args.next(), args.next()) {
        (None, None, None) => Ok(StartupMode::Production),
        (Some(flag), Some(path), None)
            if flag == "--preview-data-dir" && !path.trim().is_empty() =>
        {
            Ok(StartupMode::Preview(PathBuf::from(path)))
        }
        _ => Err("Usage: crane [--preview-data-dir <isolated-folder>]".into()),
    }
}
