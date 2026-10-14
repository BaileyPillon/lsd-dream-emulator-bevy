# Original scene scale and requesting NPC actions

Old Woman, Starship and Airship now run in the shared encounter scene and can
request the original map-node scale effect. There are 58 implemented callbacks
among 107 distinct original actions, with 49 pending. Whole natural encounters
and the 40 provisional whole-feature groups remain unaccepted.

## Scene ownership and clock

The original receiver is the owning render context. Entity startup stores that
scene as its link target; its virtual slot at 0x138 selects a scale request.
It changes map notify nodes rather than an actor's animation blend.

The controller selects a positive/negative, slow/fast frac_t packet, stores its
signed step count and replaces any previous request without restoring scale.
Each positive-count step adds the converted signed halfwords to the existing
32-bit scales. Completion stores -1 and retains the result. Reset applies the
absolute reset packet only when the count is nonzero. Requesting zero therefore
also disables the reset gate without restoring an earlier scale.

All seven slot arrays contain 410 persistent nodes each. The original traversal
visits every entry, including inactive nodes and the ten additional chain-node
slots. The constructor initializes equal scales and every effect operation
visits all 2,870 nodes in order. Rust represents their common scale once; stream
binding changes geometry and pose without reinitializing that scale. A newly
constructed scene starts at unit scale.

The original render callback sorts the scene before stepping the effect. The
shared clock records the scale used for drawing before advancing the controller.
The Bevy map meshes consume that recorded scale. Actors, generated objects,
slot origins and the player retain their own transforms.

Collision uses the original rotation-only query matrices. The original visual
scale does not enter those matrices, even at zero or negative scale. Enlarging
collision to follow the visible geometry would change original behavior.

## Requesting actions

| Action | Recovered behavior |
| --- | --- |
| Old Woman | First-tick RNG branch, local-height facing gate, distance-triggered fast growth, separate behavior/context clocks, and the original extra tick increment before the later end notification. |
| Starship | Ordered first-tick RNG calls, periodic sound, optional vertical movement, continuing forward movement, and a fast-growth request at behavior tick 300 in the selected state. |
| Airship | RNG gates for slow-growth requests and the ascent branch, half-degree yaw steps, vertical/forward motion, and the direct link request at tick 930. |

Airship's direct link also consumes the original destination-selection RNG and
queues the link sound. Its replay uses the existing production journal and link
services to compare these consequences with the original execution.

## Evidence and limits

| Original CPU comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Direct actions, owned model/properties/math | 82 | 6,492 |
| Direct actions, original models/properties/math | 82 | 6,492 |
| Complete entity updates, owned inputs | 82 | 6,492 |
| Complete entity updates, original inputs | 82 | 6,492 |
| Persistent-node controller, owned packets | 12 | 95 |
| Persistent-node controller, original packets | 12 | 95 |

Complete entity-update recordings include activation/perception before the
action. Their production encounter replays compare 6,410 resulting states per
input set, retaining pose, separate clocks, animation parts, RNG, sound requests,
scale requests and notification order. Perception notifications are recorded at
the entity boundary; original day/recall consequences have separate identified
state comparisons. These supplied scenarios do not accept a whole encounter.

The controller recordings construct 2,870 distinct original notify nodes, execute
the original traversal/setter/reset, compare every node and check setter order
and cache invalidation. They include request replacement, zero and negative
counts, integer overflow, completion and reset gates. Detailed original-model
observations and original asset/table bytes remain local.

A loaded-stage test sends an Old Woman request through the real encounter and
journal services, checks the render snapshot versus the next scale, verifies the
collision result, and rebuilds the scene. Four Bevy captures show unit scale,
an intermediate step, completion and retention. The last two images are byte
identical; the existing idle capture also remains byte identical. These are port
rendering checks, not original GPU, GTE flag, SPU, cadence or audiovisual parity.

```sh
cargo test -p lsd-assets --test native_scene_scale_npcs --test native_scene_scale_runtime --locked
cargo run -p lsd-bevy --features desktop --bin lsd-stage -- <extracted-files-root> 0 0 A --scene-scale-capture <new.png> 5
```

The capture profile supplies a scale request for inspection; it does not claim a
natural NPC encounter. Private comparisons also require `LSD_GAME_FILES`,
`LSD_NATIVE_SCENE_SCALE_NPCS`, `LSD_NATIVE_SCENE_SCALE_ENCOUNTERS` and
`LSD_NATIVE_SCENE_SCALE_RUNTIME`. `loaded_scene_scale` requires original content.

Triangle Head still needs its remaining player-callback services and comparisons.
Natural activation, stream lifetimes, original drawing/precision, reciprocal
collision, audio playback and whole-session timing remain open. Original NPC
behavior makes no AI model calls; optional AI tools and mods remain separate.
