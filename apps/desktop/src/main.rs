use crane_application::Library;
use crane_desktop::app::CraneApp;
use crane_desktop::startup::{startup_mode_from_args, StartupMode};
use crane_platform::{LibraryGuard, NativeDialogs, SystemEnvironment};
use crane_storage::{load_settings, prepare_cutover, FilesystemTransfers, JsonRepository};
use eframe::egui;
use std::fs::{self, OpenOptions};
use std::path::Path;

const PREVIEW_MARKER: &str = ".crane-preview-fixture";

fn main() {
    if let Err(error) = run() {
        eprintln!("Crane could not start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mode = startup_mode_from_args(std::env::args().skip(1))?;
    let (root, is_preview, _guard) = match &mode {
        StartupMode::Preview(root) => {
            prepare_preview_root(root)?;
            (root.clone(), true, None)
        }
        StartupMode::Production => {
            let root =
                crane_platform::resolve_legacy_data_dir().map_err(|error| error.to_string())?;
            let guard = LibraryGuard::acquire(&root).map_err(|error| error.to_string())?;
            (root, false, Some(guard))
        }
    };

    let mut repository = JsonRepository::new(root.join("notes.json"));
    let mut transfers = FilesystemTransfers::new(
        Box::new(NativeDialogs),
        root.join("backups"),
        env!("CARGO_PKG_VERSION"),
    );
    let mut environment = SystemEnvironment::new();
    if !is_preview {
        prepare_cutover(&mut repository, &mut transfers, &mut environment)
            .map_err(|error| error.message)?;
    }
    let library = Library::open(Box::new(repository), Box::new(environment))
        .map_err(|error| error.message)?
        .with_transfers(Box::new(transfers));
    let snapshot = library.snapshot();
    let settings_path = root.join("settings.json");
    let settings = load_settings(&settings_path)
        .map_err(|error| format!("Could not load native settings: {error}"))?;

    let (title, app_id) = if is_preview {
        ("Crane — Native Preview", "app.crane.desktop.preview")
    } else {
        ("Crane", "app.crane.desktop")
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([1240.0, 780.0])
            .with_min_inner_size([720.0, 560.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        app_id,
        options,
        Box::new(move |creation| {
            if let Some(render_state) = &creation.wgpu_render_state {
                let adapter = render_state.adapter.get_info();
                eprintln!(
                    "Crane renderer initialized: {} ({:?}, {:?})",
                    adapter.name, adapter.backend, adapter.device_type
                );
            }
            Ok(Box::new(CraneApp::new_with_settings(
                library,
                snapshot,
                settings,
                settings_path,
                creation.egui_ctx.clone(),
            )))
        }),
    )
    .map_err(|error| error.to_string())
}

fn prepare_preview_root(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("Could not create the preview data folder: {error}"))?;
    let marker = root.join(PREVIEW_MARKER);
    if !marker.exists() {
        let mut entries = fs::read_dir(root)
            .map_err(|error| format!("Could not inspect the preview data folder: {error}"))?;
        if entries.next().is_some() {
            return Err(
                "The preview folder contains data without Crane's fixture marker; refusing to use it."
                    .into(),
            );
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(marker)
            .map_err(|error| format!("Could not mark the preview data folder: {error}"))?;
    }
    Ok(())
}
