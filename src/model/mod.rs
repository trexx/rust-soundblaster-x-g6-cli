//! Persisted state: the last output selection, the four SBX profiles, and the last value of
//! every other setting the CLI has sent (replayed by `apply` and the watcher).

pub mod sbx;

use serde::{Deserialize, Serialize};

use crate::model::sbx::SbxState;
use crate::spec::decoder::DecoderMode;
use crate::spec::recording::{MicBoost, MicEqPreset, NoiseReductionLevel};
use crate::spec::PlaybackFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "PascalCase")]
pub enum OutputMode {
    Speakers,
    Headphones,
}

/// Device lighting as last set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lighting {
    Off,
    Rgb { red: u8, green: u8, blue: u8 },
}

/// Last value of every setting the CLI has sent. `None` means "never set", and `apply`
/// leaves such settings alone.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub decoder: Option<DecoderMode>,
    pub direct_mode: Option<bool>,
    pub spdif_direct_mode: Option<bool>,
    pub playback_filter: Option<PlaybackFilter>,
    pub lighting: Option<Lighting>,
    pub volume_ring: Option<bool>,
    pub mic_boost: Option<MicBoost>,
    pub noise_reduction: Option<bool>,
    pub noise_reduction_level: Option<NoiseReductionLevel>,
    pub aec: Option<bool>,
    pub mic_smart_volume: Option<bool>,
    pub mic_eq: Option<bool>,
    pub mic_eq_preset: Option<MicEqPreset>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct G6State {
    /// Last output set by this tool, so `output toggle` can alternate correctly.
    pub output: Option<OutputMode>,
    pub sbx: SbxState,
    pub settings: Settings,
}
