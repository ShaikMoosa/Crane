use crane_desktop::icons::{icon_asset, IconId, ALL_ICONS};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::path::PathBuf;

fn manifest_text() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons/lucide/manifest.json");
    std::fs::read_to_string(&path).expect("manifest must exist")
}

#[test]
fn manifest_matches_embedded_subset_with_no_unused_icons() {
    let manifest = manifest_text();
    assert!(manifest.contains("\"lucideVersion\": \"1.60.0\""));
    for asset in ALL_ICONS {
        assert!(
            manifest.contains(&format!("\"file\": \"{}\"", asset.file)),
            "manifest missing {}",
            asset.file
        );
        assert!(!asset.sha256.is_empty() && asset.sha256.len() == 64);
        assert!(asset
            .sha256
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        let bytes = icon_asset(asset.id).bytes;
        assert!(bytes.starts_with(b"<svg"), "{} is not an svg", asset.file);
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains("href=\"http") && !text.contains("src=\"http"));
    }
    // No unused vendored files: manifest entries == catalog entries.
    assert_eq!(manifest.matches("\"file\":").count(), ALL_ICONS.len());
    let notices = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../THIRD_PARTY_NOTICES.md"),
    )
    .expect("notices must exist");
    assert!(notices.contains("ISC License"));
    for asset in ALL_ICONS.iter().filter(|a| a.feather_derived) {
        assert!(
            notices.contains(asset.lucide_name),
            "notice missing {}",
            asset.lucide_name
        );
    }
    let _ = [
        IconId::Plus,
        IconId::Ellipsis,
        IconId::Settings,
        IconId::DatabaseBackup,
    ];
}

#[test]
fn icon_button_has_accessible_name() {
    let mut harness = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, _state| {
            crane_desktop::icons::icon_button(ui, IconId::Plus, "New note", "New note");
        },
        (),
    );
    harness.run();
    harness.get_by_label("New note");
}
