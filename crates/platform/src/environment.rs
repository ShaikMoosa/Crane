use crane_application::Environment;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static NOTE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct SystemEnvironment {
    started_at: Instant,
}

impl SystemEnvironment {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }
}

impl Default for SystemEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment for SystemEnvironment {
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn monotonic_millis(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    fn local_day_bounds_millis(&self) -> (u64, u64) {
        #[cfg(windows)]
        {
            if let Some(bounds) = windows_local_day_bounds() {
                return bounds;
            }
        }
        const DAY: u64 = 86_400_000;
        let start = self.now_millis() / DAY * DAY;
        (start, start.saturating_add(DAY))
    }

    fn new_note_id(&mut self) -> String {
        let sequence = NOTE_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("note-{}-{sequence}", self.now_millis())
    }
}

#[cfg(windows)]
fn windows_local_day_bounds() -> Option<(u64, u64)> {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    use windows_sys::Win32::System::Time::{SystemTimeToFileTime, TzSpecificLocalTimeToSystemTime};

    fn to_millis(local: &SYSTEMTIME) -> Option<u64> {
        let mut utc = SYSTEMTIME::default();
        let mut file_time = FILETIME::default();
        unsafe {
            if TzSpecificLocalTimeToSystemTime(std::ptr::null(), local, &mut utc) == 0 {
                return None;
            }
            if SystemTimeToFileTime(&utc, &mut file_time) == 0 {
                return None;
            }
        }
        let ticks = ((file_time.dwHighDateTime as u64) << 32) | file_time.dwLowDateTime as u64;
        Some(ticks / 10_000 - 11_644_473_600_000)
    }

    let mut local = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut local) };
    let start_local = SYSTEMTIME {
        wYear: local.wYear,
        wMonth: local.wMonth,
        wDay: local.wDay,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
        ..Default::default()
    };
    let start = to_millis(&start_local)?;
    Some((start, start.saturating_add(86_400_000)))
}
