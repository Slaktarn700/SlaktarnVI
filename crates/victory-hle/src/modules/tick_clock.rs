//! A clock that steps with the frame clock's ticks, for a title DLL that
//! paces by `QueryPerformanceCounter` what the console shows at vblanks
//! (Bink's videos): the counter as it was when the current tick began.
//!
//! Bink shows a new video frame when its clock passes the frame's time.
//! The title asks it a few milliseconds after each tick, how many varying
//! with the work of the frame. A 30 fps video's frames fall on every
//! second tick (60 Hz), exactly on a tick, so that wobble decided whether
//! a frame came at this tick or the next: the logo video showed its frames
//! for 3, 1, 2, 3, 1, 2 ticks in stretches, instead of 2 each
//! (`VICTORY_TRACE_SHADER`, 2026-11-26), and juddered. On the console the
//! title asks at the same point after each vblank. With this clock every
//! ask within a tick reads the tick's start: the frame changes on its tick.
//! Between ticks the clock stands still (at most one interval, 16.7 ms).

use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

/// The DLLs that get the clock (`VICTORY_BINK_TICK_CLOCK=0`: none).
pub fn wanted(dll: &str) -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_BINK_TICK_CLOCK").is_none_or(|v| v != "0"))
        && dll.eq_ignore_ascii_case("Bink2Scarlett.dll")
}

/// `QueryPerformanceCounter` for those DLLs: the count at the current
/// tick's start (the plain count while the frame clock has no ticks).
///
/// # Safety
/// `out` is null or points at an `i64` to write, as for the system's.
pub unsafe extern "system" fn query_performance_counter(out: *mut i64) -> i32 {
    let mut now = 0i64;
    // SAFETY: a local out value.
    if unsafe { QueryPerformanceCounter(&mut now) } == 0 {
        return 0;
    }
    let count = match victory_gpu::frame::since_tick() {
        Some(since) => now - counts(since),
        None => now,
    };
    if !out.is_null() {
        // SAFETY: the caller's out pointer.
        unsafe { out.write(count) };
    }
    1
}

/// `since` in counter units.
fn counts(since: std::time::Duration) -> i64 {
    static FREQUENCY: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    let f = *FREQUENCY.get_or_init(|| {
        let mut f = 0i64;
        // SAFETY: a local out value.
        unsafe { QueryPerformanceFrequency(&mut f) };
        f.max(1)
    });
    (since.as_nanos() * f as u128 / 1_000_000_000) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_bink_gets_the_clock() {
        assert!(wanted("Bink2Scarlett.dll") && wanted("bink2scarlett.DLL"));
        assert!(!wanted("XboxBvhBuilder.dll"));
    }

    #[test]
    fn the_clock_never_runs_ahead_of_the_counter() {
        let mut plain = 0i64;
        let mut ticked = 0i64;
        // SAFETY: a local out value.
        assert_eq!(unsafe { query_performance_counter(&mut ticked) }, 1);
        // SAFETY: a local out value.
        unsafe { QueryPerformanceCounter(&mut plain) };
        assert!(ticked <= plain && ticked > 0);
        assert_eq!(counts(std::time::Duration::ZERO), 0);
    }

    /// What the title's most frequent system calls cost here (run by hand,
    /// under Proton: `-- --ignored --nocapture what_calls_cost`).
    #[test]
    #[ignore]
    fn what_calls_cost() {
        use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent, Sleep};
        fn each(name: &str, n: u32, mut call: impl FnMut()) {
            let started = std::time::Instant::now();
            for _ in 0..n {
                call();
            }
            println!("{name}: {:.0} ns a call", started.elapsed().as_nanos() as f64 / f64::from(n));
        }
        let mut c = 0i64;
        // SAFETY: a local out value.
        each("QueryPerformanceCounter", 200_000, || unsafe {
            QueryPerformanceCounter(&mut c);
        });
        // SAFETY: plain calls.
        each("Sleep(0)", 50_000, || unsafe { Sleep(0) });
        // SAFETY: an event of our own, closed with the process.
        let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        // SAFETY: the event above.
        each("SetEvent", 100_000, || unsafe {
            SetEvent(event);
        });
        // SAFETY: reads the time-stamp counter.
        each("rdtsc", 1_000_000, || unsafe {
            std::hint::black_box(core::arch::x86_64::_rdtsc());
        });
    }
}
