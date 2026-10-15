# Developer reference

Use [Getting started](GETTING_STARTED.md) for installation and the dream
prototype. This guide maps development tasks to the workspace's tools and
checks. All commands run from the repository root with the pinned toolchain.

## Workspace and features

| Package | Responsibility | Default build |
| --- | --- | --- |
| [`lsd-core`](../crates/lsd-core/Cargo.toml) | Deterministic simulation and recovered original behavior | Engine independent; no game files required |
| [`lsd-assets`](../crates/lsd-assets/Cargo.toml) | Content readers, validation, and scene binding | Headless; import commands need local reference files |
| [`lsd-bevy`](../crates/lsd-bevy/Cargo.toml) | Engine integration, inspectors, and host audio | Headless compatibility probe |

`lsd-bevy` has no default Cargo features. `audio` enables host audio and asset
integration. `desktop` includes `audio`, rendering, windowing, X11, and PNG
support. Enabling a feature compiles that integration; the selected binary and
profile determine what runs. Platform dependencies are in the
[setup guide](GETTING_STARTED.md#requirements).

## Select a tool

Pass arguments after Cargo's `--` separator. Commands that use original content
read files supplied locally; they do not fetch game data. Source links below
show each tool's accepted arguments and validation.

| Binary | Package / required feature | Purpose and inputs |
| --- | --- | --- |
| [`lsd-bevy`](../crates/lsd-bevy/src/main.rs) | `lsd-bevy` / default | Deterministic JSON probe; accepts `--steps` and `--seed`; no original content |
| [`lsd-stage`](../crates/lsd-bevy/src/bin/lsd-stage.rs) | `lsd-bevy` / `desktop` | Stage inspection and dream prototype; extracted-files root, stage, map, texture variant, and profile |
| [`lsd-model`](../crates/lsd-bevy/src/bin/lsd-model.rs) | `lsd-bevy` / `desktop` | Model and animation inspection; extracted-files root, stage, map, model, clip, and texture variant |
| [`lsd-preview`](../crates/lsd-bevy/src/bin/lsd-preview.rs) | `lsd-bevy` / `desktop` | Procedural grid for development; no original content |
| [`lsd-box-check`](../crates/lsd-bevy/src/bin/lsd-box-check.rs) | `lsd-bevy` / `desktop` | Controlled screen-box rendering diagnostic; new PNG output path |
| [`lsd-audio-live`](../crates/lsd-bevy/src/bin/lsd-audio-live.rs) | `lsd-bevy` / `audio` | Host-output diagnostic; extracted-files root, packed note, and duration |
| [`lsd-asset-check`](../crates/lsd-assets/src/bin/lsd-asset-check.rs) | `lsd-assets` / default | Validate the canonical executable's header and identity |
| [`lsd-asset-inventory`](../crates/lsd-assets/src/bin/lsd-asset-inventory.rs) | `lsd-assets` / default | Inspect the local map, texture, model, and animation corpus |
| [`lsd-audio-check`](../crates/lsd-assets/src/bin/lsd-audio-check.rs) | `lsd-assets` / default | Inspect local sound banks and sequence content |
| [`lsd-sequence`](../crates/lsd-assets/src/bin/lsd-sequence.rs) | `lsd-assets` / default | Inspect sequence events over a supplied callback count and loop limit |
| [`lsd-sound`](../crates/lsd-assets/src/bin/lsd-sound.rs) | `lsd-assets` / default | Render a controlled sound to a new WAV, with an optional register trace |
| [`lsd-music`](../crates/lsd-assets/src/bin/lsd-music.rs) | `lsd-assets` / default | Render a controlled sequence to a new WAV, with an optional register trace |

The default `cargo run -p lsd-bevy` command runs the probe. Select `--bin lsd-stage`
for the dream prototype. The procedural grid and controlled audio
renders have their own purposes; see [project status](STATUS.md) for the
current gameplay scope.

### Work without original files

```sh
cargo run -p lsd-bevy --locked -- --steps 120 --seed 1
cargo test -p lsd-core --locked
cargo test -p lsd-assets --locked
```

For a procedural graphics check with no game files:

```sh
cargo run -p lsd-bevy --bin lsd-preview --features desktop --locked
```

The grid uses WASD to move and the arrow keys to turn. The dream profile uses
its own [player controls](GETTING_STARTED.md#run-the-dream-prototype).

### Work with local content

```sh
cargo run -p lsd-assets --bin lsd-asset-check --locked -- "/path/to/extracted/files/SLPS_015.56"
cargo run -p lsd-assets --bin lsd-asset-inventory --locked -- "/path/to/extracted/files"
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- "/path/to/extracted/files" 0 0 A --dream
```

Keep captures and rendered WAVs in a private output directory. Choose new output
paths for diagnostics that preserve existing files. The recovery notes explain
capture-specific inputs, supplied state, and limitations:
[scene links](SCENE_LINKS.md), [box rendering](BOX_RENDERING.md),
[live audio](LIVE_AUDIO.md), and [music voices](SEQUENCE_VOICES.md).

## Iterate on a subsystem

Run the relevant package or integration target while developing:

```sh
cargo test -p lsd-core --test native_day_clock --locked
cargo test -p lsd-assets --test formats --locked
cargo test -p lsd-bevy --test scene_audio --features audio --locked
```

Test targets named `native_*` contain bounded comparison fixtures; the name
alone does not imply original-code execution during that test. Some suites also
contain ignored tests requiring local assets or traces. Inspect their source
and required inputs before running an individual ignored case. Ordinary CI
does not supply those original inputs.

For a behavior change, use the [compatibility report template](templates/COMPATIBILITY_REPORT.md)
to record the reference, starting state, inputs, comparison, and gaps. For a
reported mismatch, use the [compatibility issue form](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/new?template=compatibility.yml).
Architecture and ownership are described in [ARCHITECTURE.md](ARCHITECTURE.md).

## Check a pull request

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
cargo test --workspace --locked
cargo test -p lsd-bevy --features audio --locked
cargo check -p lsd-bevy --features desktop --locked
python tools/check_docs.py
```

The [required CI matrix](../.github/workflows/rust.yml) runs the shared checks
on Windows and Ubuntu, and compiles the desktop target on Windows. Local
original-input comparisons and exercised desktop scenarios supplement that
matrix for changes that need them. Documentation-only changes need the
documentation check; they do not need new Rust tests.

Each crate's API overview explains its role, entry points, and examples.
Generate local API documentation when exploring public types:

```sh
cargo doc --workspace --no-deps --locked
```

Open `target/doc/lsd_core/index.html`, `target/doc/lsd_assets/index.html`, or
`target/doc/lsd_bevy/index.html` in a browser. A custom `CARGO_TARGET_DIR` moves
these generated files under that directory. This command documents default
features; it is not a desktop build or a reference comparison.

To include the optional scene, audio, and screen-box APIs, install the
[desktop dependencies](GETTING_STARTED.md#requirements), then run:

```sh
cargo doc -p lsd-bevy --no-deps --features desktop --locked
```

CI builds the default-feature API documentation on Windows and Linux with
`RUSTDOCFLAGS` set to `-D warnings`. Broken intra-doc links and other Rustdoc
warnings fail the build. `cargo test --workspace --locked` also checks the
documentation examples: runnable examples execute, while `no_run` examples
compile without accessing local game files.
