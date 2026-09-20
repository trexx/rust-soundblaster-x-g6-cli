// Tests of the Api layer against a recording FrameSink (no device needed).

use std::cell::RefCell;

use anyhow::Result;
use g6_cli::api::Api;
use g6_cli::device::FrameSink;
use g6_cli::model::sbx::ProfileName;
use g6_cli::model::{G6State, OutputMode};
use g6_cli::spec::{HidFrame, SbxEffect, SmartVolumeSpecial, MODE_DATA};

#[derive(Default)]
struct Recorder(RefCell<Vec<HidFrame>>);

impl FrameSink for Recorder {
    fn send(&self, frames: &[HidFrame]) -> Result<()> {
        self.0.borrow_mut().extend_from_slice(frames);
        Ok(())
    }
}

impl Recorder {
    fn take(&self) -> Vec<HidFrame> {
        std::mem::take(&mut *self.0.borrow_mut())
    }

    /// `(feature, value)` of every DATA frame recorded so far, then clear.
    fn take_data(&self) -> Vec<(u8, [u8; 4])> {
        self.take()
            .into_iter()
            .filter(|f| f.mode == MODE_DATA)
            .map(|f| (f.audio_feature, f.value))
            .collect()
    }
}

const ON: [u8; 4] = [0x00, 0x00, 0x80, 0x3f];
const OFF: [u8; 4] = [0; 4];
const PCT_50: [u8; 4] = [0x00, 0x00, 0x00, 0x3f];
const PCT_60: [u8; 4] = [0x9a, 0x99, 0x19, 0x3f];
const PCT_80: [u8; 4] = [0xcd, 0xcc, 0x4c, 0x3f];
const NIGHT: [u8; 4] = [0x00, 0x00, 0x00, 0x40];

#[test]
fn output_toggle_alternates_starting_with_speakers() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);

    api.output_toggle().unwrap();
    assert_eq!(api.state().output, Some(OutputMode::Speakers));
    assert_eq!(&sink.take()[0].to_hex()[..10], "5a2c050002");

    api.output_toggle().unwrap();
    assert_eq!(api.state().output, Some(OutputMode::Headphones));
    assert_eq!(&sink.take()[0].to_hex()[..10], "5a2c050004");

    api.output_toggle().unwrap();
    assert_eq!(api.state().output, Some(OutputMode::Speakers));
}

#[test]
fn output_toggle_from_saved_speakers_goes_to_headphones() {
    let sink = Recorder::default();
    let mut state = G6State {
        output: Some(OutputMode::Speakers),
        ..Default::default()
    };
    let mut api = Api::new(&sink, &mut state, None);
    api.output_toggle().unwrap();
    assert_eq!(api.state().output, Some(OutputMode::Headphones));
}

#[test]
fn sbx_effect_sends_toggle_and_slider_and_updates_only_that_profile() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);

    api.sbx_effect(ProfileName::Music, SbxEffect::Bass, true, Some(80), None)
        .unwrap();
    assert_eq!(sink.take_data(), vec![(0x18, ON), (0x19, PCT_80)]);

    let music = &api.state().sbx.music;
    assert!(music.bass.toggle);
    assert_eq!(music.bass.slider, 80);
    assert_eq!(
        api.state().sbx.gaming,
        Default::default(),
        "other profiles untouched"
    );
    assert_eq!(
        api.state().sbx.selected,
        ProfileName::Gaming,
        "selection unchanged"
    );
}

#[test]
fn sbx_effect_without_value_only_sends_toggle() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);

    api.sbx_effect(ProfileName::Gaming, SbxEffect::Surround, false, None, None)
        .unwrap();
    assert_eq!(sink.take_data(), vec![(0x00, OFF)]);
    assert_eq!(
        api.state().sbx.gaming.surround.slider,
        50,
        "slider keeps its default"
    );
}

#[test]
fn sbx_numeric_value_clears_smart_volume_special() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);

    api.sbx_effect(
        ProfileName::Gaming,
        SbxEffect::SmartVolume,
        true,
        None,
        Some(SmartVolumeSpecial::Night),
    )
    .unwrap();
    assert_eq!(sink.take_data(), vec![(0x04, ON), (0x06, NIGHT)]);
    assert_eq!(
        api.state().sbx.gaming.smart_volume.special,
        Some(SmartVolumeSpecial::Night)
    );

    api.sbx_effect(
        ProfileName::Gaming,
        SbxEffect::SmartVolume,
        true,
        Some(60),
        None,
    )
    .unwrap();
    assert_eq!(sink.take_data(), vec![(0x04, ON), (0x05, PCT_60)]);
    let sv = &api.state().sbx.gaming.smart_volume;
    assert_eq!(
        sv.special, None,
        "numeric value supersedes the special mode"
    );
    assert_eq!(sv.slider, 60);
}

#[test]
fn sbx_special_is_rejected_for_other_effects() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);

    let err = api
        .sbx_effect(
            ProfileName::Gaming,
            SbxEffect::Bass,
            true,
            None,
            Some(SmartVolumeSpecial::Loud),
        )
        .unwrap_err();
    assert!(err.to_string().contains("Smart Volume"), "{err}");
    assert!(sink.take().is_empty(), "nothing sent");
    assert_eq!(*api.state(), G6State::default(), "state unchanged");
}

#[test]
fn sbx_switch_replays_profile_in_fixed_order_with_slider() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    state.sbx.cinema.surround.toggle = true;
    state.sbx.cinema.bass.slider = 80;
    let mut api = Api::new(&sink, &mut state, None);

    api.sbx_switch(ProfileName::Cinema).unwrap();
    assert_eq!(
        sink.take_data(),
        vec![
            (0x00, ON),     // surround toggle
            (0x01, PCT_50), // surround slider
            (0x07, OFF),    // crystalizer toggle
            (0x08, PCT_50),
            (0x18, OFF), // bass toggle
            (0x19, PCT_80),
            (0x04, OFF),    // smart volume toggle
            (0x05, PCT_50), // smart volume slider (no special set)
            (0x02, OFF),    // dialog+ toggle
            (0x03, PCT_50),
        ]
    );
    assert_eq!(api.state().sbx.selected, ProfileName::Cinema);
}

#[test]
fn sbx_switch_sends_special_instead_of_slider_when_set() {
    let sink = Recorder::default();
    let mut state = G6State::default();
    state.sbx.special.smart_volume.special = Some(SmartVolumeSpecial::Night);
    let mut api = Api::new(&sink, &mut state, None);

    api.sbx_switch(ProfileName::Special).unwrap();
    let data = sink.take_data();
    assert_eq!(
        data[7],
        (0x06, NIGHT),
        "special replaces the 0x05 slider frame"
    );
    assert!(!data.iter().any(|(f, _)| *f == 0x05));
}

#[test]
fn api_persists_to_the_given_path() {
    let dir = std::env::temp_dir().join(format!("g6-cli-api-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.json");

    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, Some(&path));
    api.output_set(OutputMode::Headphones).unwrap();

    let reloaded = g6_cli::state::load(&path);
    assert_eq!(reloaded.output, Some(OutputMode::Headphones));
    std::fs::remove_dir_all(&dir).unwrap();
}

// ── Settings recording and replay ─────────────────────────────────────────────

#[test]
fn setters_record_their_last_value() {
    use g6_cli::model::{Lighting, Settings};
    use g6_cli::spec::decoder::DecoderMode;
    use g6_cli::spec::recording::{MicBoost, MicEqPreset, NoiseReductionLevel};
    use g6_cli::spec::PlaybackFilter;

    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);

    api.set_decoder_mode(DecoderMode::Night).unwrap();
    api.playback_direct_mode(true).unwrap();
    api.playback_spdif_direct_mode(false).unwrap();
    api.playback_filter(PlaybackFilter::SlowLinear).unwrap();
    api.lighting_rgb(255, 0, 128).unwrap();
    api.lighting_ring(false).unwrap();
    api.mic_boost(MicBoost::Db20).unwrap();
    api.mic_noise_reduction(true).unwrap();
    api.mic_noise_reduction_level(NoiseReductionLevel::Level60)
        .unwrap();
    api.mic_aec(true).unwrap();
    api.mic_smart_volume(false).unwrap();
    api.mic_eq(true).unwrap();
    api.mic_eq_preset(MicEqPreset::Preset3).unwrap();

    assert_eq!(
        api.state().settings,
        Settings {
            decoder: Some(DecoderMode::Night),
            direct_mode: Some(true),
            spdif_direct_mode: Some(false),
            playback_filter: Some(PlaybackFilter::SlowLinear),
            lighting: Some(Lighting::Rgb {
                red: 255,
                green: 0,
                blue: 128
            }),
            volume_ring: Some(false),
            mic_boost: Some(MicBoost::Db20),
            noise_reduction: Some(true),
            noise_reduction_level: Some(NoiseReductionLevel::Level60),
            aec: Some(true),
            mic_smart_volume: Some(false),
            mic_eq: Some(true),
            mic_eq_preset: Some(MicEqPreset::Preset3),
        }
    );

    api.lighting_off().unwrap();
    assert_eq!(api.state().settings.lighting, Some(Lighting::Off));
}

#[test]
fn apply_saved_replays_everything_in_a_fixed_order() {
    use g6_cli::model::{Lighting, Settings};
    use g6_cli::spec::decoder::{decoder_mode, DecoderMode};
    use g6_cli::spec::lighting::{lighting_enable_set_rgb, lighting_volume_ring};
    use g6_cli::spec::playback::{
        enable_direct_mode, enable_spdif_out_direct_mode, playback_filter, toggle_to_headphones,
    };
    use g6_cli::spec::recording::*;
    use g6_cli::spec::sbx::{sbx_slider, sbx_toggle};
    use g6_cli::spec::{AudioFeature, PlaybackFilter};

    let mut state = G6State {
        output: Some(OutputMode::Headphones),
        ..Default::default()
    };
    state.sbx.selected = ProfileName::Music;
    state.sbx.music.bass.toggle = true;
    state.sbx.music.bass.slider = 80;
    state.settings = Settings {
        decoder: Some(DecoderMode::Night),
        direct_mode: Some(true),
        spdif_direct_mode: Some(false),
        playback_filter: Some(PlaybackFilter::SlowLinear),
        lighting: Some(Lighting::Rgb {
            red: 255,
            green: 0,
            blue: 128,
        }),
        volume_ring: Some(false),
        mic_boost: Some(MicBoost::Db20),
        noise_reduction: Some(true),
        noise_reduction_level: Some(NoiseReductionLevel::Level60),
        aec: Some(true),
        mic_smart_volume: Some(false),
        mic_eq: Some(true),
        mic_eq_preset: Some(MicEqPreset::Preset3),
    };

    let sink = Recorder::default();
    let mut api = Api::new(&sink, &mut state, None);
    let applied = api.apply_saved().unwrap();
    assert_eq!(
        applied,
        [
            "output",
            "decoder",
            "direct-mode",
            "spdif-direct-mode",
            "playback-filter",
            "sbx",
            "lighting",
            "volume-ring",
            "mic-boost",
            "noise-reduction",
            "noise-reduction-level",
            "aec",
            "mic-smart-volume",
            "mic-eq",
            "mic-eq-preset",
        ]
    );

    let mut expected = toggle_to_headphones();
    expected.extend(decoder_mode(DecoderMode::Night));
    expected.extend(enable_direct_mode(true));
    expected.extend(enable_spdif_out_direct_mode(false));
    expected.extend(playback_filter(PlaybackFilter::SlowLinear));
    for (toggle, slider, on, pct) in [
        (
            AudioFeature::SurroundToggle,
            AudioFeature::SurroundSlider,
            false,
            50,
        ),
        (
            AudioFeature::CrystalizerToggle,
            AudioFeature::CrystalizerSlider,
            false,
            50,
        ),
        (AudioFeature::BassToggle, AudioFeature::BassSlider, true, 80),
        (
            AudioFeature::SmartVolumeToggle,
            AudioFeature::SmartVolumeSlider,
            false,
            50,
        ),
        (
            AudioFeature::DialogPlusToggle,
            AudioFeature::DialogPlusSlider,
            false,
            50,
        ),
    ] {
        expected.extend(sbx_toggle(toggle, on));
        expected.extend(sbx_slider(slider, pct));
    }
    expected.extend(lighting_enable_set_rgb(255, 0, 128));
    expected.extend(lighting_volume_ring(false));
    expected.extend(mic_boost(MicBoost::Db20));
    expected.extend(voice_clarity_noise_reduction(true));
    expected.extend(voice_clarity_noise_reduction_level(
        NoiseReductionLevel::Level60,
    ));
    expected.extend(voice_clarity_aec(true));
    expected.extend(voice_clarity_smart_volume(false));
    expected.extend(voice_clarity_mic_eq(true));
    expected.extend(voice_clarity_mic_eq_preset(MicEqPreset::Preset3));
    assert_eq!(sink.take(), expected);
    assert_eq!(api.state().sbx.selected, ProfileName::Music);
}

#[test]
fn apply_saved_skips_unset_settings_and_an_untouched_sbx_section() {
    use g6_cli::spec::decoder::{decoder_mode, DecoderMode};

    let sink = Recorder::default();
    let mut state = G6State::default();
    let mut api = Api::new(&sink, &mut state, None);
    assert!(api.apply_saved().unwrap().is_empty());
    assert!(sink.take().is_empty(), "a fresh state sends nothing");

    let mut state = G6State::default();
    state.settings.decoder = Some(DecoderMode::Full);
    let mut api = Api::new(&sink, &mut state, None);
    assert_eq!(api.apply_saved().unwrap(), ["decoder"]);
    assert_eq!(sink.take(), decoder_mode(DecoderMode::Full));

    let mut state = G6State::default();
    state.sbx.selected = ProfileName::Cinema;
    let mut api = Api::new(&sink, &mut state, None);
    assert_eq!(
        api.apply_saved().unwrap(),
        ["sbx"],
        "any SBX change makes the profile replay"
    );
    assert_eq!(sink.take_data().len(), 10);
}
