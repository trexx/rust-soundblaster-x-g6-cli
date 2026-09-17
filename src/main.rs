use anyhow::Result;
use clap::Parser;

use g6_cli::api::Api;
use g6_cli::cli::{
    AutostartAction, Cli, Command, LightingAction, MicAction, OutputAction, PlaybackAction,
    SbxAction,
};
use g6_cli::device::{DryRun, FrameSink, G6};
use g6_cli::{autostart, state, watch};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let state_path =
        (!cli.no_persist).then(|| cli.state.clone().unwrap_or_else(state::default_path));
    let mut g6state = state_path.as_deref().map(state::load).unwrap_or_default();

    // Only open the device when a frame will actually be sent to it.
    let sink: Box<dyn FrameSink> = if cli.dry_run || !cli.command.sends_frames() {
        Box::new(DryRun)
    } else {
        Box::new(G6::open()?.with_debug(cli.debug))
    };
    let mut api = Api::new(sink.as_ref(), &mut g6state, state_path.as_deref());

    match cli.command {
        Command::Output { action } => match action {
            OutputAction::Toggle => api.output_toggle()?,
            OutputAction::Set { mode } => api.output_set(mode)?,
        },

        Command::Decoder { mode } => api.set_decoder_mode(mode)?,

        Command::Lighting { action } => match action {
            LightingAction::Off => api.lighting_off()?,
            LightingAction::Rgb { red, green, blue } => api.lighting_rgb(red, green, blue)?,
            LightingAction::Ring { enable } => api.lighting_ring(enable.into())?,
        },

        Command::Playback { action } => match action {
            PlaybackAction::Direct { enable } => api.playback_direct_mode(enable.into())?,
            PlaybackAction::SpdifDirect { enable } => {
                api.playback_spdif_direct_mode(enable.into())?
            }
            PlaybackAction::Filter { filter } => api.playback_filter(filter)?,
        },

        Command::Mic { action } => match action {
            MicAction::Boost { db } => api.mic_boost(db)?,
            MicAction::NoiseReduction { enable, level } => {
                api.mic_noise_reduction(enable.into())?;
                if let Some(level) = level {
                    api.mic_noise_reduction_level(level)?;
                }
            }
            MicAction::Aec { enable } => api.mic_aec(enable.into())?,
            MicAction::SmartVolume { enable } => api.mic_smart_volume(enable.into())?,
            MicAction::Eq { enable, preset } => {
                api.mic_eq(enable.into())?;
                if let Some(preset) = preset {
                    api.mic_eq_preset(preset)?;
                }
            }
        },

        Command::Sbx { action } => match action {
            SbxAction::Switch { profile } => api.sbx_switch(profile)?,
            SbxAction::Current => println!("{}", api.sbx_current()),
            effect => {
                let (effect, args, special) = effect
                    .effect()
                    .expect("every other SBX action is an effect");
                api.sbx_effect(
                    args.profile,
                    effect,
                    args.enable.into(),
                    args.value,
                    special,
                )?;
            }
        },

        Command::Apply => {
            let applied = api.apply_saved()?;
            if applied.is_empty() {
                println!("Nothing saved to apply yet");
            } else {
                println!("Applied: {}", applied.join(", "));
            }
        }

        Command::Watch(args) => {
            let mut log = watch::open_log(args.log.as_deref())?;
            watch::run(
                &args,
                state_path.as_deref(),
                cli.dry_run,
                cli.debug,
                log.as_mut(),
            )?;
        }

        Command::Autostart { action } => match action {
            AutostartAction::Enable => autostart::enable(cli.state.as_deref())?,
            AutostartAction::Disable => autostart::disable()?,
            AutostartAction::Status => autostart::status()?,
        },
    }

    Ok(())
}
