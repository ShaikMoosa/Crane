use crane_domain::Store;

pub(crate) const CONFIRMATION_TTL_MILLIS: u64 = 300_000;

pub(crate) struct PendingRestore {
    pub token: String,
    pub replacement: Store,
    pub library_revision: u64,
    pub issued_at_monotonic: u64,
}

impl PendingRestore {
    pub fn is_fresh(&self, revision: u64, monotonic_now: u64) -> bool {
        self.library_revision == revision
            && monotonic_now.saturating_sub(self.issued_at_monotonic) < CONFIRMATION_TTL_MILLIS
    }
}
