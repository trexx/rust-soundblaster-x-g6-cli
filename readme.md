# SoundBlaster X G6 CLI

A Windows CLI tool for controlling the Creative SoundBlaster X G6 USB DAC.

Communicates via the HID interface (USB interface 4) — no driver replacement required.
The device's audio class driver continues to function normally during use.

---

## Requirements

- Windows 10 or later (x64)
- SoundBlaster X G6 connected via USB

---

## Installation

Download `g6-cli.exe` from the Releases page and place it anywhere on your `PATH`.
Keep `g6-watch.exe` next to it if you want settings applied automatically at logon
(see [Automatic apply on connect](#automatic-apply-on-connect)).
`state.json` is created automatically next to the executable on first run (see
[State file](#state-file) if that directory is not writable).

### Build from source

```
cargo build --release --target x86_64-pc-windows-msvc
```

The binary will be at `target\x86_64-pc-windows-msvc\release\g6-cli.exe`.

The test suite does not need the device and also runs on Linux/macOS (`cargo test`;
Linux needs `libudev-dev` to build `hidapi`). `--dry-run` works without the device too.

---

## Usage

```
g6-cli [OPTIONS] <COMMAND>
```

### Global options

| Flag | Description |
|---|---|
| `--dry-run` | Print the HID frames that would be sent (one hex line each) without opening the device |
| `--debug` | Print raw HID frames and device responses to stderr |
| `--no-persist` | Do not read or write the state file |
| `--state <PATH>` | Use this state file instead of `state.json` next to the executable (env: `G6_CLI_STATE`) |

---

## Commands

### Output

```
g6-cli output toggle
g6-cli output set <speakers|headphones>
```

`toggle` alternates between Speakers and Headphones using the last state saved in `state.json`.

---

### Decoder

```
g6-cli decoder <normal|full|night>
```

---

### Lighting

```
g6-cli lighting off
g6-cli lighting rgb <R> <G> <B>       # R/G/B: 0-255
g6-cli lighting ring <on|off>          # volume-knob LED
```

---

### Playback

```
g6-cli playback direct <on|off>
g6-cli playback spdif-direct <on|off>
g6-cli playback filter <fast-min|slow-min|fast-lin|slow-lin>
```

---

### Mic

```
g6-cli mic boost <0|10|20|30>
g6-cli mic noise-reduction <on|off> [--level <0|20|40|60|80|100>]
g6-cli mic aec <on|off>
g6-cli mic smart-volume <on|off>
g6-cli mic eq <on|off> [--preset <preset-1|...|preset-dm-1>]
```

Mic EQ presets: `preset-1` through `preset-10` and `preset-dm-1`.

---

### SBX Sound Effects

Profiles: `gaming` | `music` | `cinema` | `special`

```
g6-cli sbx switch <profile>      # apply stored settings for this profile
g6-cli sbx current               # print active profile name

g6-cli sbx surround     <profile> <on|off> [--value 0-100]
g6-cli sbx crystalizer  <profile> <on|off> [--value 0-100]
g6-cli sbx bass         <profile> <on|off> [--value 0-100]
g6-cli sbx smart-volume <profile> <on|off> [--value 0-100 | --special <night|loud>]
g6-cli sbx dialog-plus  <profile> <on|off> [--value 0-100]
```

Settings are persisted per-profile in `state.json` so `sbx switch` replays them next time.
For Smart Volume, `--special` and `--value` replace each other: whichever was set last is
what `sbx switch` sends.

---

## Automatic apply on connect

The CLI remembers the last value of every setting it sends (the `settings` section of
`state.json`). Two commands replay that configuration:

```
g6-cli apply                                   # send everything saved to the device now
g6-cli watch [--interval 2] [--settle 1500] [--log <PATH>]
                                               # keep running; apply whenever the G6 appears
```

`watch` checks for the device every `--interval` seconds. When it appears (or is already
present when `watch` starts) it waits `--settle` milliseconds for Windows to finish
enumerating it, then applies the saved settings. Log lines go to stderr, or to `--log`.
Add `--dry-run` to see the frames that would be sent without touching the device.

What is applied, in order: output, decoder, playback direct / SPDIF direct / filter, the
selected SBX profile, lighting and volume ring, mic boost, noise reduction (and level), AEC,
smart volume, EQ (and preset). Settings you have never set are skipped, and the SBX profile
is only replayed once you have changed any SBX setting, so a fresh install never pushes
"all effects off" to the device. The watcher only reads `state.json`; it never writes it, so
the CLI can be used while it runs and the next connect picks up the new values.

### Start at logon

`g6-watch.exe` is `g6-cli watch` without a console window. Keep it next to `g6-cli.exe`, then:

```
g6-cli autostart enable      # registers g6-watch.exe under HKCU\...\Run for the current user
g6-cli autostart status
g6-cli autostart disable
```

No administrator rights are needed. `g6-watch.exe` logs to `g6-watch.log` next to the state
file (`--log <PATH>` to change that) and honours `--state` / `G6_CLI_STATE` like the CLI; the
registration includes `--state` when you enable autostart with a custom state file.

---

## State file

`state.json` records the last output selection (for `output toggle`) and the four SBX
profiles. By default it sits next to `g6-cli.exe`. If that directory is not writable
(for example under `Program Files`), point `--state` or the `G6_CLI_STATE` environment
variable at a writable location such as `%APPDATA%\g6-cli\state.json`.

The file is written atomically. If it ever becomes unreadable, it is moved aside to
`state.json.bak` and defaults are used, so nothing is overwritten silently.

---

## Notes

- Volume, mute, and mixer controls are handled by Windows' own audio stack (right-click the
  taskbar volume icon -> "Open Volume Mixer"). Those features required exclusive access to the
  USB AudioControl interface, which would disable system audio.
- The wire format is documented in `doc/usb-spec.md`, with the Wireshark captures it was
  derived from under `doc/payloads/`.
