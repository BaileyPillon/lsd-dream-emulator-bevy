# LSD: Dream Emulator — Bevy Port

[![Rust CI](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/actions/workflows/rust.yml/badge.svg)](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/actions/workflows/rust.yml)

An independent Rust/Bevy port of **LSD: Dream Emulator**, built from original
PlayStation behavior evidence. The goal is faithful gameplay, content, NPC
scripts, rendering and audio, with optional mods and AI development tools kept
separate from the original experience.

**Early development.** The repository currently provides compatibility
primitives, a deterministic Bevy probe, original map/texture decoding and an
original-stage asset viewer. Full gameplay remains on the [roadmap](docs/ROADMAP.md).

## Start here

The workspace pins Rust **1.97.1**, Bevy **0.20.0** and its Cargo lockfile.
Install Rust and the platform dependencies in the
[official Bevy setup guide](https://bevy.org/learn/quick-start/getting-started/setup/).

```sh
cargo test --workspace --locked
cargo run -p lsd-bevy --locked -- --steps 120 --seed 1
```

The default executable runs a reproducible compatibility probe without a window
or game assets. Its steps deliberately exercise the RNG; they are development
steps rather than the original game's clock or random-call history.

For the procedural stage-grid preview:

```sh
cargo run -p lsd-bevy --bin lsd-preview --features desktop --locked
```

Use **WASD** to move and the **left/right arrows** to turn. Green tiles show the
original stage-zero next-stream exclusion bounds. The geometry is generated
for development. Original collision and gameplay remain open.

To inspect an original stage from a local disc extraction:

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A
```

This viewer uses original map geometry, textures and tile placement. See the
[asset pipeline](docs/ASSET_PIPELINE.md) for corpus evidence, captures and
remaining rendering/gameplay requirements.

To validate your own reference executable, without copying or modifying it:

```sh
cargo run -p lsd-assets --bin lsd-asset-check --locked -- /path/to/SLPS_015.56
```

## Progress

| Measure | Current port |
| --- | ---: |
| Rust primitives with original native golden observations | 3 |
| Numeric native reference observations | 2,580 |
| Provisional whole-game feature groups accepted in the port | 0 / 40 |
| Original LBD maps decoded by the asset pipeline | 418 |
| Original NPC behaviors ported and accepted | 0 |
| AI model calls | 0 |

The native checks cover RNG results/state, animation lead-word fields and signed
cell bounds. Slot ordering also has focused Rust tests. Whole gameplay parity
requires separate original/port comparisons. The 40-group taxonomy is
provisional; an exhaustive official feature total is unknown.

The companion [matching C decompilation](https://github.com/BaileyPillon/lsd-dream-emulator-decompilation)
is paused at **194,212 / 199,912 game-function bytes in C (97.15%)**. Its code
coverage measures C recovery and is tracked separately from this port. The C
checkout and its pending source corrections have been preserved.

See [parity.json](docs/parity.json), [architecture](docs/ARCHITECTURE.md),
[reference evidence](docs/REFERENCE.md) and [contributing](CONTRIBUTING.md).

## Project boundaries

- `lsd-core`: deterministic compatibility logic and original NPC scripts as
  they are ported. No engine, network or AI dependencies.
- `lsd-assets`: validation and bounded decoding of locally supplied original assets.
- `lsd-bevy`: runtime integration, presentation and development previews.
- `tools/ai-assist`: the planned, separate analysis/modding workflow.

No discs, executables, extracted game assets or BIOS files are distributed.
Imported assets stay in the ignored `assets/imported/` directory. New Rust code
is MIT licensed; the [notice](NOTICE.md) records reference attribution.
