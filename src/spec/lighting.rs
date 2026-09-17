use super::{HidFrame, EMPTY_ADDITIONAL};

pub fn lighting_disable() -> Vec<HidFrame> {
    vec![HidFrame::new(
        [0x3a, 0x02],
        [0x06, 0x00],
        0x00,
        [0; 4],
        EMPTY_ADDITIONAL,
    )]
}

pub fn lighting_enable_set_rgb(r: u8, g: u8, b: u8) -> Vec<HidFrame> {
    let mut frames = Vec::new();
    for _ in 0..3 {
        frames.push(HidFrame::new(
            [0x3a, 0x02],
            [0x06, 0x01],
            0x00,
            [0; 4],
            EMPTY_ADDITIONAL,
        ));
        frames.push(HidFrame::new(
            [0x3a, 0x06],
            [0x04, 0x00],
            0x03,
            [0x01, 0x00, 0x01, 0x00],
            EMPTY_ADDITIONAL,
        ));

        // value: [0x01, 0x01, 0xff, blue]; additional[0..2] = [green, red]
        let value = [0x01, 0x01, 0xff, b];
        let mut additional = [0u8; 54];
        additional[0] = g;
        additional[1] = r;
        frames.push(HidFrame::new(
            [0x3a, 0x09],
            [0x0a, 0x00],
            0x03,
            value,
            additional,
        ));
    }
    frames
}

/// Enable or disable the volume ring LED (bright white LED under the volume knob).
/// audio_feature: 0x00=enabled, 0x01=disabled.
pub fn lighting_volume_ring(enable: bool) -> Vec<HidFrame> {
    let af = if enable { 0x00 } else { 0x01 };
    vec![
        HidFrame::new([0x39, 0x03], [0x00, 0x0e], af, [0; 4], EMPTY_ADDITIONAL),
        HidFrame::new([0x39, 0x01], [0x01, 0x00], 0x00, [0; 4], EMPTY_ADDITIONAL),
    ]
}
