use crane_domain::Note;

#[derive(Clone, Debug)]
pub enum Command {
    CreateNote,
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

#[derive(Clone, Debug)]
pub enum CommandData {
    Created(Note),
    Saved { note: Note, draft_revision: u64 },
    StateChanged(Note),
}
