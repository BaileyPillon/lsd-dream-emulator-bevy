# Getting started

## Requirements

- Git and a Rust installation managed by rustup.
- The pinned Rust **1.97.1** toolchain; `rust-toolchain.toml` selects it.
- Bevy **0.20.0**, pinned in the workspace manifest and Cargo lockfile.
- A compatible graphics environment for desktop previews.

The default workspace builds and tests without game files or an audio device.
Audio and desktop features need the platform dependencies in
[Bevy's setup guide](https://bevy.org/learn/quick-start/getting-started/setup/).
On Debian/Ubuntu, audio builds require `libasound2-dev`. The X11 desktop backend
also needs the `libxkbcommon-x11-0` runtime library:

```sh
sudo apt install libasound2-dev libxkbcommon-x11-0
```

Windows builds require the Rust MSVC toolchain's C/C++ linker and Windows SDK.
Public CI exercises headless tests on Linux and Windows, plus the Windows
desktop compile check. macOS desktop operation has not been validated here.

## Build and verify

```sh
git clone https://github.com/BaileyPillon/lsd-dream-emulator-bevy.git
cd lsd-dream-emulator-bevy
cargo test --workspace --locked
cargo run -p lsd-bevy --locked -- --steps 120 --seed 1
```

The final command runs a deterministic compatibility probe and prints JSON.
It is a development check, not the game launcher.

## Prepare local reference files

Supply your own extracted Japanese reference files. The extraction root must
contain `SLPS_015.56` and the original `CDI` directory hierarchy. Keep the files
outside Git, or under the ignored `assets/imported/` directory.

Validate the executable:

```sh
cargo run -p lsd-assets --bin lsd-asset-check --locked -- "/path/to/extracted/files/SLPS_015.56"
```

The supported executable is 505,856 bytes, SHA-256
`282308ade239ce61b5603651dbbab0476a681b5d3618e82c0d3c3a95316d302f`.
Other editions have not been accepted. No download or extraction is performed
by these commands. See [asset formats and corpus checks](ASSET_PIPELINE.md).

## Run the dream prototype

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- "/path/to/extracted/files" 0 0 A --dream
```

This development profile starts the original first-day state with a supplied
new-game bootstrap and seed. It connects imported content and live sound
effects; complete game flows and parity remain unfinished. See [status](STATUS.md).

With the game window focused, **W/S** moves forward/back and **A/D** turns.
These are the dream profile's player controls; the inspection camera has a
separate input path. On WSLg, software rendering can run slowly, and audio
requires the WSLg PulseAudio service to be available.

For a stage inspection without the dream profile:

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- "/path/to/extracted/files" 0 0 A
```

The three positional values select stage, map, and texture variant. Append
`--walk` for the chunk movement prototype or `--walk-links` for the streamed
link prototype. See [scene integration](SCENE_LINKS.md) and [day progression](DAYS.md).

## Inspect animation

```sh
cargo run -p lsd-bevy --bin lsd-model --features desktop --locked -- "/path/to/extracted/files" 0 0 0 0 A
```

Use **Space** to pause and **N** to step. This inspector exercises recovered
animation rules; it does not accept an entire NPC encounter.

## Render a controlled music diagnostic

```sh
cargo run -p lsd-assets --bin lsd-music --locked -- "/path/to/SLPS_015.56" "/path/to/AMBIENT.VH" "/path/to/BGA.SEQ" 12 "/path/to/new-music.wav"
```

This supplies one music bank and fixed SDK cadence for a local WAV render.
It does not enable music in the dream prototype. Add a new JSON output path as
the final argument to retain a register trace. See [music recovery](SEQUENCE_VOICES.md).

## Validate a contribution

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p lsd-bevy --features audio --locked
cargo check -p lsd-bevy --features desktop --locked
```

The [contributor guide](../CONTRIBUTING.md) explains reference-only tests,
comparison evidence, and keeping original assets private.
The [developer reference](DEVELOPMENT.md) lists every executable, its required
features and inputs, and commands for targeted tests and local API documentation.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Executable identity is rejected | Confirm the supported edition and hash; do not modify the importer to silently accept another input |
| A game file cannot be found | Point to the extraction root containing both `SLPS_015.56` and `CDI`; quote paths with spaces |
| Linux audio dependency is missing | Install the platform audio development dependencies before enabling `audio` or `desktop` |
| Startup reports a missing `libxkbcommon-x11.so` | Install `libxkbcommon-x11-0` in the Linux environment running the desktop target |
| Graphics initialization fails | Check GPU/driver/display support; the headless probe and tests can still run |
| Reference-dependent tests are ignored | They require separately supplied local assets and native traces; this is expected in public CI |

For reproducible failures, use the [bug report form](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/new?template=bug_report.yml).
