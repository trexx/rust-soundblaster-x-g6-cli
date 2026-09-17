use super::{HidFrame, PlaybackFilter, EMPTY_ADDITIONAL, MODE_COMMIT, MODE_DATA};

/// Playback features reset (DATA + COMMIT pair, value 0) whenever the output is switched.
const OUTPUT_RESET_FEATURES: [u8; 11] = [
    0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14,
];

/// Output switch: a `2c05`/`2c01` header pair, then a DATA+COMMIT pair for every feature in
/// `OUTPUT_RESET_FEATURES` followed by the output-specific `tail` features.
fn output_switch(intermediate: [u8; 2], tail: &[u8]) -> Vec<HidFrame> {
    let mut frames = vec![
        HidFrame::new([0x2c, 0x05], intermediate, 0x00, [0; 4], EMPTY_ADDITIONAL),
        HidFrame::new([0x2c, 0x01], [0x01, 0x00], 0x00, [0; 4], EMPTY_ADDITIONAL),
    ];
    for &feat in OUTPUT_RESET_FEATURES.iter().chain(tail) {
        frames.push(HidFrame::playback(MODE_DATA, feat, [0; 4]));
        frames.push(HidFrame::playback(MODE_COMMIT, feat, [0; 4]));
    }
    frames
}

pub fn toggle_to_speakers() -> Vec<HidFrame> {
    output_switch([0x00, 0x02], &[0x09, 0x09])
}

pub fn toggle_to_headphones() -> Vec<HidFrame> {
    output_switch([0x00, 0x04], &[0x09, 0x06, 0x09])
}

/// `39 03` flag frame followed by its `39 01` commit.
fn flag_39(intermediate: [u8; 2], enable: bool) -> Vec<HidFrame> {
    vec![
        HidFrame::new(
            [0x39, 0x03],
            intermediate,
            u8::from(enable),
            [0; 4],
            EMPTY_ADDITIONAL,
        ),
        HidFrame::new([0x39, 0x01], [0x01, 0x00], 0x00, [0; 4], EMPTY_ADDITIONAL),
    ]
}

pub fn enable_direct_mode(enable: bool) -> Vec<HidFrame> {
    flag_39([0x00, 0x05], enable)
}

pub fn enable_spdif_out_direct_mode(enable: bool) -> Vec<HidFrame> {
    flag_39([0x00, 0x0d], enable)
}

pub fn playback_filter(filter: PlaybackFilter) -> Vec<HidFrame> {
    vec![
        HidFrame::new(
            [0x6c, 0x03],
            filter.intermediate_bytes(),
            0x00,
            [0; 4],
            EMPTY_ADDITIONAL,
        ),
        HidFrame::new([0x6c, 0x01], [0x01, 0x00], 0x00, [0; 4], EMPTY_ADDITIONAL),
    ]
}
