// Byte-level tests of the HID wire format (no device needed). Expected bytes come from the
// Wireshark captures in doc/payloads/ and the tables in doc/usb-spec.md.

use g6_cli::spec::decoder::{decoder_mode, DecoderMode};
use g6_cli::spec::lighting::{lighting_disable, lighting_enable_set_rgb, lighting_volume_ring};
use g6_cli::spec::playback::{
    enable_direct_mode, enable_spdif_out_direct_mode, playback_filter, toggle_to_headphones,
    toggle_to_speakers,
};
use g6_cli::spec::recording::{
    mic_boost, voice_clarity_aec, voice_clarity_mic_eq, voice_clarity_mic_eq_preset,
    voice_clarity_noise_reduction, voice_clarity_noise_reduction_level, voice_clarity_smart_volume,
    MicBoost, MicEqPreset, NoiseReductionLevel,
};
use g6_cli::spec::sbx::{sbx_slider, sbx_smart_volume_special, sbx_toggle};
use g6_cli::spec::{
    slider_percent_bytes, AudioFeature, HidFrame, PlaybackFilter, SmartVolumeSpecial,
};

fn hex4(b: [u8; 4]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_lines(frames: &[HidFrame]) -> Vec<String> {
    frames.iter().map(HidFrame::to_hex).collect()
}

fn golden(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Hex of a frame's 4-byte value field.
fn value(frame: &HidFrame) -> String {
    hex4(frame.value)
}

/// Hex of the fixed 12-character head: prefix | mode | intermediate | feature.
fn head(frame: &HidFrame) -> String {
    frame.to_hex()[..12].to_owned()
}

// ── Value encoding (doc/usb-spec.md "Numeric values" table) ───────────────────

#[test]
fn slider_percent_encoding_matches_spec_table() {
    for (pct, expected) in [
        (0, "00000000"),
        (1, "0ad7233c"),
        (2, "0ad7a33c"),
        (3, "8fc2f53c"),
        (25, "0000803e"),
        (33, "c3f5a83e"),
        (50, "0000003f"),
        (80, "cdcc4c3f"),
        (99, "a4707d3f"),
        (100, "0000803f"),
    ] {
        assert_eq!(hex4(slider_percent_bytes(pct)), expected, "slider {pct}%");
    }
}

#[test]
#[should_panic(expected = "0..=100")]
fn slider_percent_over_100_panics() {
    slider_percent_bytes(101);
}

#[test]
fn noise_reduction_levels_match_captures() {
    use NoiseReductionLevel::*;
    for (level, expected) in [
        (Level0, "00000000"),
        (Level20, "cdcccc3d"),
        (Level40, "cdcc4c3e"),
        (Level60, "9a99993e"),
        (Level80, "cdcccc3e"),
        (Level100, "0000003f"),
    ] {
        assert_eq!(hex4(level.value_bytes()), expected, "{level:?}");
    }
}

#[test]
fn mic_boost_values_match_captures() {
    use MicBoost::*;
    for (boost, expected) in [
        (Db0, "00000000"),
        (Db10, "0a000000"),
        (Db20, "14000000"),
        (Db30, "1e000000"),
    ] {
        assert_eq!(hex4(boost.value_bytes()), expected, "{boost:?}");
    }
}

// ── Lighting ──────────────────────────────────────────────────────────────────

#[test]
fn lighting_ring_on_off() {
    let on = lighting_volume_ring(true);
    assert_eq!(on.len(), 2);
    assert_eq!(head(&on[0]), "5a3903000e00");
    assert_eq!(head(&on[1]), "5a3901010000");
    assert_eq!(head(&lighting_volume_ring(false)[0]), "5a3903000e01");
}

#[test]
fn lighting_disable_frame() {
    let frames = lighting_disable();
    assert_eq!(frames.len(), 1);
    assert_eq!(head(&frames[0]), "5a3a02060000");
}

#[test]
fn lighting_rgb_matches_spec_rows() {
    // doc/usb-spec.md "Enable - Set RGB (255:0:0)": three frames, sent three times.
    let frames = lighting_enable_set_rgb(255, 0, 0);
    assert_eq!(frames.len(), 9);
    for burst in frames.chunks(3) {
        assert_eq!(head(&burst[0]), "5a3a02060100");
        assert_eq!(head(&burst[1]), "5a3a06040003");
        assert_eq!(value(&burst[1]), "01000100");
        assert_eq!(head(&burst[2]), "5a3a090a0003");
        assert_eq!(value(&burst[2]), "0101ff00");
        assert_eq!(
            &burst[2].additional[..2],
            &[0x00, 0xff],
            "additional = [green, red]"
        );
    }
    // (255:255:255): value ends with blue, additional starts with green, red.
    let white = lighting_enable_set_rgb(255, 255, 255);
    assert_eq!(value(&white[2]), "0101ffff");
    assert_eq!(&white[2].additional[..2], &[0xff, 0xff]);
}

// ── Decoder ───────────────────────────────────────────────────────────────────

#[test]
fn decoder_modes_match_captures() {
    for (mode, expected) in [
        (DecoderMode::Normal, "00000040"),
        (DecoderMode::Full, "0000803f"),
        (DecoderMode::Night, "00004040"),
    ] {
        let frames = decoder_mode(mode);
        assert_eq!(frames.len(), 2);
        assert_eq!(head(&frames[0]), "5a1207019702", "{mode:?}");
        assert_eq!(value(&frames[0]), expected, "{mode:?}");
        assert_eq!(head(&frames[1]), "5a1103019702", "{mode:?} commit");
        assert_eq!(value(&frames[1]), "00000000", "{mode:?} commit");
    }
}

// ── Playback ──────────────────────────────────────────────────────────────────

#[test]
fn output_headphones_matches_golden_capture() {
    assert_eq!(
        hex_lines(&toggle_to_headphones()),
        golden(include_str!(
            "../doc/payloads/toggle-output-to-headphones.hex"
        ))
    );
}

#[test]
fn output_speakers_matches_golden_capture() {
    assert_eq!(
        hex_lines(&toggle_to_speakers()),
        golden(include_str!(
            "../doc/payloads/toggle-output-to-speakers.hex"
        ))
    );
}

#[test]
fn direct_mode_frames() {
    let on = enable_direct_mode(true);
    assert_eq!(on.len(), 2);
    assert_eq!(head(&on[0]), "5a3903000501");
    assert_eq!(head(&on[1]), "5a3901010000");
    assert_eq!(head(&enable_direct_mode(false)[0]), "5a3903000500");
}

#[test]
fn spdif_direct_mode_frames() {
    assert_eq!(head(&enable_spdif_out_direct_mode(true)[0]), "5a3903000d01");
    assert_eq!(
        head(&enable_spdif_out_direct_mode(false)[0]),
        "5a3903000d00"
    );
}

#[test]
fn playback_filters_use_intermediate_field() {
    for (filter, expected) in [
        (PlaybackFilter::FastMinimum, "5a6c03000100"),
        (PlaybackFilter::SlowMinimum, "5a6c03000200"),
        (PlaybackFilter::FastLinear, "5a6c03000400"),
        (PlaybackFilter::SlowLinear, "5a6c03000500"),
    ] {
        let frames = playback_filter(filter);
        assert_eq!(frames.len(), 2);
        assert_eq!(head(&frames[0]), expected, "{filter:?}");
        assert_eq!(head(&frames[1]), "5a6c01010000", "{filter:?} commit");
    }
}

// ── Mic ───────────────────────────────────────────────────────────────────────

#[test]
fn mic_boost_frames() {
    let frames = mic_boost(MicBoost::Db30);
    assert_eq!(frames.len(), 2);
    assert_eq!(head(&frames[0]), "5a3c04000002");
    assert_eq!(value(&frames[0]), "1e000000");
    assert_eq!(head(&frames[1]), "5a3c02010000");
}

#[test]
fn recording_toggles_use_recording_intermediate() {
    for (name, frames, feature) in [
        ("noise reduction", voice_clarity_noise_reduction(true), "04"),
        ("aec", voice_clarity_aec(true), "00"),
        ("smart volume", voice_clarity_smart_volume(true), "2c"),
        ("mic eq", voice_clarity_mic_eq(true), "13"),
    ] {
        assert_eq!(frames.len(), 2, "{name}");
        assert_eq!(head(&frames[0]), format!("5a12070195{feature}"), "{name}");
        assert_eq!(value(&frames[0]), "0000803f", "{name} on");
        assert_eq!(
            head(&frames[1]),
            format!("5a11030195{feature}"),
            "{name} commit"
        );
        assert_eq!(value(&frames[1]), "00000000", "{name} commit");
    }
    assert_eq!(value(&voice_clarity_noise_reduction(false)[0]), "00000000");
}

#[test]
fn noise_reduction_level_frames() {
    let frames = voice_clarity_noise_reduction_level(NoiseReductionLevel::Level60);
    assert_eq!(head(&frames[0]), "5a1207019505");
    assert_eq!(value(&frames[0]), "9a99993e");
    assert_eq!(head(&frames[1]), "5a1103019505");
}

#[test]
fn mic_eq_presets_match_captures() {
    // Band values for features 0x14..=0x1B, from doc/payloads/raw/g6-recording-mic-eq-preset-*.txt.
    use MicEqPreset::*;
    #[rustfmt::skip]
    let expected: [(MicEqPreset, [&str; 8]); 11] = [
        (Preset1,   ["000040c0", "000080c0", "00000000", "00000040", "00004040", "000040c0", "00008040", "0000a040"]),
        (Preset2,   ["000040c0", "000080c0", "00000000", "00000040", "00008040", "000000c0", "00000040", "00008040"]),
        (Preset3,   ["000000c0", "000040c0", "00004040", "00008040", "00008040", "000080c0", "00004040", "00000040"]),
        (Preset4,   ["000040c0", "0000a0c0", "00000000", "00008040", "00000000", "000040c0", "00000000", "00000000"]),
        (Preset5,   ["000000c0", "000040c0", "00000040", "00008040", "00008040", "00000000", "000040c0", "00000040"]),
        (Preset6,   ["0000a0c0", "000080c0", "000000c0", "00000000", "00004040", "00008040", "0000c040", "0000e040"]),
        (Preset7,   ["00000000", "00004040", "000000c0", "000080c0", "000080c0", "000000c0", "0000a040", "0000e040"]),
        (Preset8,   ["00000000", "00000000", "00000040", "00000040", "00004040", "000080c0", "00000040", "00008040"]),
        (Preset9,   ["00000000", "00000000", "00000040", "00000040", "000000c0", "00000000", "000080c0", "00008040"]),
        (Preset10,  ["00000000", "00000040", "000000c0", "00000000", "00004040", "0000a040", "0000c040", "0000a040"]),
        (PresetDm1, ["00000000", "00000041", "00000000", "00004041", "00004041", "00008040", "00000041", "00002041"]),
    ];
    for (preset, bands) in expected {
        let frames = voice_clarity_mic_eq_preset(preset);
        assert_eq!(frames.len(), 16, "{preset:?}");
        for (i, band) in bands.iter().enumerate() {
            let feature = 0x14 + i as u8;
            let data = &frames[2 * i];
            let commit = &frames[2 * i + 1];
            assert_eq!(
                head(data),
                format!("5a12070195{feature:02x}"),
                "{preset:?} band {i}"
            );
            assert_eq!(value(data), *band, "{preset:?} band {i}");
            assert_eq!(
                head(commit),
                format!("5a11030195{feature:02x}"),
                "{preset:?} band {i} commit"
            );
            assert_eq!(value(commit), "00000000", "{preset:?} band {i} commit");
        }
    }
}

// ── SBX ───────────────────────────────────────────────────────────────────────

#[test]
fn sbx_toggle_frames() {
    let on = sbx_toggle(AudioFeature::SurroundToggle, true);
    assert_eq!(on.len(), 2);
    assert_eq!(head(&on[0]), "5a1207019600");
    assert_eq!(value(&on[0]), "0000803f");
    assert_eq!(head(&on[1]), "5a1103019600");
    assert_eq!(value(&on[1]), "00000000");
    assert_eq!(
        value(&sbx_toggle(AudioFeature::SurroundToggle, false)[0]),
        "00000000"
    );
}

#[test]
fn sbx_slider_frames() {
    let frames = sbx_slider(AudioFeature::BassSlider, 80);
    assert_eq!(head(&frames[0]), "5a1207019619");
    assert_eq!(value(&frames[0]), "cdcc4c3f");
}

#[test]
fn sbx_smart_volume_special_frames() {
    let night = sbx_smart_volume_special(SmartVolumeSpecial::Night);
    assert_eq!(head(&night[0]), "5a1207019606");
    assert_eq!(value(&night[0]), "00000040");
    assert_eq!(
        value(&sbx_smart_volume_special(SmartVolumeSpecial::Loud)[0]),
        "0000803f"
    );
}

#[test]
fn audio_feature_bytes() {
    use AudioFeature::*;
    for (feature, byte) in [
        (SurroundToggle, 0x00),
        (SurroundSlider, 0x01),
        (DialogPlusToggle, 0x02),
        (DialogPlusSlider, 0x03),
        (SmartVolumeToggle, 0x04),
        (SmartVolumeSlider, 0x05),
        (SmartVolumeSpecial, 0x06),
        (CrystalizerToggle, 0x07),
        (CrystalizerSlider, 0x08),
        (BassToggle, 0x18),
        (BassSlider, 0x19),
    ] {
        assert_eq!(feature.byte(), byte, "{feature:?}");
    }
}

// ── Frame length invariant ────────────────────────────────────────────────────

#[test]
fn all_frames_are_64_bytes() {
    let all: Vec<HidFrame> = [
        lighting_volume_ring(true),
        lighting_disable(),
        lighting_enable_set_rgb(255, 128, 0),
        decoder_mode(DecoderMode::Normal),
        enable_direct_mode(true),
        enable_spdif_out_direct_mode(false),
        playback_filter(PlaybackFilter::FastMinimum),
        toggle_to_speakers(),
        toggle_to_headphones(),
        mic_boost(MicBoost::Db0),
        voice_clarity_noise_reduction(true),
        voice_clarity_noise_reduction_level(NoiseReductionLevel::Level100),
        sbx_toggle(AudioFeature::SurroundToggle, true),
        sbx_slider(AudioFeature::BassSlider, 50),
        sbx_smart_volume_special(SmartVolumeSpecial::Loud),
        voice_clarity_mic_eq_preset(MicEqPreset::Preset1),
    ]
    .into_iter()
    .flatten()
    .collect();

    for (i, frame) in all.iter().enumerate() {
        assert_eq!(frame.to_bytes().len(), HidFrame::LEN, "frame {i}");
        assert_eq!(frame.to_hex().len(), 2 * HidFrame::LEN, "frame {i}");
        assert_eq!(frame.to_bytes()[0], 0x5a, "frame {i} prefix");
    }
}
