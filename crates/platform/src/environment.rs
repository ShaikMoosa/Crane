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
fn next_local_date(year: u16, month: u16, day: u16) -> (u16, u16, u16) {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day < days_in_month {
        (year, month, day + 1)
    } else if month < 12 {
        (year, month + 1, 1)
    } else {
        (year + 1, 1, 1)
    }
}

#[cfg(windows)]
fn local_day_bounds_for(
    local: &windows_sys::Win32::Foundation::SYSTEMTIME,
    mut to_millis: impl FnMut(&windows_sys::Win32::Foundation::SYSTEMTIME) -> Option<u64>,
) -> Option<(u64, u64)> {
    use windows_sys::Win32::Foundation::SYSTEMTIME;

    let start_local = SYSTEMTIME {
        wYear: local.wYear,
        wMonth: local.wMonth,
        wDay: local.wDay,
        ..Default::default()
    };
    let (next_year, next_month, next_day) = next_local_date(local.wYear, local.wMonth, local.wDay);
    let end_local = SYSTEMTIME {
        wYear: next_year,
        wMonth: next_month,
        wDay: next_day,
        ..Default::default()
    };
    Some((to_millis(&start_local)?, to_millis(&end_local)?))
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
    local_day_bounds_for(&local, to_millis)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::Foundation::SYSTEMTIME;

    #[test]
    fn next_local_date_crosses_month_year_and_leap_boundaries() {
        assert_eq!(next_local_date(2026, 1, 31), (2026, 2, 1));
        assert_eq!(next_local_date(2026, 12, 31), (2027, 1, 1));
        assert_eq!(next_local_date(2028, 2, 28), (2028, 2, 29));
        assert_eq!(next_local_date(2028, 2, 29), (2028, 3, 1));
    }

    #[test]
    fn local_day_end_comes_from_converted_next_midnight() {
        let local = SYSTEMTIME {
            wYear: 2026,
            wMonth: 3,
            wDay: 8,
            ..Default::default()
        };
        let bounds = local_day_bounds_for(&local, |midnight| {
            if midnight.wDay == 8 {
                Some(1_000)
            } else {
                Some(1_000 + 82_800_000)
            }
        })
        .expect("bounds");

        assert_eq!(bounds, (1_000, 82_801_000));
    }
}
