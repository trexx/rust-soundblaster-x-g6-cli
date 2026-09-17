//! Background watcher: polls for the G6 and replays the saved settings whenever it appears.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::Args;
use hidapi::HidApi;

use crate::api::Api;
use crate::device::{self, DryRun, FrameSink, G6, G6_PRODUCT_ID, G6_VENDOR_ID};
use crate::state;

/// Command-line options of the watcher, shared by `g6-cli watch` and `g6-watch`.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct WatchArgs {
    /// Seconds between two checks for the device
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u64).range(1..))]
    pub interval: u64,

    /// Milliseconds to wait after the device appears before talking to it
    #[arg(long, default_value_t = 1500)]
    pub settle: u64,

    /// Append log lines to this file (default: stderr for `g6-cli watch`, g6-watch.log next to
    /// the state file for `g6-watch`)
    #[arg(long, value_name = "PATH")]
    pub log: Option<PathBuf>,
}

/// Where the watcher looks for the G6 and how it gets a sink for it.
pub trait DeviceSource {
    /// Is the G6 plugged in right now?
    fn present(&mut self) -> Result<bool>;
    /// Open the device (called only after `present` returned `true` in the same tick).
    fn open(&mut self) -> Result<Box<dyn FrameSink>>;
}

/// Device source backed by hidapi. Each probe enumerates only the G6's vendor/product ids.
pub struct HidSource {
    api: HidApi,
    debug: bool,
    dry_run: bool,
}

impl HidSource {
    pub fn new(debug: bool, dry_run: bool) -> Result<Self> {
        let api = HidApi::new().context("Failed to initialize HID API")?;
        Ok(Self {
            api,
            debug,
            dry_run,
        })
    }
}

impl DeviceSource for HidSource {
    fn present(&mut self) -> Result<bool> {
        self.api
            .reset_devices()
            .context("Failed to reset HID device list")?;
        self.api
            .add_devices(G6_VENDOR_ID, G6_PRODUCT_ID)
            .context("Failed to enumerate HID devices")?;
        Ok(device::find(&self.api).is_some())
    }

    fn open(&mut self) -> Result<Box<dyn FrameSink>> {
        if self.dry_run {
            return Ok(Box::new(DryRun));
        }
        Ok(Box::new(G6::open_with(&self.api)?.with_debug(self.debug)))
    }
}

// ── Presence state machine ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Unchanged,
    Connected,
    Disconnected,
}

/// Tracks whether the device is present and whether the saved settings still need applying.
#[derive(Debug, Default)]
pub struct Monitor {
    observed: bool,
    present: bool,
    pending: bool,
    failure_logged: bool,
}

impl Monitor {
    /// Record one observation. A device that is present on the very first observation counts
    /// as connected, so settings are applied when the watcher starts with the G6 plugged in.
    pub fn observe(&mut self, present: bool) -> Event {
        let event = match (self.observed, self.present, present) {
            (false, _, true) | (true, false, true) => Event::Connected,
            (true, true, false) => Event::Disconnected,
            _ => Event::Unchanged,
        };
        self.observed = true;
        self.present = present;
        match event {
            Event::Connected => self.pending = true,
            Event::Disconnected => {
                self.pending = false;
                self.failure_logged = false;
            }
            Event::Unchanged => {}
        }
        event
    }

    pub fn needs_apply(&self) -> bool {
        self.present && self.pending
    }

    pub fn applied(&mut self) {
        self.pending = false;
        self.failure_logged = false;
    }

    /// Returns `true` the first time a failure is reported for the current connection.
    pub fn note_failure(&mut self) -> bool {
        !std::mem::replace(&mut self.failure_logged, true)
    }
}

// ── Watcher ───────────────────────────────────────────────────────────────────

pub struct Watcher<'a> {
    monitor: Monitor,
    settle: Duration,
    state_path: Option<&'a Path>,
    log: &'a mut dyn Write,
}

impl<'a> Watcher<'a> {
    pub fn new(settle: Duration, state_path: Option<&'a Path>, log: &'a mut dyn Write) -> Self {
        Self {
            monitor: Monitor::default(),
            settle,
            state_path,
            log,
        }
    }

    /// One poll: observe presence, log transitions, and apply the saved settings if due.
    pub fn tick(&mut self, source: &mut dyn DeviceSource) -> Result<()> {
        match self.monitor.observe(source.present()?) {
            Event::Connected => {
                self.log("G6 connected")?;
                std::thread::sleep(self.settle);
            }
            Event::Disconnected => self.log("G6 disconnected")?,
            Event::Unchanged => {}
        }

        if self.monitor.needs_apply() {
            match self.apply(source) {
                Ok(applied) if applied.is_empty() => {
                    self.log("nothing saved to apply")?;
                    self.monitor.applied();
                }
                Ok(applied) => {
                    self.log(&format!("applied: {}", applied.join(", ")))?;
                    self.monitor.applied();
                }
                Err(e) => {
                    if self.monitor.note_failure() {
                        self.log(&format!("apply failed, will retry: {e:#}"))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn apply(&mut self, source: &mut dyn DeviceSource) -> Result<Vec<&'static str>> {
        // Re-read the state file every time so changes made with the CLI are picked up.
        let mut state = self.state_path.map(state::load).unwrap_or_default();
        let sink = source.open()?;
        Api::new(sink.as_ref(), &mut state, None).apply_saved()
    }

    pub fn log(&mut self, message: &str) -> Result<()> {
        writeln!(self.log, "{} {message}", timestamp()).context("Failed to write to the log")
    }
}

/// Poll forever with `HidSource`, applying the saved settings whenever the G6 connects.
pub fn run(
    args: &WatchArgs,
    state_path: Option<&Path>,
    dry_run: bool,
    debug: bool,
    log: &mut dyn Write,
) -> Result<()> {
    let mut source = HidSource::new(debug, dry_run)?;
    let mut watcher = Watcher::new(Duration::from_millis(args.settle), state_path, log);
    watcher.log(&format!(
        "watching for the G6 every {}s{}",
        args.interval,
        if dry_run { " (dry run)" } else { "" }
    ))?;
    loop {
        watcher.tick(&mut source)?;
        std::thread::sleep(Duration::from_secs(args.interval));
    }
}

/// Append to `path`, or write to stderr when no path is given.
pub fn open_log(path: Option<&Path>) -> Result<Box<dyn Write>> {
    Ok(match path {
        Some(path) => Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .with_context(|| format!("Failed to open log file {}", path.display()))?,
        ),
        None => Box::new(std::io::stderr()),
    })
}

// ── Timestamps ────────────────────────────────────────────────────────────────

/// Current time as `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix(secs)
}

/// `YYYY-MM-DD HH:MM:SS` (UTC) for a Unix timestamp, via the civil-from-days algorithm.
pub fn format_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hour, min, sec) = (rem / 3600, rem % 3600 / 60, rem % 60);

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
}
