use crate::AppError;
use crane_domain::Store;

pub trait LibraryRepository: Send {
    fn load_or_seed(&mut self, now: u64) -> Result<Store, AppError>;
    fn commit(&mut self, candidate: &Store) -> Result<(), AppError>;
}

pub trait Environment: Send {
    fn now_millis(&self) -> u64;
    fn monotonic_millis(&self) -> u64;
    fn new_note_id(&mut self) -> String;
}
