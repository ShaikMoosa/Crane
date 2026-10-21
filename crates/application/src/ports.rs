use crate::AppError;
use crate::ExportSummary;
use crane_domain::{NamedBytes, Note, Store};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome<T> {
    Completed(T),
    Cancelled,
}

pub trait LibraryRepository: Send {
    fn load_or_seed(&mut self, now: u64) -> Result<Store, AppError>;
    fn commit(&mut self, candidate: &Store) -> Result<(), AppError>;
}

pub trait Environment: Send {
    fn now_millis(&self) -> u64;
    fn monotonic_millis(&self) -> u64;
    fn new_note_id(&mut self) -> String;
    /// Local calendar day containing `now_millis`, as `[start, end)` epoch millis.
    fn local_day_bounds_millis(&self) -> (u64, u64) {
        const DAY: u64 = 86_400_000;
        let start = self.now_millis() / DAY * DAY;
        (start, start.saturating_add(DAY))
    }
}

pub trait TransferIo: Send {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError>;
    fn export_note(&mut self, note: &Note) -> Result<Outcome<usize>, AppError>;
    fn export_library(&mut self, store: &Store) -> Result<Outcome<ExportSummary>, AppError>;
    fn create_backup(&mut self, store: &Store, created_at: u64)
        -> Result<Outcome<usize>, AppError>;
    fn pick_backup(&mut self) -> Result<Outcome<Vec<u8>>, AppError>;
    fn write_safety_backup(&mut self, store: &Store, created_at: u64) -> Result<(), AppError>;

    fn write_pre_migration_backup(
        &mut self,
        store: &Store,
        created_at: u64,
    ) -> Result<String, AppError> {
        self.write_safety_backup(store, created_at)?;
        Ok(format!("pre-migration-{created_at}.crane-backup.json"))
    }
}
