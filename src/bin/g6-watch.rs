//! Console-less companion of `g6-cli`: runs `g6-cli watch` at logon (see `g6-cli autostart`).
//! Because it has no console, it always logs to a file: `--log`, or `g6-watch.log` next to
//! the state file.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use g6_cli::state;
use g6_cli::watch::{self, WatchArgs};

#[derive(Parser)]
#[command(
    name = "g6-watch",
    about = "Applies the saved g6-cli settings whenever the SoundBlaster X G6 connects",
    version
)]
struct Cli {
    #[command(flatten)]
    watch: WatchArgs,

    /// State file to use instead of state.json next to the executable
    #[arg(long, env = "G6_CLI_STATE", value_name = "PATH")]
    state: Option<PathBuf>,

    /// Print the HID frames instead of sending them
    #[arg(long)]
    dry_run: bool,
}

fn main() {
    let cli = Cli::parse();
    let state_path = cli.state.unwrap_or_else(state::default_path);
    let log_path = cli
        .watch
        .log
        .clone()
        .unwrap_or_else(|| state_path.with_file_name("g6-watch.log"));

    let mut log = match watch::open_log(Some(&log_path)) {
        Ok(log) => log,
        Err(_) => std::process::exit(2),
    };
    if let Err(e) = watch::run(
        &cli.watch,
        Some(&state_path),
        cli.dry_run,
        false,
        log.as_mut(),
    ) {
        let _ = writeln!(log, "{} fatal: {e:#}", watch::timestamp());
        std::process::exit(1);
    }
}
