# Original floor-following actor movement

Horse now executes its original sound, animation and forward/floor action in
the encounter scene. The shared wrapper also provides the floor-following
operation required by several remaining NPCs. There are 58 implemented action
callbacks among 107 distinct original callbacks; 49 remain pending. This count
does not accept complete NPC encounters or whole-game parity.

## Movement and floor protocol

The original forward and sideways wrappers clear the shared contact slot,
perform signed-halfword local-axis movement, and run the optional synchronous
movement notification. If that notification assigns a contact, the wrapper
skips its floor search. Otherwise it queries the actor's scene owner.

A failed owner lookup returns without a floor signal. A located query tries
the original cell/tile chain and ready-layer order. On a hit it stores the
identified floor tile and applies the original correction; a located miss
keeps the contact empty. The resulting negative-one or negative-two virtual
calls stay local: the inherited implementation does not send these negative
codes through the parent gameplay journal.

The Rust contact slot now distinguishes a peer from a floor contact. It retains
the original single-slot behavior when movement clears it or a body callback
replaces it. The session supplies the live scene geometry on each actor clock
callback, so floor queries use the current slot and readiness state.

Horse requests its note at the original animation-duration period. Its normal
forward amount is minus 384; behavior tick 110 uses minus 11520. Both pass through
the original floor wrapper without a movement notification. Context, behavior
and animation clocks keep their separate update order.

## Evidence

| Comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Original entity forward/sideways wrapper in owned floor graphs | 2,304 | 4,608 |
| Full Horse action with owned model/properties/math | 48 | 1,216 |
| Full Horse action with imported original model/properties/math | 48 | 1,216 |
| Horse encounter adapter with supplied live floor scene | 1 | 130 |

The wrapper cases cover spatial and layered lookup, mode-two selection,
permuted slots, readiness gates, chained tiles, slopes/headings, positive and
negative floor segments, no owner, no location, no hit, pre-existing contacts,
both axes, optional notifications and signed movement truncation. They compare
position, rotation, scale, cache invalidation, movement amount, contact identity,
local signals and parent journal events.

The original runs contain 234 floor hits, 614 located misses and 1,456 cases
without a local floor-result call. The hit/miss calls emit no negative parent
journal event. Movement notifications retain their separate parent events.

Horse trajectories cover ticks 109/110/111, negative ticks and signed overflow,
floor corrections, leaving bounds and the same eight floor graphs. The imported
Horse starts dormant under its original properties. These action comparisons
explicitly invoke the original behavior-start method; they do not establish
natural activation. The encounter-adapter test compares the full 130-frame
owned trajectory. Local virtual-call instrumentation is compared by the direct
wrapper/action replay, rather than exposed by the scene adapter.

The static graph supplies consistent cached world positions and owned
trigonometric/square-root inputs. The original allocator, entity/model/transform
construction, geometry intersection, owner lookup and correction routines run
without replacing game-function outputs. The semantic GTE bridge does not
accept hardware flags or timing. Original model/property observations stay
private and are identified by hashes.

Earlier failed harness inputs remain private: initial static world-cache values
did not match the declared tile positions, and the Rust replay initially lacked
the square-root lookup used by native segment-length checks. The corrected
recordings supply consistent cache and math inputs to both implementations.

```sh
cargo test -p lsd-assets --test native_actor_floor --locked
LSD_GAME_FILES=/path/to/local/files \
LSD_NATIVE_HORSE_FLOOR=/path/to/private/original-horse-floor-cases.json \
  cargo test -p lsd-assets --test native_actor_floor --locked -- --ignored
```

See [floor-following checks](actor-floor-checks.json) for hashes, repeated native
recordings and wider validation. The combined original player-contact and
floor-bypass path now has [coupled comparisons](CONTACT_FLOOR.md), alongside
Soldier and Giant Turtle actions. Natural activation, complete collision response,
cache/residency/lifetime timing and GPU/SPU output remain unaccepted. All 40
provisional whole-feature groups still require acceptance. C decompilation
stays paused and preserved; original behavior makes no AI model calls.
