use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::model::sbx::ProfileName;
use crate::model::OutputMode;
use crate::spec::decoder::DecoderMode;
use crate::spec::recording::{MicBoost, MicEqPreset, NoiseReductionLevel};
use crate::spec::{PlaybackFilter, SbxEffect, SmartVolumeSpecial};
use crate::watch::WatchArgs;

#[derive(Debug, Parser)]
#[command(
    name = "g6-cli",
    about = "SoundBlaster X G6 CLI (Windows, HID)",
    version
)]
pub struct Cli {
    /// Print the HID frames that would be sent (one hex line each) without opening the device
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Print raw HID frames and device responses to stderr
    #[arg(long, global = true)]
    pub debug: bool,

    /// Do not read or write the state file
    #[arg(long, global = true)]
    pub no_persist: bool,

    /// State file to use instead of state.json next to the executable
    #[arg(long, global = true, env = "G6_CLI_STATE", value_name = "PATH")]
    pub state: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Switch playback output between Speakers and Headphones
    Output {
        #[command(subcommand)]
        action: OutputAction,
    },
    /// Set the digital decoder mode
    Decoder { mode: DecoderMode },
    /// Control device lighting
    Lighting {
        #[command(subcommand)]
        action: LightingAction,
    },
    /// Control HID playback settings
    Playback {
        #[command(subcommand)]
        action: PlaybackAction,
    },
    /// Control microphone settings
    Mic {
        #[command(subcommand)]
        action: MicAction,
    },
    /// Control SBX sound effects
    Sbx {
        #[command(subcommand)]
        action: SbxAction,
    },
    /// Send every saved setting to the device now
    Apply,
    /// Keep running and apply the saved settings whenever the G6 connects
    Watch(WatchArgs),
    /// Start the watcher automatically at logon (Windows only)
    Autostart {
        #[command(subcommand)]
        action: AutostartAction,
    },
}

impl Command {
    /// Whether running this command sends frames through the sink opened by `main`.
    /// `watch` opens the device itself when it needs it.
    pub fn sends_frames(&self) -> bool {
        !matches!(
            self,
            Command::Sbx {
                action: SbxAction::Current
            } | Command::Watch(_)
                | Command::Autostart { .. }
        )
    }
}

// ── Output subcommands ────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum OutputAction {
    /// Toggle between Speakers and Headphones
    Toggle,
    /// Set output to a specific mode
    Set { mode: OutputMode },
}

// ── Lighting subcommands ──────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum LightingAction {
    /// Disable device lighting
    Off,
    /// Enable lighting and set RGB colour
    Rgb {
        #[arg(value_name = "R")]
        red: u8,
        #[arg(value_name = "G")]
        green: u8,
        #[arg(value_name = "B")]
        blue: u8,
    },
    /// Enable or disable the volume ring LED
    Ring { enable: OnOff },
}

// ── Playback subcommands ──────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum PlaybackAction {
    /// Enable or disable Direct Mode
    Direct { enable: OnOff },
    /// Enable or disable SPDIF-Out Direct Mode
    SpdifDirect { enable: OnOff },
    /// Set the DAC playback filter
    Filter { filter: PlaybackFilter },
}

// ── Mic subcommands ───────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum MicAction {
    /// Set mic boost in dB
    Boost {
        #[arg(value_name = "dB")]
        db: MicBoost,
    },
    /// Enable or disable noise reduction
    NoiseReduction {
        enable: OnOff,
        /// Noise reduction level in percent
        #[arg(long, value_name = "level")]
        level: Option<NoiseReductionLevel>,
    },
    /// Enable or disable Acoustic Echo Cancellation
    Aec { enable: OnOff },
    /// Enable or disable Smart Volume
    SmartVolume { enable: OnOff },
    /// Enable or disable microphone equalizer
    Eq {
        enable: OnOff,
        /// Apply an EQ preset
        #[arg(long, value_name = "preset")]
        preset: Option<MicEqPreset>,
    },
}

// ── SBX subcommands ───────────────────────────────────────────────────────────

/// Arguments shared by every SBX effect subcommand.
#[derive(Debug, Args, PartialEq, Eq)]
pub struct EffectArgs {
    pub profile: ProfileName,
    pub enable: OnOff,
    /// Effect strength, 0-100
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub value: Option<u8>,
}

#[derive(Debug, Subcommand)]
pub enum SbxAction {
    /// Switch to a saved SBX profile (sends all stored settings to device)
    Switch { profile: ProfileName },
    /// Print the currently active SBX profile name
    Current,
    /// Control Surround effect
    Surround(EffectArgs),
    /// Control Crystalizer effect
    Crystalizer(EffectArgs),
    /// Control Bass effect
    Bass(EffectArgs),
    /// Control Smart Volume effect
    SmartVolume {
        #[command(flatten)]
        args: EffectArgs,
        /// Use Night or Loud special mode instead of a numeric value
        #[arg(long, conflicts_with = "value")]
        special: Option<SmartVolumeSpecial>,
    },
    /// Control Dialog Plus effect
    DialogPlus(EffectArgs),
}

impl SbxAction {
    /// For the effect subcommands: which effect, its arguments, and the Smart Volume special mode.
    pub fn effect(&self) -> Option<(SbxEffect, &EffectArgs, Option<SmartVolumeSpecial>)> {
        match self {
            Self::Surround(a) => Some((SbxEffect::Surround, a, None)),
            Self::Crystalizer(a) => Some((SbxEffect::Crystalizer, a, None)),
            Self::Bass(a) => Some((SbxEffect::Bass, a, None)),
            Self::SmartVolume { args, special } => Some((SbxEffect::SmartVolume, args, *special)),
            Self::DialogPlus(a) => Some((SbxEffect::DialogPlus, a, None)),
            Self::Switch { .. } | Self::Current => None,
        }
    }
}

// ── Autostart subcommands ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum AutostartAction {
    /// Register g6-watch.exe to start at logon for the current user
    Enable,
    /// Remove the logon registration
    Disable,
    /// Show whether autostart is registered
    Status,
}

// ── on/off value enum ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OnOff {
    On,
    Off,
}

impl From<OnOff> for bool {
    fn from(v: OnOff) -> bool {
        matches!(v, OnOff::On)
    }
}
