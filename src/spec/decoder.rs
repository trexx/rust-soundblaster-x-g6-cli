use serde::{Deserialize, Serialize};

use super::{f32_bytes, HidFrame, MODE_COMMIT, MODE_DATA};

const DECODER_AUDIO_FEATURE: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum DecoderMode {
    Normal,
    Full,
    Night,
}

impl DecoderMode {
    fn value_bytes(self) -> [u8; 4] {
        f32_bytes(match self {
            Self::Normal => 2.0,
            Self::Full => 1.0,
            Self::Night => 3.0,
        })
    }
}

pub fn decoder_mode(mode: DecoderMode) -> Vec<HidFrame> {
    vec![
        HidFrame::decoder(MODE_DATA, DECODER_AUDIO_FEATURE, mode.value_bytes()),
        HidFrame::decoder(MODE_COMMIT, DECODER_AUDIO_FEATURE, [0; 4]),
    ]
}
