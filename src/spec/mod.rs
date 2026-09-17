//! Wire format of the G6 HID protocol: frame layout, value encoding and feature ids.
//!
//! Every numeric value the G6 accepts is a little-endian IEEE-754 `f32` (sliders are
//! `0.0..=1.0`, discrete modes are small whole numbers such as `1.0`, `2.0`, `3.0`), except
//! mic boost, which is a little-endian `u32` of the dB value.

pub mod decoder;
pub mod lighting;
pub mod playback;
pub mod recording;
pub mod sbx;

use serde::{Deserialize, Serialize};

// Static frame constants
pub const STATIC_PREFIX: u8 = 0x5A;
pub const PLAYBACK_INTERMEDIATE: [u8; 2] = [0x01, 0x96];
pub const RECORDING_INTERMEDIATE: [u8; 2] = [0x01, 0x95];
pub const DECODER_INTERMEDIATE: [u8; 2] = [0x01, 0x97];
pub const MODE_DATA: [u8; 2] = [0x12, 0x07];
pub const MODE_COMMIT: [u8; 2] = [0x11, 0x03];
pub const EMPTY_ADDITIONAL: [u8; 54] = [0u8; 54];

/// A 64-byte HID payload. Sent to the device prepended with a 0x00 report ID byte (65 bytes total).
/// Layout: PREFIX(1) | mode(2) | intermediate(2) | audio_feature(1) | value(4) | additional(54) = 64
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidFrame {
    pub mode: [u8; 2],
    pub intermediate: [u8; 2],
    pub audio_feature: u8,
    pub value: [u8; 4],
    pub additional: [u8; 54],
}

impl HidFrame {
    pub const LEN: usize = 64;

    pub fn new(
        mode: [u8; 2],
        intermediate: [u8; 2],
        audio_feature: u8,
        value: [u8; 4],
        additional: [u8; 54],
    ) -> Self {
        Self {
            mode,
            intermediate,
            audio_feature,
            value,
            additional,
        }
    }

    /// Standard playback/SBX frame using PLAYBACK_INTERMEDIATE.
    pub fn playback(mode: [u8; 2], audio_feature: u8, value: [u8; 4]) -> Self {
        Self::new(
            mode,
            PLAYBACK_INTERMEDIATE,
            audio_feature,
            value,
            EMPTY_ADDITIONAL,
        )
    }

    /// Standard recording frame using RECORDING_INTERMEDIATE.
    pub fn recording(mode: [u8; 2], audio_feature: u8, value: [u8; 4]) -> Self {
        Self::new(
            mode,
            RECORDING_INTERMEDIATE,
            audio_feature,
            value,
            EMPTY_ADDITIONAL,
        )
    }

    /// Standard decoder frame using DECODER_INTERMEDIATE.
    pub fn decoder(mode: [u8; 2], audio_feature: u8, value: [u8; 4]) -> Self {
        Self::new(
            mode,
            DECODER_INTERMEDIATE,
            audio_feature,
            value,
            EMPTY_ADDITIONAL,
        )
    }

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut buf = [0u8; Self::LEN];
        buf[0] = STATIC_PREFIX;
        buf[1..3].copy_from_slice(&self.mode);
        buf[3..5].copy_from_slice(&self.intermediate);
        buf[5] = self.audio_feature;
        buf[6..10].copy_from_slice(&self.value);
        buf[10..].copy_from_slice(&self.additional);
        buf
    }

    pub fn to_hex(&self) -> String {
        self.to_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ── Value encoding ────────────────────────────────────────────────────────────

/// Encode a value as the little-endian IEEE-754 `f32` the G6 uses in its value field.
pub const fn f32_bytes(v: f32) -> [u8; 4] {
    v.to_le_bytes()
}

/// Percent `0..=100` → value bytes for SBX sliders and on/off toggles (`0.0` at 0 %, `1.0` at 100 %).
///
/// # Panics
/// If `pct > 100`.
pub fn slider_percent_bytes(pct: u8) -> [u8; 4] {
    assert!(pct <= 100, "slider percent must be 0..=100, got {pct}");
    f32_bytes(f32::from(pct) / 100.0)
}

// ── SBX audio feature bytes ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFeature {
    SurroundToggle = 0x00,
    SurroundSlider = 0x01,
    DialogPlusToggle = 0x02,
    DialogPlusSlider = 0x03,
    SmartVolumeToggle = 0x04,
    SmartVolumeSlider = 0x05,
    SmartVolumeSpecial = 0x06,
    CrystalizerToggle = 0x07,
    CrystalizerSlider = 0x08,
    BassToggle = 0x18,
    BassSlider = 0x19,
}

impl AudioFeature {
    pub fn byte(self) -> u8 {
        self as u8
    }
}

/// One SBX sound effect. Each has an on/off toggle and a 0–100 strength slider;
/// Smart Volume additionally has the Night/Loud special modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbxEffect {
    Surround,
    Crystalizer,
    Bass,
    SmartVolume,
    DialogPlus,
}

impl SbxEffect {
    /// All effects, in the order a profile switch sends them.
    pub const ALL: [SbxEffect; 5] = [
        Self::Surround,
        Self::Crystalizer,
        Self::Bass,
        Self::SmartVolume,
        Self::DialogPlus,
    ];

    pub fn toggle_feature(self) -> AudioFeature {
        match self {
            Self::Surround => AudioFeature::SurroundToggle,
            Self::Crystalizer => AudioFeature::CrystalizerToggle,
            Self::Bass => AudioFeature::BassToggle,
            Self::SmartVolume => AudioFeature::SmartVolumeToggle,
            Self::DialogPlus => AudioFeature::DialogPlusToggle,
        }
    }

    pub fn slider_feature(self) -> AudioFeature {
        match self {
            Self::Surround => AudioFeature::SurroundSlider,
            Self::Crystalizer => AudioFeature::CrystalizerSlider,
            Self::Bass => AudioFeature::BassSlider,
            Self::SmartVolume => AudioFeature::SmartVolumeSlider,
            Self::DialogPlus => AudioFeature::DialogPlusSlider,
        }
    }
}

// ── SmartVolumeSpecial ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum SmartVolumeSpecial {
    Night,
    Loud,
}

impl SmartVolumeSpecial {
    pub fn value_bytes(self) -> [u8; 4] {
        f32_bytes(match self {
            Self::Night => 2.0,
            Self::Loud => 1.0,
        })
    }
}

// ── PlaybackFilter ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum PlaybackFilter {
    /// Fast roll-off, minimum phase
    #[value(name = "fast-min")]
    FastMinimum,
    /// Slow roll-off, minimum phase
    #[value(name = "slow-min")]
    SlowMinimum,
    /// Fast roll-off, linear phase
    #[value(name = "fast-lin")]
    FastLinear,
    /// Slow roll-off, linear phase
    #[value(name = "slow-lin")]
    SlowLinear,
}

impl PlaybackFilter {
    /// The filter value is used as the HID frame's *intermediate* field, not the value field.
    pub fn intermediate_bytes(self) -> [u8; 2] {
        match self {
            Self::FastMinimum => [0x00, 0x01],
            Self::SlowMinimum => [0x00, 0x02],
            Self::FastLinear => [0x00, 0x04],
            Self::SlowLinear => [0x00, 0x05],
        }
    }
}
