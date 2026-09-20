//! Load/save of the JSON state file.

use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::G6State;

/// `state.json` next to the running executable.
pub fn default_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("state.json")))
        .unwrap_or_else(|| PathBuf::from("state.json"))
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s: OsString = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Load the state file, falling back to defaults if it is missing.
///
/// An unreadable or unparsable file is moved aside to `<path>.bak` so that the next
/// [`save`] cannot silently overwrite it, and a warning is printed to stderr.
pub fn load(path: &Path) -> G6State {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return G6State::default(),
        Err(e) => {
            eprintln!(
                "Warning: could not read {} ({e}); using defaults.",
                path.display()
            );
            return G6State::default();
        }
    };

    match serde_json::from_str(&text) {
        Ok(state) => state,
        Err(e) => {
            let backup = with_suffix(path, ".bak");
            match std::fs::rename(path, &backup) {
                Ok(()) => eprintln!(
                    "Warning: {} is not valid ({e}); moved it to {} and using defaults.",
                    path.display(),
                    backup.display()
                ),
                Err(re) => eprintln!(
                    "Warning: {} is not valid ({e}) and could not be moved aside ({re}); using defaults.",
                    path.display()
                ),
            }
            G6State::default()
        }
    }
}

/// Write the state file atomically (write to `<path>.tmp`, then rename over `path`).
pub fn save(path: &Path, state: &G6State) -> Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    let tmp = with_suffix(path, ".tmp");
    std::fs::write(&tmp, json).with_context(|| format!("Failed to write {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("Failed to move {} to {}", tmp.display(), path.display()))?;
    Ok(())
}
