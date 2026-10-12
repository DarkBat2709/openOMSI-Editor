//! Low-volume timing and independent hang reports for the standalone editor.
use std::{collections::HashMap, sync::{Mutex, OnceLock}, time::{Duration, Instant}};

struct Active { label: &'static str, detail: String, started: Instant, reported: Option<Instant> }
#[derive(Default)]
struct State { next: u64, active: HashMap<u64, Active>, last: HashMap<&'static str, Instant> }
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
        log::info!("EDITOR-DIAG enabled: slow steps >=250ms; watchdog >=2s; repeat interval 5s");
        if let Err(e) = std::thread::Builder::new().name("editor-watchdog".into()).spawn(|| loop {
            std::thread::sleep(Duration::from_secs(1));
            let Some(state) = STATE.get() else { continue; };
            let messages = match state.lock() { Ok(mut s) => overdue(&mut s, Instant::now()), Err(_) => return };
            // Never hold the registry lock while writing to the logger.
            for message in messages { log::warn!("{message}"); }
        }) { log::warn!("EDITOR-DIAG watchdog unavailable: {e}"); }
        Mutex::new(State::default())
    })
}

pub struct Span(Option<u64>);
impl Span {
    pub fn new(label: &'static str, detail: impl Into<String>) -> Self {
        if !*ENABLED.get_or_init(|| std::env::var("OPENOMSI_EDITOR_DIAGNOSTICS").as_deref() == Ok("1")) { return Self(None); }
        let Ok(mut s) = state().lock() else { return Self(None); };
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
