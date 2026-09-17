// Argument parsing tests (no device needed).

use clap::{CommandFactory, Parser};
use g6_cli::cli::{Cli, Command, EffectArgs, MicAction, OnOff, SbxAction};
use g6_cli::model::sbx::ProfileName;
use g6_cli::spec::recording::{MicBoost, NoiseReductionLevel};
use g6_cli::spec::{SbxEffect, SmartVolumeSpecial};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("g6-cli").chain(args.iter().copied()))
}

#[test]
fn cli_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn sbx_effect_subcommands_share_arguments() {
    let cli = parse(&["sbx", "surround", "gaming", "on", "--value", "70"]).unwrap();
    let Command::Sbx { action } = cli.command else {
        panic!("expected sbx")
    };
    let (effect, args, special) = action.effect().unwrap();
    assert_eq!(effect, SbxEffect::Surround);
    assert_eq!(
        *args,
        EffectArgs {
            profile: ProfileName::Gaming,
            enable: OnOff::On,
            value: Some(70)
        }
    );
    assert_eq!(special, None);
}

#[test]
fn sbx_smart_volume_special_parses() {
    let cli = parse(&["sbx", "smart-volume", "cinema", "on", "--special", "loud"]).unwrap();
    let Command::Sbx { action } = cli.command else {
        panic!("expected sbx")
    };
    let (effect, args, special) = action.effect().unwrap();
    assert_eq!(effect, SbxEffect::SmartVolume);
    assert_eq!(args.profile, ProfileName::Cinema);
    assert_eq!(args.value, None);
    assert_eq!(special, Some(SmartVolumeSpecial::Loud));
}

#[test]
fn sbx_special_conflicts_with_value() {
    assert!(parse(&[
        "sbx",
        "smart-volume",
        "gaming",
        "on",
        "--value",
        "10",
        "--special",
        "night"
    ])
    .is_err());
}

#[test]
fn sbx_value_must_be_at_most_100() {
    assert!(parse(&["sbx", "bass", "gaming", "on", "--value", "101"]).is_err());
    assert!(parse(&["sbx", "bass", "gaming", "on", "--value", "100"]).is_ok());
}

#[test]
fn sbx_switch_and_current_are_not_effects() {
    let cli = parse(&["sbx", "current"]).unwrap();
    assert!(!cli.command.sends_frames());
    let Command::Sbx { action } = cli.command else {
        panic!("expected sbx")
    };
    assert!(action.effect().is_none());

    let cli = parse(&["sbx", "switch", "music"]).unwrap();
    assert!(cli.command.sends_frames());
    let Command::Sbx {
        action: SbxAction::Switch { profile },
    } = cli.command
    else {
        panic!("expected switch")
    };
    assert_eq!(profile, ProfileName::Music);
}

#[test]
fn mic_boost_accepts_only_supported_steps() {
    let cli = parse(&["mic", "boost", "30"]).unwrap();
    let Command::Mic {
        action: MicAction::Boost { db },
    } = cli.command
    else {
        panic!("expected boost")
    };
    assert_eq!(db, MicBoost::Db30);
    assert!(parse(&["mic", "boost", "15"]).is_err());
    assert!(parse(&["mic", "boost", "40"]).is_err());
}

#[test]
fn noise_reduction_level_accepts_only_supported_steps() {
    let cli = parse(&["mic", "noise-reduction", "on", "--level", "60"]).unwrap();
    let Command::Mic {
        action: MicAction::NoiseReduction { level, .. },
    } = cli.command
    else {
        panic!("expected noise-reduction")
    };
    assert_eq!(level, Some(NoiseReductionLevel::Level60));
    assert!(parse(&["mic", "noise-reduction", "on", "--level", "50"]).is_err());
}

#[test]
fn lighting_rgb_components_are_bytes() {
    assert!(parse(&["lighting", "rgb", "255", "128", "0"]).is_ok());
    assert!(parse(&["lighting", "rgb", "256", "0", "0"]).is_err());
}

#[test]
fn global_flags_work_after_the_subcommand() {
    let cli = parse(&["output", "toggle", "--dry-run", "--no-persist", "--debug"]).unwrap();
    assert!(cli.dry_run && cli.no_persist && cli.debug);
}

#[test]
fn state_path_flag() {
    let cli = parse(&["--state", "custom/state.json", "sbx", "current"]).unwrap();
    assert_eq!(
        cli.state.as_deref(),
        Some(std::path::Path::new("custom/state.json"))
    );
}

#[test]
fn mic_eq_preset_names_accept_documented_and_legacy_spellings() {
    use g6_cli::spec::recording::MicEqPreset;
    for (arg, expected) in [
        ("preset-1", MicEqPreset::Preset1),
        ("preset1", MicEqPreset::Preset1),
        ("preset-10", MicEqPreset::Preset10),
        ("preset10", MicEqPreset::Preset10),
        ("preset-dm-1", MicEqPreset::PresetDm1),
        ("preset-dm1", MicEqPreset::PresetDm1),
    ] {
        let cli = parse(&["mic", "eq", "on", "--preset", arg]).unwrap();
        let Command::Mic {
            action: MicAction::Eq { preset, .. },
        } = cli.command
        else {
            panic!("expected eq")
        };
        assert_eq!(preset, Some(expected), "{arg}");
    }
    assert!(parse(&["mic", "eq", "on", "--preset", "preset-11"]).is_err());
}

// ── apply / watch / autostart ─────────────────────────────────────────────────

#[test]
fn apply_watch_and_autostart_parse() {
    use g6_cli::cli::AutostartAction;
    use g6_cli::watch::WatchArgs;

    let cli = parse(&["apply"]).unwrap();
    assert!(matches!(cli.command, Command::Apply));
    assert!(cli.command.sends_frames());

    let cli = parse(&["watch"]).unwrap();
    let Command::Watch(args) = &cli.command else {
        panic!("expected watch")
    };
    assert_eq!(
        *args,
        WatchArgs {
            interval: 2,
            settle: 1500,
            log: None
        }
    );
    assert!(!cli.command.sends_frames(), "watch opens the device itself");

    let cli = parse(&[
        "watch",
        "--interval",
        "5",
        "--settle",
        "0",
        "--log",
        "w.log",
    ])
    .unwrap();
    let Command::Watch(args) = &cli.command else {
        panic!("expected watch")
    };
    assert_eq!(
        *args,
        WatchArgs {
            interval: 5,
            settle: 0,
            log: Some("w.log".into())
        }
    );
    assert!(
        parse(&["watch", "--interval", "0"]).is_err(),
        "interval must be at least 1 s"
    );

    let cli = parse(&["autostart", "enable"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Autostart {
            action: AutostartAction::Enable
        }
    ));
    assert!(!cli.command.sends_frames());
    assert!(parse(&["autostart", "status"]).is_ok());
    assert!(parse(&["autostart", "disable"]).is_ok());
}

#[test]
fn autostart_command_line_and_reg_output() {
    use g6_cli::autostart::{parse_reg_value, run_command_line, WATCHER_EXE};
    use std::path::Path;

    let dir = Path::new(r"C:\Program Files\g6");
    let exe = dir.join(WATCHER_EXE).display().to_string();
    assert_eq!(run_command_line(dir, None), format!("\"{exe}\""));
    assert_eq!(
        run_command_line(dir, Some(Path::new(r"D:\my state\state.json"))),
        format!("\"{exe}\" --state \"D:\\my state\\state.json\"")
    );

    let output = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\r\n    g6-cli    REG_SZ    \"C:\\g6\\g6-watch.exe\"\r\n\r\n";
    assert_eq!(
        parse_reg_value(output).as_deref(),
        Some("\"C:\\g6\\g6-watch.exe\"")
    );
    assert_eq!(parse_reg_value("ERROR: not found"), None);
}
