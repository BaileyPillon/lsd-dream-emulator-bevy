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
Original day preparation, mood scoring, dream-clock expiry and flashback storage
now also run in Rust. The `--dream` profile starts day one at its original spawn
and commits its end-of-day result; see [day recovery](docs/DAYS.md).
Original scene style, texture/music choices, generation-trigger proximity and
all 14 sound callbacks now run in Rust. The `--dream` profile consumes the
selected textures and queues sound requests; see [generation recovery](docs/GENERATION.md)
for the constructor, full RNG history, rendering and audio work still required.
Generated model/sprite construction and updates now also run in the day profile,
including their original RNG calls and imported object presentation. See
[generated objects](docs/GENERATION_OBJECTS.md) for validation and remaining
renderer, effects, lifetime and scheduling work.
Dream gradient/environment layer state and palette rotation are now connected
to the day profile. A combined generation-frame comparison checks object RNG,
layer updates, palette commands and sound-request order together. See
[generation layers](docs/GENERATION_LAYERS.md) for the evidence and remaining
original GPU and audio comparisons.
Screen boxes now use the original SDK packet fields and draw order. Background
composition and foreground blending use five-bit integer color arithmetic;
see [box rendering](docs/BOX_RENDERING.md) for native packet checks, shader pixel
comparisons and the remaining full-renderer requirements.
The day profile now binds chunk encounters to imported multipart actors, with
original signed gates, placement, model selection and cousin-spawn order.
Stream release, scene rebuild and dream expiry retire their render instances.
See [encounter recovery](docs/ENCOUNTERS.md) for original CPU comparisons and
the remaining named NPC scripts, interaction, physics and scheduling work.
The shared entity lifecycle, perception query math and interaction policy now
also run in Rust. Futon, Television and Drumstick scripts are connected. Nine
additional callbacks now retain their original movement, scaling, visibility,
frame-gated sound or empty action. Dog, Gargoyle and Demon King are also
connected. Sound, motion, event and floor work brings the total to 41 of
107 distinct callbacks implemented, with 66 pending. See
[entity behavior](docs/ENTITY_BEHAVIOR.md) for scoped original CPU comparisons,
actor sound queues and the remaining body dispatch. Soldier and Giant Turtle
now use the shared floor service. See [contact and floor behavior](docs/CONTACT_FLOOR.md)
for their original-code comparisons and the player-contact floor bypass.
Giant Maiko, Fox, Standing Penguin and Sliding Penguin also run in the shared
scene; see [branching movement actions](docs/BRANCHING_FLOOR.md).
Astronaut, Rainbow and Boat Bird now query the live mood chart through the
shared scene services. See [color-dependent NPCs](docs/COLOR_NPCS.md).
Actor notifications now apply original mood/score, recall, direct dynamic-link
and video/end-clock policy during the emitting actor's update. See
[actor journal recovery](docs/ACTOR_JOURNAL.md) for native comparisons and
the remaining inherited body, aim, cinematic playback and full-session work.
See [actor motion and callbacks](docs/ACTOR_MOTION.md) for the new operations,
canonical model comparisons and a day-eight Yokai capture using the original
proximity policy.
Entity body-message gates and contact state now have native comparisons, and
bound journal acknowledgements deliver their link/video/end consequences
synchronously. See [entity body routing](docs/ENTITY_BODY.md) for the inherited
contact correction and remaining query directions/movement response.
Imported actors now retain their original copied root collision binding, and
model cuboid preparation, movement extension and cached-world matrix application
have native comparisons. See [actor collision geometry](docs/ACTOR_GEOMETRY.md)
for the import path. Moving actors now deliver prepared clouds to the imported
player mesh and apply contact/link/video/end consequences in the same call.
The day profile also corrects its player body to `DREAME5.TMD`; `DREAMER.TMD`
belongs to scene/generated helpers. See [actor/player queries](docs/ACTOR_PLAYER_QUERIES.md)
for comparisons and the remaining movement response and native cache timing.
Movement notifications now preserve cached world position/matrix through local
pose changes, and keep the original signed collision margin. See
[cached movement queries](docs/CACHED_MOVEMENT.md) for full movement-wrapper
comparisons and the remaining natural frame/hierarchy work.
Actors now follow the original clock subscription order, including stop/restart
reordering. Waiting signals retain interaction checks while holding action and
animation timers; finished signals retire registered actors. See
[actor clock recovery](docs/ACTOR_CLOCK.md) for native comparisons and the
remaining global player/map/generation/renderer schedule.
Clock dispatch now uses a live cursor, so removing the next subscriber skips it
and a new subscription waits for the next notification. Actor finish cleanup
removes each actor during dispatch. See [live clock traversal](docs/FRAME_CURSOR.md)
for original-code comparisons and the remaining shared scene scheduling.
Player clock callbacks now run before fresh pad dispatch, preserving the original
input phase in the linked scene. An unmodified original-emulator run also records
the shared clock graph. See [frame and pad order](docs/FRAME_ORDER.md) for the
comparisons and the remaining work to connect every scene phase.
The loaded scene now uses one live clock for actors, effect, map generation,
streaming, render snapshot, session counter and player work. Actors born during
streaming wait for the next notification; pad dispatch and session handoff follow
the clock. See [shared scene frame](docs/SHARED_FRAME.md) for its checks and the
remaining original CD, renderer/cache and lifetime work.
The day profile now imports the original Gray Man model and creates one dormant
special instance per scene. Original day gates activate that instance with
player-relative placement, integer facing and stop/restart clock ordering.
See [special encounters](docs/SPECIAL_ENCOUNTERS.md) for original CPU comparisons.
Its original forward action, triggered color flash and flashback-count reset
now run through the shared clock. See [Gray Man behavior](docs/GRAY_MAN.md) for
the original CPU/packet checks and the remaining natural encounter comparisons.
Bear, Rabbit and Peacock now run their original action scripts. Negative trigger
distances also apply the original player-facing operation during interaction,
including waiting signals. See [animal actions and facing](docs/PURSUIT.md) for
state, RNG, sound and pose comparisons.
Dog now reads the player action state, and Gargoyle and Demon King clear the
original view/movement callbacks. Their direct dream-link consequences preserve
RNG, destination and sound order. See [player-interacting NPCs](docs/PLAYER_NPCS.md)
for comparisons and remaining natural encounter work.
Ferris Wheel now preserves its ride/video/end branches and original player
movement. Bell Striker, Elephant, Minotaur, Giant Minotaur, Boatman, Tea Doll,
Buddha, Ox Cart, Opera Singer, Marching Band and Gears also run their recovered
actions. See [sound, motion and event actions](docs/ACTION_PHASES.md) for original
CPU and scene-adapter comparisons. Movie playback remains unfinished.
Horse now uses the original forward/floor wrapper, including owner lookup,
ready-layer and tile-chain order, correction and local hit/miss signals. See
[floor-following actors](docs/ACTOR_FLOOR.md) for native and encounter-adapter
comparisons, the shared contact slot and the remaining collision work.

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

Append `--dream` to start the original first-day state with a supplied new-game
bootstrap and seed 1. `--dream-capture /path/to/new.png 4000` runs supplied idle
host ticks through timeout and the next-day result. Original menus, named NPC
scripts, special-day media, camera/raster accuracy, GPU fades and audio remain unfinished.

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
| Native day, mood, clock and flashback scenarios with generated content | 841 |
| Startup scenarios with original calendar/mood/spawn content | 1,460 |
| Calendar days covered by the original startup campaign | 365 |
| Original generation trigger records loaded locally | 56 |
| Generation scenarios compared with owned-content original CPU execution | 2,082 |
| Original-content generation style starts compared | 5,110 |
| Original-content music-then-texture choices compared | 10,220 |
| Native generated-object constructor cases compared with owned content | 108 |
| Native generation-build cases compared with owned content | 432 |
| Private original-content generation builds compared | 168 |
| Supplied original permanent-link destinations rebuilt locally | 17 |
| Static object instances with a semantic-GTE geometry probe | 13,037 |
| Owned signed encounter-gate comparisons | 18,432 |
| Owned chunk lookup comparisons | 168 |
| Coupled owned chunk/model/entity spawn comparisons | 161 |
| Entities constructed by the coupled native spawn checks | 106 |
| Original model actor constructors compared | 100 |
| Constructor-following original animation steps compared | 600 |
| Original chunk encounter records checked against local model libraries | 98 |
| Available declared actor-model bindings in those records | 159 |
| Loaded encounter contexts checked for bind/retire/rebuild behavior | 97 |
| Callbacks at the initial entity-behavior checkpoint | 3 / 107 |
| Additional callbacks compared using owned model/action graphs | 9 |
| Total original callbacks implemented and compared in supplied graphs | 32 / 107 |
| Local-axis motion cases compared with original CPU execution | 288 |
| Rotation/scale packet cases compared with original CPU execution | 144 |
| Invalid packet cases preserving transform state before a native fault | 12 |
| Additional callback state observations with owned models | 2,214 |
| Additional callback types compared with canonical models/properties | 8 |
| Their canonical model/property state observations | 378 |
| Entity lifecycle state observations compared with original CPU execution | 544 |
| Supplied interaction-policy scenarios compared | 288 |
| Original target-query projection/height/boundary cases compared | 720 |
| Futon/Television observations with their actual model/property bindings | 40 |
| Owned actor-journal scenarios compared with original CPU execution | 336 |
| Owned entity body-notification scenarios compared | 2,016 |
| Coupled journal/actor-contact scenarios compared in a supplied parent graph | 336 |
| Owned prepared-cloud comparisons | 504 |
| Canonical actor collision-binding/bounds observations | 900 |
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
