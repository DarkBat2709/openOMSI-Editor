//! Low-volume timing and independent hang reports for the standalone editor.
use std::{collections::HashMap, sync::{Mutex, OnceLock}, time::{Duration, Instant}};

struct Active { label: &'static str, detail: String, started: Instant, reported: Option<Instant> }
#[derive(Default)]
struct State { next: u64, active: HashMap<u64, Active>, last: HashMap<&'static str, Instant>, samples: HashMap<&'static str, Samples>, window: Option<Instant> }
// Fixed-size histograms avoid retaining one allocation per frame or mouse event.
const BOUNDS_MS: [u64; 12] = [1, 2, 4, 8, 16, 25, 33, 50, 100, 250, 500, 1000];
#[derive(Default)]
struct Samples { count: u64, total_us: u128, max_us: u128, bins: [u64; 13] }
impl Samples {
    fn record(&mut self, elapsed: Duration) {
        let us = elapsed.as_micros();
        self.count += 1; self.total_us += us; self.max_us = self.max_us.max(us);
        let index = BOUNDS_MS.iter().position(|ms| us <= *ms as u128 * 1000).unwrap_or(12);
        self.bins[index] += 1;
    }
    fn p95_bucket(&self) -> String {
        let target = (self.count * 95).div_ceil(100);
        let mut seen = 0;
        for (i, count) in self.bins.iter().enumerate() {
            seen += count;
            if seen >= target { return BOUNDS_MS.get(i).map(|v| format!("<={v}")).unwrap_or_else(|| ">1000".into()); }
        }
        "none".into()
    }
}
fn summaries(state: &mut State, now: Instant) -> Vec<String> {
    let start = *state.window.get_or_insert(now);
    if now.saturating_duration_since(start) < Duration::from_secs(5) { return Vec::new(); }
    state.window = Some(now);
    let mut samples: Vec<_> = state.samples.drain().collect();
    samples.sort_by_key(|(label, _)| *label);
    samples.into_iter().map(|(label, s)| format!(
        "EDITOR-PERF step={label} count={} avg_ms={:.3} max_ms={:.3} p95_bucket_ms={} window_ms={}",
        s.count, s.total_us as f64 / s.count as f64 / 1000.0, s.max_us as f64 / 1000.0,
        s.p95_bucket(), now.saturating_duration_since(start).as_millis())).collect()
}
static ENABLED: OnceLock<bool> = OnceLock::new();
static STATE: OnceLock<Mutex<State>> = OnceLock::new();

fn overdue(state: &mut State, now: Instant) -> Vec<String> {
    state.active.iter_mut().filter_map(|(id, a)| {
        let age = now.saturating_duration_since(a.started);
        if age < Duration::from_secs(2) || a.reported.is_some_and(|t| now.saturating_duration_since(t) < Duration::from_secs(5)) { return None; }
        a.reported = Some(now);
        Some(format!("EDITOR-DIAG waiting id={id} step={} elapsed_ms={} {}", a.label, age.as_millis(), a.detail))
    }).collect()
}

fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| {
        log::info!("EDITOR-DIAG enabled: slow steps >=250ms; watchdog >=2s; repeat interval 5s; EDITOR-PERF summaries every 5s including short steps");
        if let Err(e) = std::thread::Builder::new().name("editor-watchdog".into()).spawn(|| loop {
            std::thread::sleep(Duration::from_secs(1));
            let Some(state) = STATE.get() else { continue; };
            let messages = match state.lock() { Ok(mut s) => { let now = Instant::now(); let mut messages = overdue(&mut s, now); messages.extend(summaries(&mut s, now)); messages }, Err(_) => return };
            // Never hold the registry lock while writing to the logger.
            for message in messages {
                if message.starts_with("EDITOR-PERF") { log::info!("{message}"); }
                else { log::warn!("{message}"); }
            }
        }) { log::warn!("EDITOR-DIAG watchdog unavailable: {e}"); }
        Mutex::new(State::default())
    })
}

pub struct Span(Option<u64>);
impl Span {
    pub fn new(label: &'static str, detail: impl Into<String>) -> Self {
        if !*ENABLED.get_or_init(|| std::env::var("OPENOMSI_EDITOR_DIAGNOSTICS").as_deref() == Ok("1")) { return Self(None); }
        let Ok(mut s) = state().lock() else { return Self(None); };
        s.window.get_or_insert_with(Instant::now);
        s.next += 1;
        let id = s.next;
        s.active.insert(id, Active { label, detail: detail.into(), started: Instant::now(), reported: None });
        Self(Some(id))
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let Some(id) = self.0 else { return; };
        let Ok(mut s) = state().lock() else { return; };
        let Some(a) = s.active.remove(&id) else { return; };
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(a.started);
        s.samples.entry(a.label).or_default().record(elapsed);
        if elapsed < Duration::from_millis(250) { return; }
        // Always finish an operation already reported by the watchdog.
        if a.reported.is_none() && s.last.get(a.label).is_some_and(|t| now.saturating_duration_since(*t) < Duration::from_secs(5)) { return; }
        s.last.insert(a.label, now);
        drop(s);
        log::warn!("EDITOR-DIAG completed id={id} step={} elapsed_ms={} {}", a.label, elapsed.as_millis(), a.detail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_steps_are_counted_and_summary_windows_reset() {
        let start = Instant::now(); let mut state = State::default(); state.window = Some(start);
        for _ in 0..95 { state.samples.entry("pick").or_default().record(Duration::from_millis(2)); }
        for _ in 0..5 { state.samples.entry("pick").or_default().record(Duration::from_millis(90)); }
        assert!(summaries(&mut state, start + Duration::from_secs(4)).is_empty());
        let out = summaries(&mut state, start + Duration::from_secs(5));
        assert_eq!(out.len(), 1); assert!(out[0].contains("count=100"));
        assert!(out[0].contains("avg_ms=6.400")); assert!(out[0].contains("max_ms=90.000"));
        assert!(out[0].contains("p95_bucket_ms=<=2")); assert!(state.samples.is_empty());
        assert!(summaries(&mut state, start + Duration::from_secs(10)).is_empty());
    }
    #[test]
    fn watchdog_threshold_repeat_and_completion() {
        let start = Instant::now();
        let mut s = State::default();
        s.active.insert(1, Active { label: "preview", detail: "spline=42".into(), started: start, reported: None });
        assert!(overdue(&mut s, start + Duration::from_millis(1999)).is_empty());
        let messages = overdue(&mut s, start + Duration::from_secs(2));
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("spline=42"));
        assert!(overdue(&mut s, start + Duration::from_secs(6)).is_empty());
        assert_eq!(overdue(&mut s, start + Duration::from_secs(7)).len(), 1);
        s.active.remove(&1);
        assert!(overdue(&mut s, start + Duration::from_secs(20)).is_empty());
    }
}
