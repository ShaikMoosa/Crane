use crane_platform::{classify_legacy_process_paths, LegacyProcessStatus, PlatformError};
use std::path::PathBuf;

#[test]
fn another_crane_executable_is_classified_as_legacy_running() {
    let current = PathBuf::from(r"C:\Program Files\Crane\crane.exe");
    let processes = vec![
        Ok(current.clone()),
        Ok(PathBuf::from(r"C:\Legacy Crane\crane.exe")),
        Ok(PathBuf::from(r"C:\Windows\notepad.exe")),
    ];

    assert_eq!(
        classify_legacy_process_paths(&current, processes),
        LegacyProcessStatus::Running
    );
}

#[test]
fn unreadable_matching_process_is_unknown_and_blocks_safe_assumptions() {
    let current = PathBuf::from(r"C:\Program Files\Crane\crane.exe");
    let processes = vec![Err(PlatformError::new("access denied"))];

    assert_eq!(
        classify_legacy_process_paths(&current, processes),
        LegacyProcessStatus::Unknown
    );
}

#[test]
fn current_process_and_unrelated_executables_are_absent() {
    let current = PathBuf::from(r"C:\Program Files\Crane\crane.exe");
    let processes = vec![
        Ok(current.clone()),
        Ok(PathBuf::from(r"C:\Windows\notepad.exe")),
    ];

    assert_eq!(
        classify_legacy_process_paths(&current, processes),
        LegacyProcessStatus::Absent
    );
}
