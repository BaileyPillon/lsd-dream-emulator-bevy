# LSD: Dream Emulator — Bevy Port

[![Rust CI](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/actions/workflows/rust.yml/badge.svg)](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/actions/workflows/rust.yml)

An independent Rust/Bevy port of **LSD: Dream Emulator**, built from original
PlayStation behavior evidence. The goal is faithful gameplay, content, NPC
scripts, rendering and audio, with optional mods and AI development tools kept
separate from the original experience.

**Early development.** The repository currently provides compatibility
primitives, a deterministic Bevy probe, original map/texture/animation decoding,
integer animation playback, and original-stage/model asset viewers. Full
gameplay remains on the [roadmap](docs/ROADMAP.md).

An original control/movement and collision prototype now runs in a loaded map;
see [movement recovery](docs/MOVEMENT_COLLISION.md) for its scope and checks.

Original scene lookup and floor query order also run in a supplied slot graph.
The layered prototype can load all five cottage maps or six tower maps; see
[scene recovery](docs/SCENE_RECOVERY.md). A streamed viewer now handles spatial
neighbor selection and slot reuse through the recovered protocol; see
[stream recovery](docs/STREAMING_RECOVERY.md). Original CD timing and actor
callbacks still need implementation.

Surrounding body queries now preserve the original rectangle/chain order and
tile attribute gate; see [body dispatch recovery](docs/BODY_DISPATCH_RECOVERY.md).
The shared geometry path also retains fixed-point parent rotation and matrix
saturation, compared with the original hit and actor notification chain in
supplied graphs; see [body transform recovery](docs/BODY_TRANSFORMS.md).
Original link tables and destination rules now have a local loader and native
CPU comparisons. Transition policy preserves notification cancellation and
callback order; see [link recovery](docs/LINKS.md). Wiring these rules into
natural actors and visible dream transitions remains open.
Dream tunnel, teleport, dynamic-link and stair handlers now also run in Rust,
including all four stair scripts and cached actor keys. Their checks execute
original angle/position setters in supplied graphs; see
[link runtime recovery](docs/LINK_RUNTIME.md).
The streamed inspector can now run the linked player and original map-notification
policy, including per-contact pose restoration. See
[linked scene integration](docs/SCENE_LINKS.md) for its remaining lifecycle work.
Original effect color cycles and map completion callbacks now also run in Rust
and consume link requests in the inspector. See [effect recovery](docs/EFFECTS.md)
for the native comparisons and remaining GPU/session work.
The inspector now replaces resident maps and textures after a completed link,
preserves RNG/timer state and runs the original player-motion reset. See
[session recovery](docs/SESSION.md) for its construction and day-service limits.

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

Append `--walk` for the single-chunk movement prototype. Full streaming,
linking and original scene scheduling remain separate requirements.

Append `--walk-links` for the streamed link prototype, or
`--walk-links-capture /path/to/new.png 120` for a supplied forward-input capture.
The inspector consumes original effect requests and rebuild notifications, then
loads the destination scene. GPU fades, original construction/clock scheduling,
day generation and audio remain open.

For a supplied permanent-wall contact followed by scene replacement, append
`--link-rebuild-capture /path/to/new.png 0 24`. The final numbers select the
stage's wall-trigger index and host tick count. This development scenario
supplies the collision contact and keeps the chosen texture variant across stages.

To inspect original model animation from a stage's streamed library:

```sh
cargo run -p lsd-bevy --bin lsd-model --features desktop --locked -- /path/to/extracted/files 0 0 0 0 A
```

Use **Space** to pause and **N** to step. This inspector runs the recovered
animation rules; original NPC actions, game cadence and PS1 rendering remain
separate requirements. See [animation playback](docs/ANIMATION_RUNTIME.md).

To validate your own reference executable, without copying or modifying it:

```sh
cargo run -p lsd-assets --bin lsd-asset-check --locked -- /path/to/SLPS_015.56
```

## Progress

| Measure | Current port |
| --- | ---: |
| Initial foundation primitives with native golden observations | 3 |
| Initial foundation numeric native observations | 2,580 |
| Provisional whole-game feature groups accepted in the port | 0 / 40 |
| Original LBD maps decoded by the asset pipeline | 418 |
| Streamed model libraries decoded | 98 |
| Animation frames decoded | 4,260 |
| Original animation entries compared in supplied actor graphs | 122 |
| Matching native animation state observations | 10,228 |
| Reference control steps compared in supplied graphs | 960 |
| Supplied body-transform cases compared with the original hit/notification path | 120 |
| Original link triggers loaded locally | 74 |
| Original random spawn entries loaded locally | 239 |
| Effect states compared using generated test colors | 3,600 |
| Effect states compared using locally imported original colors | 536 |
| Original map effect-completion scenarios compared | 84 |
| Original session routing/tick/link-policy scenarios compared | 1,048 |
| Original scene-motion reset scenarios compared | 160 |
| Supplied original permanent-link destinations rebuilt locally | 17 |
| Static object instances with a semantic-GTE geometry probe | 13,037 |
| Original NPC behaviors ported and accepted | 0 |
| AI model calls | 0 |

The native checks cover RNG results/state, animation lead-word fields and signed
cell bounds. Slot ordering also has focused Rust tests. Whole gameplay parity
requires separate original/port comparisons. The 40-group taxonomy is
provisional; an exhaustive official feature total is unknown.

Animation comparisons cover integer motion, primitive/parent attachment,
selection, frame order, pause/resume and loop/timer wrapping. These supplied
graphs have original NPC action callbacks disabled. They do not accept whole
NPC encounters, natural construction/lifetimes or hardware timing.

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
