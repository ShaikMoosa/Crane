use crane_platform::PlatformError;
use crane_storage::{
    load_settings, save_settings, save_settings_with, NativeSettings, ThemeChoice,
};
use std::path::PathBuf;

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "crane-settings-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn settings_round_trip_and_default_when_missing() {
    let root = temp_root("round-trip");
    let path = root.join("settings.json");
    assert_eq!(load_settings(&path).unwrap(), NativeSettings::default());

    let settings = NativeSettings {
        schema_version: 1,
        theme: ThemeChoice::Light,
        reduced_motion: true,
    };
    save_settings(&path, &settings).unwrap();
    assert_eq!(load_settings(&path).unwrap(), settings);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsupported_settings_schema_is_rejected() {
    let root = temp_root("unsupported");
    let path = root.join("settings.json");
    std::fs::write(
        &path,
        r#"{"schemaVersion":99,"theme":"dark","reducedMotion":false}"#,
    )
    .unwrap();
    let error = load_settings(&path).unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported settings version 99"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_settings_write_retains_the_previous_file() {
    let root = temp_root("failed-write");
    let path = root.join("settings.json");
    let original = NativeSettings::default();
    save_settings(&path, &original).unwrap();
    let before = std::fs::read(&path).unwrap();

    let changed = NativeSettings {
        theme: ThemeChoice::Light,
        ..original.clone()
    };
    assert!(save_settings_with(&path, &changed, |_, _| {
        Err(PlatformError::new("synthetic replacement failure"))
    })
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(load_settings(&path).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}
