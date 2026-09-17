// Tests of state file load/save (uses a per-process temp directory).

use std::fs;
use std::path::PathBuf;

use g6_cli::model::sbx::ProfileName;
use g6_cli::model::{G6State, OutputMode};
use g6_cli::spec::SmartVolumeSpecial;
use g6_cli::state::{load, save};

fn temp_path(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("g6-cli-state-test-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("state.json")
}

#[test]
fn save_then_load_round_trips() {
    let path = temp_path("round_trip");
    let mut state = G6State {
        output: Some(OutputMode::Headphones),
        ..Default::default()
    };
    state.sbx.selected = ProfileName::Music;
    state.sbx.music.bass.toggle = true;
    state.sbx.music.bass.slider = 80;
    state.sbx.music.smart_volume.special = Some(SmartVolumeSpecial::Night);

    save(&path, &state).unwrap();
    assert_eq!(load(&path), state);
    assert!(
        !path.with_extension("json.tmp").exists(),
        "temp file is renamed away"
    );
}

#[test]
fn json_layout_is_stable() {
    // Existing state.json files must keep loading: check the exact field names and enum spellings.
    let path = temp_path("layout");
    let mut state = G6State {
        output: Some(OutputMode::Speakers),
        ..Default::default()
    };
    state.sbx.gaming.smart_volume.special = Some(SmartVolumeSpecial::Loud);
    save(&path, &state).unwrap();

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["output"], "Speakers");
    assert_eq!(json["sbx"]["selected"], "Gaming");
    assert_eq!(json["sbx"]["gaming"]["smart_volume"]["special"], "Loud");
    assert_eq!(json["sbx"]["gaming"]["surround"]["slider"], 50);
    assert_eq!(json["sbx"]["gaming"]["surround"]["toggle"], false);
}

#[test]
fn missing_file_gives_defaults() {
    let path = temp_path("missing");
    assert_eq!(load(&path), G6State::default());
    assert!(!path.exists(), "load never creates the file");
}

#[test]
fn partial_file_is_filled_with_defaults() {
    let path = temp_path("partial");
    fs::write(
        &path,
        r#"{"output":"Headphones","sbx":{"selected":"Cinema"}}"#,
    )
    .unwrap();
    let state = load(&path);
    assert_eq!(state.output, Some(OutputMode::Headphones));
    assert_eq!(state.sbx.selected, ProfileName::Cinema);
    assert_eq!(state.sbx.cinema, Default::default());
}

#[test]
fn corrupt_file_is_moved_aside_not_overwritten() {
    let path = temp_path("corrupt");
    fs::write(&path, "this is not json").unwrap();

    assert_eq!(load(&path), G6State::default());

    let backup = PathBuf::from(format!("{}.bak", path.display()));
    assert_eq!(fs::read_to_string(&backup).unwrap(), "this is not json");
    assert!(
        !path.exists(),
        "corrupt file no longer at the original path"
    );

    // A subsequent save writes a fresh file and leaves the backup alone.
    save(&path, &G6State::default()).unwrap();
    assert!(path.exists());
    assert!(backup.exists());
}

// ── settings ──────────────────────────────────────────────────────────────────

#[test]
fn settings_round_trip_with_stable_json_spelling() {
    use g6_cli::model::Lighting;
    use g6_cli::spec::decoder::DecoderMode;
    use g6_cli::spec::recording::{MicBoost, MicEqPreset, NoiseReductionLevel};
    use g6_cli::spec::PlaybackFilter;

    let path = temp_path("settings");
    let mut state = G6State::default();
    state.settings.decoder = Some(DecoderMode::Night);
    state.settings.playback_filter = Some(PlaybackFilter::FastLinear);
    state.settings.lighting = Some(Lighting::Rgb {
        red: 255,
        green: 16,
        blue: 0,
    });
    state.settings.volume_ring = Some(true);
    state.settings.mic_boost = Some(MicBoost::Db10);
    state.settings.noise_reduction_level = Some(NoiseReductionLevel::Level80);
    state.settings.mic_eq_preset = Some(MicEqPreset::PresetDm1);

    save(&path, &state).unwrap();
    assert_eq!(load(&path), state);

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["settings"]["decoder"], "Night");
    assert_eq!(json["settings"]["playback_filter"], "FastLinear");
    assert_eq!(json["settings"]["lighting"]["Rgb"]["red"], 255);
    assert_eq!(json["settings"]["volume_ring"], true);
    assert_eq!(json["settings"]["mic_boost"], "Db10");
    assert_eq!(json["settings"]["noise_reduction_level"], "Level80");
    assert_eq!(json["settings"]["mic_eq_preset"], "PresetDm1");
    assert_eq!(
        json["settings"]["aec"],
        serde_json::Value::Null,
        "unset settings are null"
    );

    let mut state = G6State::default();
    state.settings.lighting = Some(Lighting::Off);
    save(&path, &state).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["settings"]["lighting"], "Off");
}

#[test]
fn state_file_without_settings_section_loads_with_no_settings() {
    let path = temp_path("pre_settings");
    fs::write(&path, r#"{"output":"Speakers","sbx":{"selected":"Music"}}"#).unwrap();
    let state = load(&path);
    assert_eq!(state.output, Some(OutputMode::Speakers));
    assert_eq!(state.sbx.selected, ProfileName::Music);
    assert_eq!(state.settings, Default::default());
}
