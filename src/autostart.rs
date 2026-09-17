//! Register the console-less companion `g6-watch.exe` to start at logon for the current
//! user, via the `HKCU\...\Run` registry key. Windows only; uses `reg.exe`, so no extra crate.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const WATCHER_EXE: &str = "g6-watch.exe";
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "g6-cli";

/// The command line registered for logon: the watcher next to `exe_dir`, plus `--state`
/// when a custom state file is in use.
pub fn run_command_line(exe_dir: &Path, state: Option<&Path>) -> String {
    let mut cmd = format!("\"{}\"", exe_dir.join(WATCHER_EXE).display());
    if let Some(state) = state {
        cmd.push_str(&format!(" --state \"{}\"", state.display()));
    }
    cmd
}

/// The value part of a `reg query … /v name` output line, e.g. `    g6-cli    REG_SZ    "C:\…"`.
pub fn parse_reg_value(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.split_once("REG_SZ"))
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn enable(state: Option<&Path>) -> Result<()> {
    ensure_supported()?;
    let exe_dir = exe_dir()?;
    let watcher = exe_dir.join(WATCHER_EXE);
    if !watcher.exists() {
        bail!(
            "{} not found; keep {WATCHER_EXE} next to g6-cli.exe (it is part of the release)",
            watcher.display()
        );
    }
    let cmd = run_command_line(&exe_dir, state);
    reg(&[
        "add", RUN_KEY, "/v", VALUE_NAME, "/t", "REG_SZ", "/d", &cmd, "/f",
    ])?;
    println!("Autostart enabled: {cmd}");
    Ok(())
}

pub fn disable() -> Result<()> {
    ensure_supported()?;
    if reg(&["query", RUN_KEY, "/v", VALUE_NAME]).is_err() {
        println!("Autostart was not enabled");
        return Ok(());
    }
    reg(&["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])?;
    println!("Autostart disabled");
    Ok(())
}

pub fn status() -> Result<()> {
    ensure_supported()?;
    match reg(&["query", RUN_KEY, "/v", VALUE_NAME]) {
        Ok(output) => {
            let value = parse_reg_value(&output).unwrap_or_else(|| output.trim().to_owned());
            println!("Autostart enabled: {value}");
        }
        Err(_) => println!("Autostart not enabled"),
    }
    Ok(())
}

fn ensure_supported() -> Result<()> {
    if cfg!(windows) {
        Ok(())
    } else {
        bail!("autostart is only supported on Windows; start `g6-cli watch` from your session manager instead")
    }
}

fn exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Failed to locate the running executable")?;
    exe.parent()
        .map(Path::to_path_buf)
        .context("Failed to determine the executable's directory")
}

/// Run `reg.exe` with `args`; `Err` if it cannot be started or exits unsuccessfully.
fn reg(args: &[&str]) -> Result<String> {
    let output = Command::new("reg")
        .args(args)
        .output()
        .context("Failed to run reg.exe")?;
    if !output.status.success() {
        bail!(
            "reg.exe {} failed: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
