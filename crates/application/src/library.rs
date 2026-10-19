use crate::{
    AppError, Command, CommandData, CommandResult, Environment, ErrorCode, LibraryRepository,
    NoteState,
};
use crane_domain::{extract_tags, validate_note, validate_store, Note, Store, SCHEMA_VERSION};

#[derive(Clone, Debug)]
pub struct LibrarySnapshot {
    pub store: Store,
    pub revision: u64,
}

pub struct Library {
    repository: Box<dyn LibraryRepository>,
    environment: Box<dyn Environment>,
    store: Store,
    revision: u64,
}

impl Library {
    pub fn open(
        mut repository: Box<dyn LibraryRepository>,
        environment: Box<dyn Environment>,
    ) -> Result<Self, AppError> {
        let store = repository.load_or_seed(environment.now_millis())?;
        if store.schema_version != SCHEMA_VERSION {
            return Err(AppError::new(
                ErrorCode::UnsupportedVersion,
                format!("Unsupported store version {}.", store.schema_version),
            ));
        }
        validate_store(&store)?;
        Ok(Self {
            repository,
            environment,
            store,
            revision: 0,
        })
    }

    pub fn execute(&mut self, command: Command) -> Result<CommandResult, AppError> {
        let (candidate, data) = match command {
            Command::CreateNote => self.prepare_create()?,
            Command::SaveNote {
                id,
                title,
                body,
                draft_revision,
            } => self.prepare_save(id, title, body, draft_revision)?,
            Command::SetNoteState { id, state, value } => {
                self.prepare_state_change(id, state, value)?
            }
        };
        validate_store(&candidate)?;
        self.repository.commit(&candidate)?;
        self.store = candidate;
        self.revision = self.revision.saturating_add(1);
        Ok(CommandResult {
            revision: self.revision,
            data,
        })
    }

    pub fn snapshot(&self) -> LibrarySnapshot {
        LibrarySnapshot {
            store: self.store.clone(),
            revision: self.revision,
        }
    }

    fn prepare_create(&mut self) -> Result<(Store, CommandData), AppError> {
        let timestamp = self.environment.now_millis();
        let note = Note {
            id: self.environment.new_note_id(),
            title: "Untitled note".into(),
            body: String::new(),
            tags: Vec::new(),
            created_at: timestamp,
            updated_at: timestamp,
            pinned: false,
            archived: false,
            trashed: false,
            extensions: Default::default(),
        };
        validate_note(&note)?;
        let mut candidate = self.store.clone();
        candidate.notes.push(note.clone());
        Ok((candidate, CommandData::Created(note)))
    }

    fn prepare_save(
        &self,
        id: String,
        title: String,
        body: String,
        draft_revision: u64,
    ) -> Result<(Store, CommandData), AppError> {
        let mut candidate = self.store.clone();
        let note = candidate
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "That note no longer exists."))?;
        note.title = title;
        note.body = body;
        note.tags = extract_tags(&note.body);
        note.updated_at = self.environment.now_millis();
        validate_note(note)?;
        let saved = note.clone();
        Ok((
            candidate,
            CommandData::Saved {
                note: saved,
                draft_revision,
            },
        ))
    }

    fn prepare_state_change(
        &self,
        id: String,
        state: NoteState,
        value: bool,
    ) -> Result<(Store, CommandData), AppError> {
        let mut candidate = self.store.clone();
        let note = candidate
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "That note no longer exists."))?;
        match state {
            NoteState::Pinned => note.pinned = value,
            NoteState::Archived => note.archived = value,
            NoteState::Trashed => note.trashed = value,
        }
        note.updated_at = self.environment.now_millis();
        let changed = note.clone();
        Ok((candidate, CommandData::StateChanged(changed)))
    }
}
