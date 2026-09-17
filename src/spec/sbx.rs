use super::{
    slider_percent_bytes, AudioFeature, HidFrame, SmartVolumeSpecial, MODE_COMMIT, MODE_DATA,
};

pub fn sbx_toggle(feature: AudioFeature, enable: bool) -> Vec<HidFrame> {
    let value = if enable {
        slider_percent_bytes(100)
    } else {
        [0u8; 4]
    };
    vec![
        HidFrame::playback(MODE_DATA, feature.byte(), value),
        HidFrame::playback(MODE_COMMIT, feature.byte(), [0; 4]),
    ]
}

/// # Panics
/// If `value > 100`.
pub fn sbx_slider(feature: AudioFeature, value: u8) -> Vec<HidFrame> {
    vec![
        HidFrame::playback(MODE_DATA, feature.byte(), slider_percent_bytes(value)),
        HidFrame::playback(MODE_COMMIT, feature.byte(), [0; 4]),
    ]
}

pub fn sbx_smart_volume_special(special: SmartVolumeSpecial) -> Vec<HidFrame> {
    let af = AudioFeature::SmartVolumeSpecial.byte();
    vec![
        HidFrame::playback(MODE_DATA, af, special.value_bytes()),
        HidFrame::playback(MODE_COMMIT, af, [0; 4]),
    ]
}
