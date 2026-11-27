//! Diagnostics: the spread of real frame times (the title's presents, not
//! frame generation's), for the frame lines. Their 5 s averages hide the
//! stalls a scene has: each line here gives the median, p90, p99 and worst
//! since the last one, the frames over 50 and 100 ms, and the worst drops (a
//! frame of 50 ms or more and at least twice the last line's median) with
//! when they came and what they drew.

use std::sync::Mutex;
use std::time::Instant;

/// A drop: its milliseconds, seconds since the first present, draws, and
/// the present's number.
type Drop = (f32, f64, u64, u64);

struct Frames {
    started: Option<Instant>,
    /// The last present: when, and the draws counted by then.
    last: Option<(Instant, u64)>,
    presents: u64,
    times: Vec<f32>,
    drops: Vec<Drop>,
    /// The last line's median, which a drop is measured against.
    median: f32,
}

static FRAMES: Mutex<Frames> =
    Mutex::new(Frames { started: None, last: None, presents: 0, times: Vec::new(), drops: Vec::new(), median: 0.0 });

/// The drops a line names, the worst first.
const NAMED: usize = 6;

fn is_drop(ms: f32, median: f32) -> bool {
    ms >= 50.0 && ms >= 2.0 * median
}

/// The title's present is over.
pub(crate) fn presented() {
    let now = Instant::now();
    let drawn = crate::draw::drawn();
    let mut f = FRAMES.lock().unwrap_or_else(|e| e.into_inner());
    let started = *f.started.get_or_insert(now);
    f.presents += 1;
    if let Some((last, was)) = f.last {
        let ms = (now - last).as_secs_f32() * 1000.0;
        f.times.push(ms);
        if is_drop(ms, f.median) {
            let drop = (ms, (now - started).as_secs_f64(), drawn.saturating_sub(was), f.presents);
            f.drops.push(drop);
        }
    }
    f.last = Some((now, drawn));
}

/// Median, p90, p99 and worst of `times` (sorted here), and how many are
/// over 50 and over 100 ms.
fn spread(times: &mut [f32]) -> Option<([f32; 4], [usize; 2])> {
    if times.is_empty() {
        return None;
    }
    times.sort_by(f32::total_cmp);
    let at = |q: f64| times[((times.len() - 1) as f64 * q).round() as usize];
    let over = |ms: f32| times.iter().filter(|&&t| t > ms).count();
    Some(([at(0.5), at(0.9), at(0.99), times[times.len() - 1]], [over(50.0), over(100.0)]))
}

/// The worst `NAMED` of `drops`, as a line names them.
fn worst(mut drops: Vec<Drop>) -> String {
    drops.sort_by(|a, b| b.0.total_cmp(&a.0));
    let named: Vec<String> = drops
        .iter()
        .take(NAMED)
        .map(|(ms, at, draws, present)| format!("{ms:.0} ms at {at:.1} s (present {present}, {draws} draws)"))
        .collect();
    if named.is_empty() { "none".into() } else { named.join(", ") }
}

/// The line for the frame lines: what the presents since the last were.
pub(crate) fn line() {
    let (mut times, drops) = {
        let mut f = FRAMES.lock().unwrap_or_else(|e| e.into_inner());
        (std::mem::take(&mut f.times), std::mem::take(&mut f.drops))
    };
    let Some(([median, p90, p99, most], [over50, over100])) = spread(&mut times) else {
        return;
    };
    FRAMES.lock().unwrap_or_else(|e| e.into_inner()).median = median;
    tracing::info!(
        "real frame times since the last line (the title's presents): {} frames, median {median:.1} ms, p90 {p90:.1}, \
         p99 {p99:.1}, worst {most:.1}; {over50} over 50 ms, {over100} over 100 ms; {} drops (50 ms and twice the last \
         median), the worst: {}",
        times.len(),
        drops.len(),
        worst(drops)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spread_of_frame_times() {
        let mut times: Vec<f32> = (1..=100).map(|n| n as f32).collect();
        times.reverse();
        let (q, over) = spread(&mut times).unwrap();
        assert_eq!(q, [51.0, 90.0, 99.0, 100.0]);
        assert_eq!(over, [50, 0]);
        assert!(spread(&mut []).is_none());
    }

    #[test]
    fn a_drop_is_long_and_twice_the_median() {
        assert!(is_drop(80.0, 40.0));
        assert!(!is_drop(70.0, 40.0), "under twice the median");
        assert!(!is_drop(45.0, 10.0), "under 50 ms");
        assert!(is_drop(50.0, 0.0), "before any median");
    }

    #[test]
    fn the_worst_drops_first() {
        let drops = vec![(60.0, 1.0, 10, 5), (120.0, 2.5, 20, 9), (80.0, 3.0, 30, 12)];
        assert_eq!(
            worst(drops),
            "120 ms at 2.5 s (present 9, 20 draws), 80 ms at 3.0 s (present 12, 30 draws), 60 ms at 1.0 s (present 5, 10 draws)"
        );
        assert_eq!(worst(Vec::new()), "none");
    }
}
