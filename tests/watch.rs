// Tests of the connect watcher against a scripted DeviceSource (no device needed).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{bail, Result};
use g6_cli::device::FrameSink;
use g6_cli::model::G6State;
use g6_cli::spec::decoder::{decoder_mode, DecoderMode};
use g6_cli::spec::HidFrame;
use g6_cli::state::save;
use g6_cli::watch::{format_unix, DeviceSource, Event, Monitor, Watcher};

struct SharedRecorder(Rc<RefCell<Vec<HidFrame>>>);

impl FrameSink for SharedRecorder {
    fn send(&self, frames: &[HidFrame]) -> Result<()> {
        self.0.borrow_mut().extend_from_slice(frames);
        Ok(())
    }
}

/// Scripted presence and open results; every open that is not scripted to fail succeeds.
struct FakeSource {
    presence: VecDeque<bool>,
    open_failures: VecDeque<bool>,
    opens: usize,
    frames: Rc<RefCell<Vec<HidFrame>>>,
}

impl FakeSource {
    fn new(presence: &[bool]) -> Self {
        Self {
            presence: presence.iter().copied().collect(),
            open_failures: VecDeque::new(),
            opens: 0,
            frames: Rc::default(),
        }
    }

    fn failing_opens(mut self, failures: &[bool]) -> Self {
        self.open_failures = failures.iter().copied().collect();
        self
    }

    fn frames(&self) -> Vec<HidFrame> {
        self.frames.borrow().clone()
    }
}

impl DeviceSource for FakeSource {
    fn present(&mut self) -> Result<bool> {
        Ok(self
            .presence
            .pop_front()
            .expect("scripted presence exhausted"))
    }

    fn open(&mut self) -> Result<Box<dyn FrameSink>> {
        self.opens += 1;
        if self.open_failures.pop_front().unwrap_or(false) {
            bail!("interface not ready");
        }
        Ok(Box::new(SharedRecorder(self.frames.clone())))
    }
}

/// A state file whose only saved setting is decoder = Night.
fn state_with_decoder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("g6-cli-watch-test-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.json");
    let mut state = G6State::default();
    state.settings.decoder = Some(DecoderMode::Night);
    save(&path, &state).unwrap();
    path
}

fn run_ticks(
    source: &mut FakeSource,
    state_path: Option<&std::path::Path>,
    ticks: usize,
) -> String {
    let mut log = Vec::new();
    {
        let mut watcher = Watcher::new(Duration::ZERO, state_path, &mut log);
        for _ in 0..ticks {
            watcher.tick(source).unwrap();
        }
    }
    String::from_utf8(log).unwrap()
}

fn count(log: &str, needle: &str) -> usize {
    log.lines().filter(|l| l.contains(needle)).count()
}

// ── Monitor ───────────────────────────────────────────────────────────────────

#[test]
fn monitor_reports_transitions_once() {
    let mut m = Monitor::default();
    assert_eq!(m.observe(false), Event::Unchanged);
    assert!(!m.needs_apply());
    assert_eq!(m.observe(true), Event::Connected);
    assert!(m.needs_apply());
    assert_eq!(m.observe(true), Event::Unchanged);
    assert!(m.needs_apply(), "still pending until applied");
    m.applied();
    assert!(!m.needs_apply());
    assert_eq!(m.observe(false), Event::Disconnected);
    assert_eq!(m.observe(false), Event::Unchanged);
    assert_eq!(m.observe(true), Event::Connected);
    assert!(m.needs_apply());
}

#[test]
fn monitor_treats_present_at_start_as_connected() {
    let mut m = Monitor::default();
    assert_eq!(m.observe(true), Event::Connected);
    assert!(m.needs_apply());
}

#[test]
fn monitor_cancels_pending_apply_on_disconnect_and_logs_failures_once() {
    let mut m = Monitor::default();
    m.observe(true);
    assert!(m.note_failure(), "first failure is reported");
    assert!(!m.note_failure(), "repeat failures are silent");
    m.observe(false);
    assert!(!m.needs_apply());
    m.observe(true);
    assert!(m.note_failure(), "a new connection reports again");
}

// ── Watcher ───────────────────────────────────────────────────────────────────

#[test]
fn watcher_applies_on_every_connect_and_at_start() {
    let path = state_with_decoder("connects");
    let mut source = FakeSource::new(&[false, true, true, false, true]);
    let log = run_ticks(&mut source, Some(&path), 5);

    let mut expected = decoder_mode(DecoderMode::Night);
    expected.extend(decoder_mode(DecoderMode::Night));
    assert_eq!(source.frames(), expected, "applied exactly twice");
    assert_eq!(source.opens, 2);
    assert_eq!(count(&log, "G6 connected"), 2);
    assert_eq!(count(&log, "G6 disconnected"), 1);
    assert_eq!(count(&log, "applied: decoder"), 2);
    assert!(
        log.lines().all(|l| l.len() > 20 && l.as_bytes()[4] == b'-'),
        "timestamped: {log}"
    );
}

#[test]
fn watcher_retries_open_failures_and_logs_the_failure_once() {
    let path = state_with_decoder("retries");
    let mut source = FakeSource::new(&[true, true, true, true]).failing_opens(&[true, true]);
    let log = run_ticks(&mut source, Some(&path), 4);

    assert_eq!(
        source.opens, 3,
        "two failures, then success, then nothing more"
    );
    assert_eq!(source.frames(), decoder_mode(DecoderMode::Night));
    assert_eq!(count(&log, "apply failed"), 1, "{log}");
    assert_eq!(count(&log, "applied: decoder"), 1);
}

#[test]
fn watcher_picks_up_state_changes_between_connects() {
    let path = state_with_decoder("reload");
    let mut source = FakeSource::new(&[true, false, true]);
    let mut log = Vec::new();
    let mut watcher = Watcher::new(Duration::ZERO, Some(&path), &mut log);
    watcher.tick(&mut source).unwrap();
    watcher.tick(&mut source).unwrap();

    let mut state = G6State::default();
    state.settings.decoder = Some(DecoderMode::Full);
    save(&path, &state).unwrap();
    watcher.tick(&mut source).unwrap();

    let mut expected = decoder_mode(DecoderMode::Night);
    expected.extend(decoder_mode(DecoderMode::Full));
    assert_eq!(source.frames(), expected);
}

#[test]
fn watcher_with_nothing_saved_logs_and_does_not_retry() {
    let mut source = FakeSource::new(&[true, true]);
    let log = run_ticks(&mut source, None, 2);
    assert_eq!(source.opens, 1);
    assert!(source.frames().is_empty());
    assert_eq!(count(&log, "nothing saved to apply"), 1, "{log}");
}

// ── Timestamps ────────────────────────────────────────────────────────────────

#[test]
fn format_unix_matches_known_dates() {
    assert_eq!(format_unix(0), "1970-01-01 00:00:00");
    assert_eq!(format_unix(951_782_400), "2000-02-29 00:00:00");
    assert_eq!(format_unix(1_700_000_000), "2023-11-14 22:13:20");
    assert_eq!(format_unix(4_102_444_799), "2099-12-31 23:59:59");
}
