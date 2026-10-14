# Chunk encounters and multipart actors

The `--dream` profile now loads the original encounter channel from the local
Japanese executable and each ready map's MML/MOM library. The channel creates
multipart model actors with original spawn positions and rotations. Stream
release and scene rebuild retire the previous actors; dream completion clears
the channel and its Bevy render instances.

Twelve original named callbacks now use the shared entity lifecycle and sound
context; other callbacks keep an explicit pending state. Perception/interaction
policy runs before action and animation. See [entity behavior](ENTITY_BEHAVIOR.md)
for scope and remaining body dispatch. Trigger/link/video/end journal consequences
are connected separately in [actor journal recovery](ACTOR_JOURNAL.md).
The two proximity sound slots documented in
[generation recovery](GENERATION.md) have their own state and lifetime.

## Recovered rules

`lsd-core/src/encounters.rs` implements the ordered program in the original
`scene/link.c`: chunk lookup and the special stage-four palette alias, signed
day/month/color/type gates, used-byte updates, teleport enabling, signed model
selection, unsigned variant selection and entity-two cousin recursion. A failed
model lookup still permits the cousin path. A failed actor creation stops the
current object. Chunk processing retains the original shared RNG call on a
failed day gate outside stage zero, including its Gray Man activation request.

`lsd-core/src/model_actor.rs` creates the initial part types, finds the root via
the first frame's primitive selector, binds primitives/parents and selects clip
zero through `AnimationPlayer`. The successful constructor's owner flags,
action kind, initial movement halfword and collision mode are retained. Allocation failure
and complete resource ownership require more original comparisons.

`lsd-assets/src/encounter_tables.rs` and `encounter_scene.rs` import the canonical
executable's chunk/object/variant/rotation/position tables and 130 entity-property
records. These bytes remain local. Vertical stage maps retain the row byte in
their encounter catalogue key; the original placement function separately
collapses that row in the horizontal plane. Positive unlock initialization and
common animation-enable fields are retained. Teleport requests are consumed
once so they cannot leak from a retired scene.

The Bevy adapter rebuilds meshes when primitive bindings change, updates part
parents and local poses, and applies hidden flags to mesh nodes. Hiding a mesh
does not hide its descendant bones. Imported textures share the existing VRAM
and palette refresh path. The adapter still uses floating-point Bevy transforms;
native GTE projection, lighting, textured rasterization and global draw order
remain requirements.

## Evidence

| Comparison | Cases or observations |
| --- | ---: |
| Signed gate/used-byte transitions on owned inputs | 18,432 |
| Chunk lookup and stage-four alternate-record selection | 168 |
| Coupled original chunk/MML/model/entity spawn scenarios on owned inputs | 161 |
| Entities actually constructed in those coupled scenarios | 106 |
| Original MOM inputs with actor constructor state compared | 100 |
| Following animation enable/tick/disable observations | 600 |
| Original chunk records checked against local map libraries | 98 |
| Available declared model bindings in those records | 159 |
| Distinct loaded map contexts in the lifecycle regression | 97 |
| Actor instances constructed in that lifecycle regression | 364 |

The native harness executes the original routines from hash-identified
`SLPS_015.56`, using the real allocator, in-memory MML/MOM construction and SDK
mapping/binding. It provides already-initialized CD state and explicit external
palette/audio service boundaries. Owned coupled scenarios use callback-free
entities and supplied scene/system roots. The supplied audio sinks record
commands and return deterministic service results; they prove no SPU output.
Original function outputs are not replaced.

Constructor snapshots normalize original resource nodes in child lists into
model/primitive bindings before comparing part relationships. That normalization
does not establish parity of the entire scene/resource graph. Original model
inputs and their detailed observations remain private; the public gate/spawn
fixtures contain owned input tables and numeric observations.

The loaded-content regression checks repeated binding without extra RNG use,
slot retirement/reload, preserved used flags, new serials after a scene epoch,
and final cleanup. It is a Rust integration regression, not a comparison with
the original global scheduler. The first-day timeout regression also exercises
the connected encounter channel. Capture checks count live Bevy roots, bones
and meshes and preserve private PNG hashes. See
[encounter checks](encounter-checks.json) for recorded evidence and exact scope.
The owned coupled model input has since had its MOS/TMD offsets corrected and
been regenerated. All 161 spawn observations stayed unchanged. A decoder check
now requires a valid MOM with two frames and two primitives; see
[entity behavior](ENTITY_BEHAVIOR.md) for the correction evidence. The initial
entity part-flag setup has also been corrected against real entity constructors.

```sh
cargo test -p lsd-core --test native_encounter_gates --test native_encounter_program --locked
LSD_GAME_FILES=/path/to/extracted/files cargo test -p lsd-assets --test native_encounter_content --test native_dream_runtime --locked -- --ignored --nocapture
```

The constructor comparison additionally needs private original CPU observations
in `LSD_NATIVE_ACTOR_CONSTRUCTORS`. CI runs the owned fixtures without original
game content. Raw executable bytes, assets, imported tables and screenshots are
excluded from the public repository.

## Remaining requirements

Recover the original named NPC callback catalog, Gray Man construction and
activation, inherited body and aim dispatch, full encounter consequences, actor audio,
full lifetime/allocation failure paths and global timing. Compare native scenes
and original GPU/audio output with the running port. These checks accept zero
whole NPC behaviors and zero whole-game feature groups. Optional AI analysis or
gameplay mods remain separate; this change makes no AI model calls.
