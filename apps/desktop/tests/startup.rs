use crane_desktop::startup::{startup_mode_from_args, StartupMode};
use std::path::PathBuf;

fn mode(args: &[&str]) -> Result<StartupMode, String> {
    startup_mode_from_args(args.iter().map(|value| value.to_string()))
}

#[test]
fn no_arguments_selects_production() {
    assert_eq!(mode(&[]).unwrap(), StartupMode::Production);
}

#[test]
fn preview_flag_selects_isolated_preview_root() {
    assert_eq!(
        mode(&["--preview-data-dir", "artifacts/preview"]).unwrap(),
        StartupMode::Preview(PathBuf::from("artifacts/preview"))
    );
}

#[test]
fn missing_preview_path_is_rejected() {
    assert!(mode(&["--preview-data-dir"]).is_err());
}

#[test]
fn blank_preview_path_is_rejected() {
    assert!(mode(&["--preview-data-dir", "  "]).is_err());
}

#[test]
fn unknown_or_extra_arguments_are_rejected() {
    assert!(mode(&["--production"]).is_err());
    assert!(mode(&["--preview-data-dir", "a", "extra"]).is_err());
}
