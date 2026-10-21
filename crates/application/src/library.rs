use crate::restore::PendingRestore;
use crate::{
    AppError, Command, CommandData, CommandResult, Environment, ErrorCode, LibraryRepository,
    NoteState, Outcome, TransferIo,
};
use crane_domain::knowledge::KnowledgeIndex;
use crane_domain::{
    extract_tags, inspect_backup, prepare_import, validate_note, validate_store, Note, Store,
    SCHEMA_VERSION,
};

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
    transfers: Option<Box<dyn TransferIo>>,
    pending_restore: Option<PendingRestore>,
    index: KnowledgeIndex,
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
        let index = KnowledgeIndex::build(&store);
        Ok(Self {
            repository,
            environment,
            store,
            index,
            revision: 0,
            transfers: None,
            pending_restore: None,
        })
    }

    pub fn with_transfers(mut self, io: Box<dyn TransferIo>) -> Self {
        self.transfers = Some(io);
        self
    }

    pub fn execute(&mut self, command: Command) -> Result<CommandResult, AppError> {
        match command {
            Command::CreateNoteWithTitle { title } => {
                let (candidate, data) = self.prepare_create_with_title(title)?;
                self.commit_candidate(candidate, data)
            }
            Command::CreateNote => {
                let (candidate, data) = self.prepare_create()?;
                self.commit_candidate(candidate, data)
            }
            Command::SaveNote {
                id,
                title,
                body,
                draft_revision,
            } => {
                let (candidate, data) = self.prepare_save(id, title, body, draft_revision)?;
                self.commit_candidate(candidate, data)
            }
            Command::SetNoteState { id, state, value } => {
                let (candidate, data) = self.prepare_state_change(id, state, value)?;
                self.commit_candidate(candidate, data)
            }
            Command::ImportMarkdown => self.import_markdown(),
            Command::ExportNote { id } => self.export_note(&id),
            Command::ExportLibrary => self.export_library(),
            Command::CreateBackup => self.create_backup(),
            Command::InspectBackup => self.inspect_backup(),
            Command::RestoreBackup { token } => self.restore_backup(&token),
            Command::CancelRestore { token } => self.cancel_restore(&token),
        }
    }

    fn commit_candidate(
        &mut self,
        candidate: Store,
        data: CommandData,
    ) -> Result<CommandResult, AppError> {
        validate_store(&candidate)?;
        self.repository.commit(&candidate)?;
        self.store = candidate;
        match &data {
            CommandData::Created(note) => self.index.replace_note(note),
            CommandData::StateChanged(note) => self.index.update_visibility(note),
            CommandData::Saved { note, .. } => self.index.replace_note(note),
            CommandData::Imported { store, .. } | CommandData::Restored(store) => {
                self.index = KnowledgeIndex::build(store);
            }
            _ => {}
        }
        self.revision = self.revision.saturating_add(1);
        Ok(CommandResult {
            revision: self.revision,
            data,
        })
    }

    pub fn knowledge_index(&self) -> &KnowledgeIndex {
        &self.index
    }

    pub fn snapshot(&self) -> LibrarySnapshot {
        LibrarySnapshot {
            store: self.store.clone(),
            revision: self.revision,
        }
    }

    fn prepare_create_with_title(
        &mut self,
        title: String,
    ) -> Result<(Store, CommandData), AppError> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::new(
                ErrorCode::Validation,
                "A note title is required.",
            ));
        }
        let timestamp = self.environment.now_millis();
        let note = Note {
            id: self.environment.new_note_id(),
            title,
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

    fn import_markdown(&mut self) -> Result<CommandResult, AppError> {
        let outcome = self.transfer_io()?.pick_markdown()?;
        let Outcome::Completed(files) = outcome else {
            return Ok(self.result(CommandData::Cancelled));
        };
        let timestamp = self.environment.now_millis();
        let ids: Vec<_> = (0..files.len())
            .map(|_| self.environment.new_note_id())
            .collect();
        let imported = prepare_import(files, timestamp, ids.into_iter())?;
        let imported_count = imported.len();
        let mut candidate = self.store.clone();
        candidate.notes.extend(imported);
        let data = CommandData::Imported {
            store: candidate.clone(),
            imported_count,
        };
        self.commit_candidate(candidate, data)
    }

    fn export_note(&mut self, id: &str) -> Result<CommandResult, AppError> {
        let note = self
            .store
            .notes
            .iter()
            .find(|note| note.id == id)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "That note no longer exists."))?;
        match self.transfer_io()?.export_note(&note)? {
            Outcome::Completed(exported_count) => {
                Ok(self.result(CommandData::Exported(crate::ExportSummary {
                    exported_count,
                    folder_name: String::new(),
                })))
            }
            Outcome::Cancelled => Ok(self.result(CommandData::Cancelled)),
        }
    }

    fn export_library(&mut self) -> Result<CommandResult, AppError> {
        let store = self.store.clone();
        match self.transfer_io()?.export_library(&store)? {
            Outcome::Completed(summary) => Ok(self.result(CommandData::Exported(summary))),
            Outcome::Cancelled => Ok(self.result(CommandData::Cancelled)),
        }
    }

    fn create_backup(&mut self) -> Result<CommandResult, AppError> {
        let store = self.store.clone();
        let created_at = self.environment.now_millis();
        match self.transfer_io()?.create_backup(&store, created_at)? {
            Outcome::Completed(note_count) => {
                Ok(self.result(CommandData::BackupCreated { note_count }))
            }
            Outcome::Cancelled => Ok(self.result(CommandData::Cancelled)),
        }
    }

    fn inspect_backup(&mut self) -> Result<CommandResult, AppError> {
        self.pending_restore = None;
        let outcome = self.transfer_io()?.pick_backup()?;
        let Outcome::Completed(bytes) = outcome else {
            return Ok(self.result(CommandData::Cancelled));
        };
        let (envelope, inspection) = inspect_backup(&bytes)?;
        let token = format!("restore-{}", self.environment.new_note_id());
        self.pending_restore = Some(PendingRestore {
            token: token.clone(),
            replacement: envelope.store,
            library_revision: self.revision,
            issued_at_monotonic: self.environment.monotonic_millis(),
        });
        Ok(self.result(CommandData::Inspection { token, inspection }))
    }

    fn restore_backup(&mut self, token: &str) -> Result<CommandResult, AppError> {
        let Some(pending) = self.pending_restore.as_ref() else {
            return Err(stale_confirmation());
        };
        if pending.token != token {
            return Err(stale_confirmation());
        }
        if !pending.is_fresh(self.revision, self.environment.monotonic_millis()) {
            self.pending_restore = None;
            return Err(stale_confirmation());
        }
        let pending = self.pending_restore.take().ok_or_else(stale_confirmation)?;
        let current = self.store.clone();
        let created_at = self.environment.now_millis();
        self.transfer_io()?
            .write_safety_backup(&current, created_at)?;
        validate_store(&pending.replacement)?;
        self.repository.commit(&pending.replacement)?;
        self.store = pending.replacement;
        self.index = KnowledgeIndex::build(&self.store);
        self.revision = self.revision.saturating_add(1);
        Ok(self.result(CommandData::Restored(self.store.clone())))
    }

    fn cancel_restore(&mut self, token: &str) -> Result<CommandResult, AppError> {
        if self
            .pending_restore
            .as_ref()
            .is_some_and(|pending| pending.token == token)
        {
            self.pending_restore = None;
            Ok(self.result(CommandData::RestoreCancelled))
        } else {
            Err(stale_confirmation())
        }
    }

    fn transfer_io(&mut self) -> Result<&mut (dyn TransferIo + 'static), AppError> {
        self.transfers.as_deref_mut().ok_or_else(|| {
            AppError::new(
                ErrorCode::Busy,
                "File transfer support is not available right now.",
            )
        })
    }

    fn result(&self, data: CommandData) -> CommandResult {
        CommandResult {
            revision: self.revision,
            data,
        }
    }
}

fn stale_confirmation() -> AppError {
    AppError::new(
        ErrorCode::StaleConfirmation,
        "That restore confirmation is no longer valid.",
    )
}
