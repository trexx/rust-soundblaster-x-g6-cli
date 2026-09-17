//! High-level operations. Every device-changing call sends its frames first and only then
//! records the change in memory and, when a persist path is set, on disk, so the saved
//! state never gets ahead of the device.

use std::path::Path;

use anyhow::{bail, Result};

use crate::device::FrameSink;
use crate::model::sbx::{ProfileName, SbxState};
use crate::model::{G6State, Lighting, OutputMode, Settings};
use crate::spec::decoder::{decoder_mode, DecoderMode};
use crate::spec::lighting::{lighting_disable, lighting_enable_set_rgb, lighting_volume_ring};
use crate::spec::playback::{
    enable_direct_mode, enable_spdif_out_direct_mode, playback_filter, toggle_to_headphones,
    toggle_to_speakers,
};
use crate::spec::recording::{
    mic_boost, voice_clarity_aec, voice_clarity_mic_eq, voice_clarity_mic_eq_preset,
    voice_clarity_noise_reduction, voice_clarity_noise_reduction_level, voice_clarity_smart_volume,
    MicBoost, MicEqPreset, NoiseReductionLevel,
};
use crate::spec::sbx::{sbx_slider, sbx_smart_volume_special, sbx_toggle};
use crate::spec::{HidFrame, PlaybackFilter, SbxEffect, SmartVolumeSpecial};
use crate::state;

pub struct Api<'a> {
    sink: &'a dyn FrameSink,
    state: &'a mut G6State,
    persist: Option<&'a Path>,
}

impl<'a> Api<'a> {
    /// `persist`: state file to rewrite after every change, or `None` to keep state in memory only.
    pub fn new(sink: &'a dyn FrameSink, state: &'a mut G6State, persist: Option<&'a Path>) -> Self {
        Self {
            sink,
            state,
            persist,
        }
    }

    pub fn state(&self) -> &G6State {
        self.state
    }

    fn send(&self, frames: Vec<HidFrame>) -> Result<()> {
        self.sink.send(&frames)
    }

    fn save(&self) -> Result<()> {
        match self.persist {
            Some(path) => state::save(path, self.state),
            None => Ok(()),
        }
    }

    /// Send `frames`, then record the setting they represent and save.
    fn set(&mut self, frames: Vec<HidFrame>, record: impl FnOnce(&mut Settings)) -> Result<()> {
        self.send(frames)?;
        record(&mut self.state.settings);
        self.save()
    }

    // ── Output ────────────────────────────────────────────────────────────────

    pub fn output_toggle(&mut self) -> Result<()> {
        let next = match self.state.output {
            Some(OutputMode::Headphones) | None => OutputMode::Speakers,
            Some(OutputMode::Speakers) => OutputMode::Headphones,
        };
        self.output_set(next)
    }

    pub fn output_set(&mut self, mode: OutputMode) -> Result<()> {
        let frames = match mode {
            OutputMode::Speakers => toggle_to_speakers(),
            OutputMode::Headphones => toggle_to_headphones(),
        };
        self.send(frames)?;
        self.state.output = Some(mode);
        self.save()
    }

    // ── Decoder ───────────────────────────────────────────────────────────────

    pub fn set_decoder_mode(&mut self, mode: DecoderMode) -> Result<()> {
        self.set(decoder_mode(mode), |s| s.decoder = Some(mode))
    }

    // ── Lighting ──────────────────────────────────────────────────────────────

    pub fn lighting_off(&mut self) -> Result<()> {
        self.set(lighting_disable(), |s| s.lighting = Some(Lighting::Off))
    }

    pub fn lighting_rgb(&mut self, red: u8, green: u8, blue: u8) -> Result<()> {
        self.set(lighting_enable_set_rgb(red, green, blue), |s| {
            s.lighting = Some(Lighting::Rgb { red, green, blue })
        })
    }

    pub fn lighting_ring(&mut self, enable: bool) -> Result<()> {
        self.set(lighting_volume_ring(enable), |s| {
            s.volume_ring = Some(enable)
        })
    }

    // ── Playback ──────────────────────────────────────────────────────────────

    pub fn playback_direct_mode(&mut self, enable: bool) -> Result<()> {
        self.set(enable_direct_mode(enable), |s| s.direct_mode = Some(enable))
    }

    pub fn playback_spdif_direct_mode(&mut self, enable: bool) -> Result<()> {
        self.set(enable_spdif_out_direct_mode(enable), |s| {
            s.spdif_direct_mode = Some(enable)
        })
    }

    pub fn playback_filter(&mut self, filter: PlaybackFilter) -> Result<()> {
        self.set(playback_filter(filter), |s| {
            s.playback_filter = Some(filter)
        })
    }

    // ── Mic ───────────────────────────────────────────────────────────────────

    pub fn mic_boost(&mut self, boost: MicBoost) -> Result<()> {
        self.set(mic_boost(boost), |s| s.mic_boost = Some(boost))
    }

    pub fn mic_noise_reduction(&mut self, enable: bool) -> Result<()> {
        self.set(voice_clarity_noise_reduction(enable), |s| {
            s.noise_reduction = Some(enable)
        })
    }

    pub fn mic_noise_reduction_level(&mut self, level: NoiseReductionLevel) -> Result<()> {
        self.set(voice_clarity_noise_reduction_level(level), |s| {
            s.noise_reduction_level = Some(level)
        })
    }

    pub fn mic_aec(&mut self, enable: bool) -> Result<()> {
        self.set(voice_clarity_aec(enable), |s| s.aec = Some(enable))
    }

    pub fn mic_smart_volume(&mut self, enable: bool) -> Result<()> {
        self.set(voice_clarity_smart_volume(enable), |s| {
            s.mic_smart_volume = Some(enable)
        })
    }

    pub fn mic_eq(&mut self, enable: bool) -> Result<()> {
        self.set(voice_clarity_mic_eq(enable), |s| s.mic_eq = Some(enable))
    }

    pub fn mic_eq_preset(&mut self, preset: MicEqPreset) -> Result<()> {
        self.set(voice_clarity_mic_eq_preset(preset), |s| {
            s.mic_eq_preset = Some(preset)
        })
    }

    // ── SBX ───────────────────────────────────────────────────────────────────

    pub fn sbx_current(&self) -> ProfileName {
        self.state.sbx.selected
    }

    /// Replay every stored setting of `profile` to the device and make it the selected one.
    pub fn sbx_switch(&mut self, profile: ProfileName) -> Result<()> {
        let p = self.state.sbx.profile(profile).clone();
        for effect in SbxEffect::ALL {
            self.send(sbx_toggle(effect.toggle_feature(), p.toggle(effect)))?;
            match (effect, p.smart_volume.special) {
                (SbxEffect::SmartVolume, Some(special)) => {
                    self.send(sbx_smart_volume_special(special))?;
                }
                _ => self.send(sbx_slider(effect.slider_feature(), p.slider(effect)))?,
            }
        }
        self.state.sbx.selected = profile;
        self.save()
    }

    /// Switch `effect` on or off in `profile`, and optionally set its strength: a numeric
    /// `value` (0–100) or, for Smart Volume only, a `special` mode.
    pub fn sbx_effect(
        &mut self,
        profile: ProfileName,
        effect: SbxEffect,
        enable: bool,
        value: Option<u8>,
        special: Option<SmartVolumeSpecial>,
    ) -> Result<()> {
        if special.is_some() && effect != SbxEffect::SmartVolume {
            bail!("special modes (Night/Loud) only apply to Smart Volume, not {effect:?}");
        }

        self.send(sbx_toggle(effect.toggle_feature(), enable))?;
        *self.state.sbx.profile_mut(profile).toggle_mut(effect) = enable;
        self.save()?;

        if let Some(special) = special {
            self.send(sbx_smart_volume_special(special))?;
            self.state.sbx.profile_mut(profile).smart_volume.special = Some(special);
            self.save()?;
        } else if let Some(value) = value {
            self.send(sbx_slider(effect.slider_feature(), value))?;
            let p = self.state.sbx.profile_mut(profile);
            *p.slider_mut(effect) = value;
            if effect == SbxEffect::SmartVolume {
                // A numeric value supersedes any previously chosen special mode.
                p.smart_volume.special = None;
            }
            self.save()?;
        }
        Ok(())
    }

    // ── Replay ────────────────────────────────────────────────────────────────

    /// Send every saved setting to the device, in a fixed order. Settings that were never set
    /// are skipped, and the SBX profile is only replayed once any SBX setting has been changed
    /// (a fresh profile must not push "all effects off" to the device). Returns the names of
    /// the settings that were applied; empty when nothing has been saved yet.
    pub fn apply_saved(&mut self) -> Result<Vec<&'static str>> {
        let output = self.state.output;
        let sbx_touched = self.state.sbx != SbxState::default();
        let selected = self.state.sbx.selected;
        let s = self.state.settings.clone();
        let mut applied = Vec::new();

        if let Some(mode) = output {
            self.output_set(mode)?;
            applied.push("output");
        }
        if let Some(mode) = s.decoder {
            self.set_decoder_mode(mode)?;
            applied.push("decoder");
        }
        if let Some(on) = s.direct_mode {
            self.playback_direct_mode(on)?;
            applied.push("direct-mode");
        }
        if let Some(on) = s.spdif_direct_mode {
            self.playback_spdif_direct_mode(on)?;
            applied.push("spdif-direct-mode");
        }
        if let Some(filter) = s.playback_filter {
            self.playback_filter(filter)?;
            applied.push("playback-filter");
        }
        if sbx_touched {
            self.sbx_switch(selected)?;
            applied.push("sbx");
        }
        if let Some(lighting) = s.lighting {
            match lighting {
                Lighting::Off => self.lighting_off()?,
                Lighting::Rgb { red, green, blue } => self.lighting_rgb(red, green, blue)?,
            }
            applied.push("lighting");
        }
        if let Some(on) = s.volume_ring {
            self.lighting_ring(on)?;
            applied.push("volume-ring");
        }
        if let Some(boost) = s.mic_boost {
            self.mic_boost(boost)?;
            applied.push("mic-boost");
        }
        if let Some(on) = s.noise_reduction {
            self.mic_noise_reduction(on)?;
            applied.push("noise-reduction");
        }
        if let Some(level) = s.noise_reduction_level {
            self.mic_noise_reduction_level(level)?;
            applied.push("noise-reduction-level");
        }
        if let Some(on) = s.aec {
            self.mic_aec(on)?;
            applied.push("aec");
        }
        if let Some(on) = s.mic_smart_volume {
            self.mic_smart_volume(on)?;
            applied.push("mic-smart-volume");
        }
        if let Some(on) = s.mic_eq {
            self.mic_eq(on)?;
            applied.push("mic-eq");
        }
        if let Some(preset) = s.mic_eq_preset {
            self.mic_eq_preset(preset)?;
            applied.push("mic-eq-preset");
        }
        Ok(applied)
    }
}
