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
}

pub trait TransferIo: Send {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError>;
    fn export_note(&mut self, note: &Note) -> Result<Outcome<usize>, AppError>;
    fn export_library(&mut self, store: &Store) -> Result<Outcome<ExportSummary>, AppError>;
    fn create_backup(&mut self, store: &Store, created_at: u64)
        -> Result<Outcome<usize>, AppError>;
    fn pick_backup(&mut self) -> Result<Outcome<Vec<u8>>, AppError>;
    fn write_safety_backup(&mut self, store: &Store, created_at: u64) -> Result<(), AppError>;
}
