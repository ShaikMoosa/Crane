use super::TransferError;
use crate::model::{extract_tags, validate_note, Note, Store};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILENAME_CHARS: usize = 120;

#[derive(Clone, Debug)]
pub struct NamedBytes {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ExportEntry {
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ExportManifest {
    pub entries: Vec<ExportEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub exported_count: usize,
    pub folder_name: String,
}

pub fn parse_markdown(
    name: &str,
    bytes: &[u8],
    timestamp: u64,
    id: String,
) -> Result<Note, TransferError> {
    ensure_markdown_extension(name)?;
    let source = std::str::from_utf8(bytes)
        .map_err(|_| TransferError::markdown(format!("{name} is not valid UTF-8 Markdown.")))?;
    let (heading, body) = split_title_heading(source);
    let title = heading.unwrap_or_else(|| filename_title(name));
    let note = Note {
        id,
        title,
        body: body.to_string(),
        tags: extract_tags(body),
        created_at: timestamp,
        updated_at: timestamp,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    };
    validate_note(&note)
        .map_err(|error| TransferError::markdown(format!("Could not import {name}: {error}")))?;
    Ok(note)
}

pub fn serialize_markdown(note: &Note) -> String {
    let title = if note.title.trim().is_empty() {
        "Untitled note"
    } else {
        note.title.as_str()
    };
    format!("# {title}\n\n{}", note.body)
}

pub fn suggested_markdown_filename(note: &Note) -> String {
    format!("{}.md", sanitize_filename_stem(&note.title))
}

pub fn prepare_import(
    files: Vec<NamedBytes>,
    timestamp: u64,
    mut ids: impl Iterator<Item = String>,
) -> Result<Vec<Note>, TransferError> {
    let total = files.iter().try_fold(0usize, |sum, file| {
        sum.checked_add(file.bytes.len())
            .ok_or_else(|| TransferError::markdown("The selected Markdown batch exceeds 64 MB."))
    })?;
    if total > MAX_BATCH_BYTES {
        return Err(TransferError::markdown(
            "The selected Markdown batch exceeds the 64 MB limit.",
        ));
    }

    files
        .into_iter()
        .map(|file| {
            let id = ids.next().ok_or_else(|| {
                TransferError::markdown("Crane could not assign an ID to every imported note.")
            })?;
            parse_markdown(&file.name, &file.bytes, timestamp, id)
        })
        .collect()
}

pub fn build_export_manifest(store: &Store) -> Result<ExportManifest, TransferError> {
    let mut used_notes = HashSet::new();
    let mut used_archive = HashSet::new();
    let mut entries = Vec::new();

    for note in store.notes.iter().filter(|note| !note.trashed) {
        validate_note(note).map_err(|error| {
            TransferError::markdown(format!("Could not export note {}: {error}", note.id))
        })?;
        let (folder, used) = if note.archived {
            ("Archive", &mut used_archive)
        } else {
            ("Notes", &mut used_notes)
        };
        let stem = unique_filename_stem(&note.title, used);
        entries.push(ExportEntry {
            relative_path: PathBuf::from(folder).join(format!("{stem}.md")),
            contents: serialize_markdown(note).into_bytes(),
        });
    }

    Ok(ExportManifest { entries })
}

pub fn write_note_export(path: &Path, note: &Note) -> Result<(), TransferError> {
    let parent = path
        .parent()
        .ok_or_else(|| TransferError::markdown("The export path has no parent folder."))?;
    fs::create_dir_all(parent).map_err(|error| {
        TransferError::markdown(format!("Could not create the export folder: {error}"))
    })?;
    let temporary = unique_temporary_path(parent, ".crane-note-export", "tmp");
    let result = (|| {
        FsEntryWriter.write(&temporary, serialize_markdown(note).as_bytes())?;
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| {
        TransferError::markdown(format!("Could not write the note export: {error}"))
    })
}

pub fn write_library_export(
    destination: &Path,
    manifest: &ExportManifest,
    folder_name: &str,
) -> Result<ExportSummary, TransferError> {
    write_library_export_with(destination, manifest, folder_name, &mut FsEntryWriter)
}

trait EntryWriter {
    fn write(&mut self, path: &Path, contents: &[u8]) -> io::Result<()>;
}

struct FsEntryWriter;

impl EntryWriter for FsEntryWriter {
    fn write(&mut self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(contents)?;
        file.sync_all()
    }
}

fn write_library_export_with(
    destination: &Path,
    manifest: &ExportManifest,
    folder_name: &str,
    writer: &mut impl EntryWriter,
) -> Result<ExportSummary, TransferError> {
    fs::create_dir_all(destination).map_err(|error| {
        TransferError::markdown(format!("Could not open the export destination: {error}"))
    })?;
    let final_path = destination.join(folder_name);
    if final_path.exists() {
        return Err(TransferError::markdown(format!(
            "An export named {folder_name} already exists."
        )));
    }
    let temporary = create_temporary_directory(destination)?;
    let result = (|| {
        for entry in &manifest.entries {
            ensure_safe_relative_path(&entry.relative_path)?;
            let path = temporary.join(&entry.relative_path);
            let parent = path.parent().ok_or_else(|| {
                TransferError::markdown("An export entry has no parent directory.")
            })?;
            fs::create_dir_all(parent).map_err(|error| {
                TransferError::markdown(format!("Could not create an export folder: {error}"))
            })?;
            writer.write(&path, &entry.contents).map_err(|error| {
                TransferError::markdown(format!("Could not write an exported note: {error}"))
            })?;
        }
        fs::rename(&temporary, &final_path).map_err(|error| {
            TransferError::markdown(format!("Could not publish the library export: {error}"))
        })?;
        Ok(ExportSummary {
            exported_count: manifest.entries.len(),
            folder_name: folder_name.to_string(),
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}

fn ensure_safe_relative_path(path: &Path) -> Result<(), TransferError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(TransferError::markdown(
            "An export entry has an unsafe path.",
        ));
    }
    Ok(())
}

fn create_temporary_directory(destination: &Path) -> Result<PathBuf, TransferError> {
    for attempt in 0..100u32 {
        let path = unique_temporary_path(
            destination,
            &format!(".crane-export-{}-{attempt}", std::process::id()),
            "tmp",
        );
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(TransferError::markdown(format!(
                    "Could not create a temporary export folder: {error}"
                )))
            }
        }
    }
    Err(TransferError::markdown(
        "Could not reserve a temporary export folder.",
    ))
}

fn unique_temporary_path(parent: &Path, prefix: &str, extension: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!("{prefix}-{nonce}.{extension}"))
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn ensure_markdown_extension(name: &str) -> Result<(), TransferError> {
    let is_markdown = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
    if is_markdown {
        Ok(())
    } else {
        Err(TransferError::markdown(format!(
            "{name} is not a Markdown (.md) file."
        )))
    }
}

fn filename_title(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("Untitled note")
        .to_string()
}

fn split_title_heading(source: &str) -> (Option<String>, &str) {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim().is_empty() {
            offset += line.len();
            continue;
        }
        let Some(title) = content.strip_prefix("# ") else {
            return (None, source);
        };
        if title.trim().is_empty() {
            return (None, source);
        }

        let after_heading = offset + line.len();
        let mut body = &source[after_heading..];
        if let Some(end) = body.find('\n') {
            if body[..=end].trim().is_empty() {
                body = &body[end + 1..];
            }
        } else if body.trim().is_empty() {
            body = "";
        }
        return (Some(title.trim().to_string()), body);
    }
    (None, source)
}

fn unique_filename_stem(title: &str, used: &mut HashSet<String>) -> String {
    let base = sanitize_filename_stem(title);
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while !used.insert(candidate.to_uppercase()) {
        let suffix_text = format!(" {suffix}");
        let available = MAX_FILENAME_CHARS.saturating_sub(suffix_text.chars().count());
        let prefix: String = base.chars().take(available).collect();
        candidate = format!("{prefix}{suffix_text}");
        suffix += 1;
    }
    candidate
}

fn sanitize_filename_stem(title: &str) -> String {
    let filtered: String = title
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
        .collect();
    let trimmed = filtered.trim().trim_end_matches([' ', '.']);
    let mut stem: String = trimmed.chars().take(MAX_FILENAME_CHARS).collect();
    if stem.is_empty() || is_windows_reserved_name(&stem) {
        stem = "Untitled note".into();
    }
    stem
}

fn is_windows_reserved_name(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or(stem).to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || base
            .strip_prefix("COM")
            .or_else(|| base.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(
                    number,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{extract_tags, Store, MAX_BODY_BYTES};
    use std::io;

    fn note(id: &str, title: &str, body: &str) -> Note {
        Note {
            id: id.into(),
            title: title.into(),
            body: body.into(),
            tags: extract_tags(body),
            created_at: 10,
            updated_at: 20,
            pinned: false,
            archived: false,
            trashed: false,
            extensions: Default::default(),
        }
    }

    #[test]
    fn round_trips_title_and_body() {
        let source = note("source", "Field notes", "A body with #work/field.\n\n- one");
        let encoded = serialize_markdown(&source);
        let parsed = parse_markdown("Field notes.md", encoded.as_bytes(), 42, "imported".into())
            .expect("exported Markdown should import");

        assert_eq!(parsed.title, source.title);
        assert_eq!(parsed.body, source.body);
        assert_eq!(parsed.tags, source.tags);
        assert_eq!(parsed.created_at, 42);
        assert_eq!(parsed.updated_at, 42);
        assert!(!parsed.pinned && !parsed.archived && !parsed.trashed);
    }

    #[test]
    fn uses_filename_when_h1_is_absent() {
        let parsed = parse_markdown(
            "Daily log.md",
            b"Intro\n\n#nested is not a heading",
            7,
            "daily".into(),
        )
        .expect("plain Markdown should import");

        assert_eq!(parsed.title, "Daily log");
        assert_eq!(parsed.body, "Intro\n\n#nested is not a heading");
    }

    #[test]
    fn rejects_non_utf8_and_oversized_inputs() {
        let invalid = parse_markdown("broken.md", &[0xff, 0xfe], 1, "bad".into())
            .expect_err("non-UTF-8 input must fail");
        assert!(invalid.to_string().contains("broken.md"));
        assert!(invalid.to_string().contains("UTF-8"));

        let oversized = vec![b'x'; MAX_BODY_BYTES + 1];
        let error = parse_markdown("large.md", &oversized, 1, "large".into())
            .expect_err("oversized body must fail");
        assert!(error.to_string().contains("large.md"));
        assert!(error.to_string().contains("2 MB"));
    }

    #[test]
    fn rejects_the_entire_batch_when_one_file_fails() {
        let files = vec![
            NamedBytes {
                name: "good.md".into(),
                bytes: b"# Good\n\nValid".to_vec(),
            },
            NamedBytes {
                name: "bad.txt".into(),
                bytes: b"Not Markdown".to_vec(),
            },
        ];

        let result = prepare_import(files, 3, ["one".into(), "two".into()].into_iter());
        assert!(result.is_err());
    }

    #[test]
    fn builds_notes_and_archive_manifests_without_trash() {
        let active = note("active", "Active", "Current");
        let mut archived = note("archived", "Archived", "Older");
        archived.archived = true;
        let mut trashed = note("trashed", "Trashed", "Discarded");
        trashed.trashed = true;
        let store = Store {
            schema_version: 1,
            notes: vec![active, archived, trashed],
            extensions: Default::default(),
        };

        let manifest = build_export_manifest(&store).expect("manifest should build");
        let paths: Vec<_> = manifest
            .entries
            .iter()
            .map(|entry| entry.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert_eq!(paths, ["Notes/Active.md", "Archive/Archived.md"]);
        assert!(manifest
            .entries
            .iter()
            .all(|entry| !String::from_utf8_lossy(&entry.contents).contains("Discarded")));
    }

    #[test]
    fn sanitizes_reserved_case_insensitive_filename_collisions() {
        let mut first = note("one", "CON", "Reserved");
        let second = note("two", "Plan", "Upper");
        let third = note("three", "plan", "Lower");
        let fourth = note("four", &"é".repeat(160), "Long");
        let fifth = note("five", "Σ", "Upper sigma");
        let sixth = note("six", "ς", "Final sigma");
        let seventh = note("seven", "COM¹", "Reserved superscript");
        first.archived = true;
        let store = Store {
            schema_version: 1,
            notes: vec![first, second, third, fourth, fifth, sixth, seventh],
            extensions: Default::default(),
        };

        let manifest = build_export_manifest(&store).expect("manifest should build");
        let stems: Vec<_> = manifest
            .entries
            .iter()
            .map(|entry| {
                entry
                    .relative_path
                    .file_stem()
                    .expect("entry should have a stem")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();

        assert_eq!(stems[0], "Untitled note");
        assert_eq!(stems[1], "Plan");
        assert_eq!(stems[2], "plan 2");
        assert!(stems[3].chars().count() <= MAX_FILENAME_CHARS);
        assert_eq!(stems[4], "Σ");
        assert_eq!(stems[5], "ς 2");
        assert_eq!(stems[6], "Untitled note");
    }

    #[test]
    fn duplicate_manifest_paths_abort_instead_of_overwriting_an_export() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let destination = std::env::current_dir()
            .expect("the test should have a working directory")
            .join("target")
            .join(format!(
                "crane-collision-test-{}-{nonce}",
                std::process::id()
            ));
        std::fs::create_dir_all(&destination).expect("test destination should exist");
        let manifest = ExportManifest {
            entries: vec![
                ExportEntry {
                    relative_path: PathBuf::from("Notes/Same.md"),
                    contents: b"first".to_vec(),
                },
                ExportEntry {
                    relative_path: PathBuf::from("Notes/Same.md"),
                    contents: b"second".to_vec(),
                },
            ],
        };

        let error = write_library_export(&destination, &manifest, "Crane Export collision")
            .expect_err("duplicate destinations must abort publication");

        assert!(error.to_string().contains("Could not write"));
        assert!(!destination.join("Crane Export collision").exists());
        let _ = std::fs::remove_dir_all(destination);
    }

    #[test]
    fn rejects_a_batch_larger_than_the_limit() {
        let files = vec![NamedBytes {
            name: "huge.md".into(),
            bytes: vec![b'x'; MAX_BATCH_BYTES + 1],
        }];

        let error = prepare_import(files, 1, ["one".into()].into_iter())
            .expect_err("oversized batches must fail before parsing");
        assert!(error.to_string().contains("64 MB"));
    }

    struct FailingWriter {
        writes: usize,
    }

    impl EntryWriter for FailingWriter {
        fn write(&mut self, path: &Path, contents: &[u8]) -> io::Result<()> {
            self.writes += 1;
            if self.writes == 2 {
                return Err(io::Error::other("injected write failure"));
            }
            FsEntryWriter.write(path, contents)
        }
    }

    #[test]
    fn does_not_publish_partial_export_when_a_write_fails() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let destination = std::env::current_dir()
            .expect("the test should have a working directory")
            .join("target")
            .join(format!("crane-export-test-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&destination).expect("test destination should exist");
        let manifest = ExportManifest {
            entries: vec![
                ExportEntry {
                    relative_path: PathBuf::from("Notes/One.md"),
                    contents: b"one".to_vec(),
                },
                ExportEntry {
                    relative_path: PathBuf::from("Notes/Two.md"),
                    contents: b"two".to_vec(),
                },
            ],
        };

        let error = write_library_export_with(
            &destination,
            &manifest,
            "Crane Export 2026-10-19 170000",
            &mut FailingWriter { writes: 0 },
        )
        .expect_err("the injected write failure must abort publication");

        assert!(error.to_string().contains("Could not write"));
        assert!(!destination.join("Crane Export 2026-10-19 170000").exists());
        assert_eq!(
            std::fs::read_dir(&destination)
                .expect("destination should remain readable")
                .count(),
            0
        );
        let _ = std::fs::remove_dir_all(destination);
    }
}
