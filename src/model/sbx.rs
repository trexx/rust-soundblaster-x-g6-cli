use serde::{Deserialize, Serialize};

use crate::spec::{SbxEffect, SmartVolumeSpecial};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "PascalCase")]
pub enum ProfileName {
    #[default]
    Gaming,
    Music,
    Cinema,
    Special,
}

impl std::fmt::Display for ProfileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Gaming => "Gaming",
            Self::Music => "Music",
            Self::Cinema => "Cinema",
            Self::Special => "Special",
        })
    }
}

/// Default slider position for every effect in a fresh profile.
const DEFAULT_SLIDER: u8 = 50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SbxFeature {
    pub toggle: bool,
    pub slider: u8,
}

impl Default for SbxFeature {
    fn default() -> Self {
        Self {
            toggle: false,
            slider: DEFAULT_SLIDER,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartVolumeFeature {
    pub toggle: bool,
    pub slider: u8,
    /// When set, a profile switch sends this mode instead of `slider`.
    pub special: Option<SmartVolumeSpecial>,
}

impl Default for SmartVolumeFeature {
    fn default() -> Self {
        Self {
            toggle: false,
            slider: DEFAULT_SLIDER,
            special: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SbxProfile {
    pub surround: SbxFeature,
    pub crystalizer: SbxFeature,
    pub bass: SbxFeature,
    pub smart_volume: SmartVolumeFeature,
    pub dialog_plus: SbxFeature,
}

impl SbxProfile {
    pub fn toggle(&self, effect: SbxEffect) -> bool {
        match effect {
            SbxEffect::Surround => self.surround.toggle,
            SbxEffect::Crystalizer => self.crystalizer.toggle,
            SbxEffect::Bass => self.bass.toggle,
            SbxEffect::SmartVolume => self.smart_volume.toggle,
            SbxEffect::DialogPlus => self.dialog_plus.toggle,
        }
    }

    pub fn slider(&self, effect: SbxEffect) -> u8 {
        match effect {
            SbxEffect::Surround => self.surround.slider,
            SbxEffect::Crystalizer => self.crystalizer.slider,
            SbxEffect::Bass => self.bass.slider,
            SbxEffect::SmartVolume => self.smart_volume.slider,
            SbxEffect::DialogPlus => self.dialog_plus.slider,
        }
    }

    pub fn toggle_mut(&mut self, effect: SbxEffect) -> &mut bool {
        match effect {
            SbxEffect::Surround => &mut self.surround.toggle,
            SbxEffect::Crystalizer => &mut self.crystalizer.toggle,
            SbxEffect::Bass => &mut self.bass.toggle,
            SbxEffect::SmartVolume => &mut self.smart_volume.toggle,
            SbxEffect::DialogPlus => &mut self.dialog_plus.toggle,
        }
    }

    pub fn slider_mut(&mut self, effect: SbxEffect) -> &mut u8 {
        match effect {
            SbxEffect::Surround => &mut self.surround.slider,
            SbxEffect::Crystalizer => &mut self.crystalizer.slider,
            SbxEffect::Bass => &mut self.bass.slider,
            SbxEffect::SmartVolume => &mut self.smart_volume.slider,
            SbxEffect::DialogPlus => &mut self.dialog_plus.slider,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SbxState {
    pub selected: ProfileName,
    pub gaming: SbxProfile,
    pub music: SbxProfile,
    pub cinema: SbxProfile,
    pub special: SbxProfile,
}

impl SbxState {
    pub fn profile(&self, name: ProfileName) -> &SbxProfile {
        match name {
            ProfileName::Gaming => &self.gaming,
            ProfileName::Music => &self.music,
            ProfileName::Cinema => &self.cinema,
            ProfileName::Special => &self.special,
        }
    }

    pub fn profile_mut(&mut self, name: ProfileName) -> &mut SbxProfile {
        match name {
            ProfileName::Gaming => &mut self.gaming,
            ProfileName::Music => &mut self.music,
            ProfileName::Cinema => &mut self.cinema,
            ProfileName::Special => &mut self.special,
        }
    }
}
