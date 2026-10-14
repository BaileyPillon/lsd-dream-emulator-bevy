# Actor motion and nine additional callbacks

The port now runs twelve of the original catalog's 107 distinct callbacks in
supplied, CPU-compared graphs. Nine new callbacks use local movement, pose setters,
visibility, frame-gated sound or their original empty action. The remaining 95
are pending. Whole NPC encounters and whole-game feature groups remain unaccepted.

## Operations and callbacks

`ModelActor` retains the signed movement halfword at original node offset `0x48`.
The constructor initializes it to 300; the old `turn_limit` label incorrectly
implied steering behavior. The historical constructor fixture retains that key
as an alias, while new motion comparisons establish its movement semantics.

Local movement truncates the requested amount to signed 16 bits, obtains the
actor's rotation matrix and applies the original long-vector SDK arithmetic.
Position addition wraps as signed 32-bit arithmetic. The original axis methods
use Z for notification 6, X for 7 and Y for 8. Requested notifications follow the
translation immediately. The Bevy adapter delivers them through the journal
interface. Entity contact and acknowledgement policy is now connected separately
in [entity body routing](ENTITY_BODY.md); real movement collision geometry remains
unfinished.

Rotation packets retain original fraction decoding, degree conversion and signed
remainder behavior. Scale packets retain signed halfword conversion and their
absolute/additive rules. Both decode all components before changing the pose.
Twelve invalid-input comparisons observe the original CPU division guard before
any transform/cache write. Rust returns an explicit fault with the same retained
transform state; this does not accept CPU exception signaling or timing.

| New callback | Recovered action |
| --- | --- |
| Yokai | Distance attenuation, ten-context-tick sound cadence, duration-driven clip change, local Z movement and notification 6 |
| Sailboat | Local Z movement |
| Balloon | Local Y movement |
| Giant fish | Absolute scale six |
| Halo | Absolute scale one half |
| Siblings flock | Absolute scale six |
| Bed bone, dead | Shared-RNG visibility toggle |
| Kicking man | Sound command at animation frame 30 |
| Prince/princess | Empty action, preserving context/animation scheduling |

`EntityScriptHost` supplies original rotation math and synchronous notification
delivery. `EncounterScene::tick_with_motion` connects imported tables to the day
session. Legacy callers without motion services receive an explicit fault when
the selected script needs them. Actor binding also invalidates the coordinate
cache after applying the supplied origin, matching the observed entity-start
path. Original animation, behavior and sound-context clocks remain independent.

## Evidence

| Comparison | Count |
| --- | ---: |
| Owned local-axis movement cases | 288 |
| Owned rotation/scale packet cases | 144 |
| Invalid packet fault/state cases | 12 |
| Owned callback scenarios | 54 |
| Their prepared-state/tick observations | 2,214 |
| New callback types with canonical model/property bindings | 8 |
| Canonical model/clip cases | 9 |
| Their bound/start/preparation/tick state observations | 378 |

Owned comparisons use a valid two-clip MOM with two empty TMD objects and sparse
trigonometric inputs generated for the test. They execute real original entity,
model and SDK routines and the original callbacks. Original function outputs are
not replaced. The canonical comparisons load actual declared models and property
rows, use original trigonometric inputs and supply controlled behavior activation.
They also verify the declaration-to-file/model/variant identities in Rust.
Already initialized CD, typed roots, target positions and external sound services
are supplied. Request traces prove no body collision response or SPU playback.

No regular chunk declaration binding was identified for the bed-bone callback;
its current evidence uses owned models. Recover its natural construction/binding
before accepting that encounter. See [motion checks](actor-motion-checks.json)
for fixture/function identities, source hashes and validation limits.

```sh
cargo test -p lsd-assets --test native_actor_motion --locked
LSD_GAME_FILES=/path/to/extracted/files LSD_NATIVE_ACTOR_MOTION=/path/to/private-observations.json cargo test -p lsd-assets --test native_actor_motion --locked -- --ignored --nocapture
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A --npc-motion-capture /path/to/new.png 64
```

The capture uses a supplied day-eight bootstrap, new-game history and seed 1.
Day one excludes Yokai through the original chunk gate. The capture supplies
player placement at Yokai's query; it does not force behavior activation or clip
selection. The connected policy starts the behavior. Over 63 active ticks, the
callback selects clip one and moves Z from -9,216 to -15,360. The rendered actor
uses that updated pose. This is a development scenario, not a naturally played
or accepted full original encounter. Idle, Futon and legacy captures remain
byte-identical to the previous checkpoint, and dream expiry removes actor render
instances. Original assets, rows, detailed canonical observations and captures
remain private; CI uses owned inputs.

## Remaining requirements

Recover the other 95 callbacks, Gray Man behavior, inherited motion/body
geometry, ground reconciliation, aim and full journal ancestry. Establish
natural construction/lifetimes and global scheduling, then compare original GPU,
SPU, camera and whole encounters. Menus, persistence, media and full-game
acceptance remain separate requirements. Optional AI tools and mods stay separate
from original scripts. The matching C reference remains paused and preserved.
