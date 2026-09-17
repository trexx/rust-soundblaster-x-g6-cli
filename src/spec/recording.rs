use serde::{Deserialize, Serialize};

use super::{f32_bytes, slider_percent_bytes, HidFrame, EMPTY_ADDITIONAL, MODE_COMMIT, MODE_DATA};

// ── Mic boost ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum MicBoost {
    #[value(name = "0")]
    Db0,
    #[value(name = "10")]
    Db10,
    #[value(name = "20")]
    Db20,
    #[value(name = "30")]
    Db30,
}

impl MicBoost {
    pub fn decibels(self) -> u8 {
        match self {
            Self::Db0 => 0,
            Self::Db10 => 10,
            Self::Db20 => 20,
            Self::Db30 => 30,
        }
    }

    /// Unlike every other value, mic boost is a little-endian `u32` of the dB figure.
    pub fn value_bytes(self) -> [u8; 4] {
        u32::from(self.decibels()).to_le_bytes()
    }
}

pub fn mic_boost(boost: MicBoost) -> Vec<HidFrame> {
    vec![
        HidFrame::new(
            [0x3c, 0x04],
            [0x00, 0x00],
            0x02,
            boost.value_bytes(),
            EMPTY_ADDITIONAL,
        ),
        HidFrame::new([0x3c, 0x02], [0x01, 0x00], 0x00, [0; 4], EMPTY_ADDITIONAL),
    ]
}

// ── Voice clarity ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum NoiseReductionLevel {
    #[value(name = "0")]
    Level0,
    #[value(name = "20")]
    Level20,
    #[value(name = "40")]
    Level40,
    #[value(name = "60")]
    Level60,
    #[value(name = "80")]
    Level80,
    #[value(name = "100")]
    Level100,
}

impl NoiseReductionLevel {
    pub fn percent(self) -> u8 {
        match self {
            Self::Level0 => 0,
            Self::Level20 => 20,
            Self::Level40 => 40,
            Self::Level60 => 60,
            Self::Level80 => 80,
            Self::Level100 => 100,
        }
    }

    /// Encoded as `percent / 200`, i.e. `0.0..=0.5`.
    pub fn value_bytes(self) -> [u8; 4] {
        f32_bytes(f32::from(self.percent()) / 200.0)
    }
}

fn toggle_recording_feature(audio_feature: u8, enable: bool) -> Vec<HidFrame> {
    let on_value = slider_percent_bytes(100);
    let off_value = [0u8; 4];
    vec![
        HidFrame::recording(
            MODE_DATA,
            audio_feature,
            if enable { on_value } else { off_value },
        ),
        HidFrame::recording(MODE_COMMIT, audio_feature, off_value),
    ]
}

pub fn voice_clarity_noise_reduction(enable: bool) -> Vec<HidFrame> {
    toggle_recording_feature(0x04, enable)
}

pub fn voice_clarity_noise_reduction_level(level: NoiseReductionLevel) -> Vec<HidFrame> {
    vec![
        HidFrame::recording(MODE_DATA, 0x05, level.value_bytes()),
        HidFrame::recording(MODE_COMMIT, 0x05, [0; 4]),
    ]
}

pub fn voice_clarity_aec(enable: bool) -> Vec<HidFrame> {
    toggle_recording_feature(0x00, enable)
}

pub fn voice_clarity_smart_volume(enable: bool) -> Vec<HidFrame> {
    toggle_recording_feature(0x2C, enable)
}

pub fn voice_clarity_mic_eq(enable: bool) -> Vec<HidFrame> {
    toggle_recording_feature(0x13, enable)
}

// ── Mic EQ presets ────────────────────────────────────────────────────────────

/// Audio-feature ids of the eight mic EQ bands, lowest band first.
const MIC_EQ_BAND_FEATURES: [u8; 8] = [0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B];

pub fn voice_clarity_mic_eq_preset(preset: MicEqPreset) -> Vec<HidFrame> {
    MIC_EQ_BAND_FEATURES
        .iter()
        .zip(preset.band_gains_db())
        .flat_map(|(&feat, gain)| {
            [
                HidFrame::recording(MODE_DATA, feat, f32_bytes(gain)),
                HidFrame::recording(MODE_COMMIT, feat, [0; 4]),
            ]
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum MicEqPreset {
    #[value(name = "preset-1", alias = "preset1")]
    Preset1,
    #[value(name = "preset-2", alias = "preset2")]
    Preset2,
    #[value(name = "preset-3", alias = "preset3")]
    Preset3,
    #[value(name = "preset-4", alias = "preset4")]
    Preset4,
    #[value(name = "preset-5", alias = "preset5")]
    Preset5,
    #[value(name = "preset-6", alias = "preset6")]
    Preset6,
    #[value(name = "preset-7", alias = "preset7")]
    Preset7,
    #[value(name = "preset-8", alias = "preset8")]
    Preset8,
    #[value(name = "preset-9", alias = "preset9")]
    Preset9,
    #[value(name = "preset-10", alias = "preset10")]
    Preset10,
    #[value(name = "preset-dm-1", alias = "preset-dm1")]
    PresetDm1,
}

impl MicEqPreset {
    /// Gain in dB for each of the eight bands (features `0x14..=0x1B`), as captured from
    /// Sound Blaster Command (`doc/payloads/raw/g6-recording-mic-eq-preset-*.txt`).
    #[rustfmt::skip]
    pub fn band_gains_db(self) -> [f32; 8] {
        match self {
            Self::Preset1   => [-3.0, -4.0,  0.0,  2.0,  3.0, -3.0,  4.0,  5.0],
            Self::Preset2   => [-3.0, -4.0,  0.0,  2.0,  4.0, -2.0,  2.0,  4.0],
            Self::Preset3   => [-2.0, -3.0,  3.0,  4.0,  4.0, -4.0,  3.0,  2.0],
            // Band 5 (0x18) is absent from the preset-4 capture; 0.0 is the value the
            // original Python tool used and has not been verified against the device.
            Self::Preset4   => [-3.0, -5.0,  0.0,  4.0,  0.0, -3.0,  0.0,  0.0],
            Self::Preset5   => [-2.0, -3.0,  2.0,  4.0,  4.0,  0.0, -3.0,  2.0],
            Self::Preset6   => [-5.0, -4.0, -2.0,  0.0,  3.0,  4.0,  6.0,  7.0],
            Self::Preset7   => [ 0.0,  3.0, -2.0, -4.0, -4.0, -2.0,  5.0,  7.0],
            Self::Preset8   => [ 0.0,  0.0,  2.0,  2.0,  3.0, -4.0,  2.0,  4.0],
            Self::Preset9   => [ 0.0,  0.0,  2.0,  2.0, -2.0,  0.0, -4.0,  4.0],
            Self::Preset10  => [ 0.0,  2.0, -2.0,  0.0,  3.0,  5.0,  6.0,  5.0],
            Self::PresetDm1 => [ 0.0,  8.0,  0.0, 12.0, 12.0,  4.0,  8.0, 10.0],
        }
    }
}
