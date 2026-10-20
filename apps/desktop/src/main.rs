use crane_application::Library;
use crane_desktop::app::CraneApp;
use crane_platform::SystemEnvironment;
use crane_storage::{load_settings, JsonRepository};
use eframe::egui;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

const PREVIEW_MARKER: &str = ".crane-preview-fixture";

fn main() {
    if let Err(error) = run() {
        eprintln!("Crane could not start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = preview_root_from_args(std::env::args().skip(1))?;
    prepare_preview_root(&root)?;
    let repository = JsonRepository::new(root.join("notes.json"));
    let library = Library::open(Box::new(repository), Box::new(SystemEnvironment::new()))
        .map_err(|error| error.message)?;
    let snapshot = library.snapshot();
    let settings_path = root.join("settings.json");
    let settings = load_settings(&settings_path)
        .map_err(|error| format!("Could not load native settings: {error}"))?;

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Crane — Native Preview")
            .with_inner_size([1240.0, 780.0])
            .with_min_inner_size([720.0, 560.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "app.crane.desktop.preview",
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

fn preview_root_from_args(mut args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--preview-data-dir"), Some(path), None) if !path.trim().is_empty() => {
            Ok(PathBuf::from(path))
        }
        _ => Err(
            "Use --preview-data-dir <isolated-folder>; production libraries are disabled in this preview."
                .into(),
        ),
    }
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
