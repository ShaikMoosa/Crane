use crane_domain::{BackupInspection, Note, Store};

#[derive(Clone, Debug)]
pub enum Command {
    CreateNote,
    CreateNoteWithTitle {
        title: String,
    },
    SaveNote {
        id: String,
        title: String,
        body: String,
        draft_revision: u64,
    },
    SetNoteState {
        id: String,
        state: NoteState,
        value: bool,
    },
    ImportMarkdown,
    ExportNote {
        id: String,
    },
    ExportLibrary,
    CreateBackup,
    InspectBackup,
    RestoreBackup {
        token: String,
    },
    CancelRestore {
        token: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteState {
    Pinned,
    Archived,
    Trashed,
}

#[derive(Clone, Debug)]
pub struct CommandResult {
    pub revision: u64,
    pub data: CommandData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSummary {
    pub exported_count: usize,
    pub folder_name: String,
}

#[derive(Clone, Debug)]
pub enum CommandData {
    Created(Note),
    Saved {
        note: Note,
        draft_revision: u64,
    },
    StateChanged(Note),
    Imported {
        store: Store,
        imported_count: usize,
    },
    Exported(ExportSummary),
    BackupCreated {
        note_count: usize,
    },
    Inspection {
        token: String,
        inspection: BackupInspection,
    },
    Restored(Store),
    Cancelled,
    RestoreCancelled,
}
