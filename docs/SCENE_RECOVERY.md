# Scene lookup and layered movement

`lsd-core` now resolves world positions through a supplied seven-slot scene
graph. It preserves the original slot-label search, half-open X/Z bounds,
vertical layer boundaries, signed divisions, local coordinate wrapping and
odd-row placement. Slot labels and map indices remain distinct.

Floor queries retain the owner's cell index and visit current, next and previous
layers in that order. A stream's ready flag gates geometry traversal. The first
successful query wins; the port does not replace this order with a nearest-floor
search. Contacts retain their scene slot and map index for later callbacks.

The stage and model inspectors now resolve map filenames from the canonical
executable's path catalogue. This includes later maps stored in `N1`, `N2` and
other subdirectories. Stage dimensions and paths are read from local inputs;
the original tables are not included in the repository.

## Try the layered prototype

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 0 0 A --walk-layers
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 6 0 A --walk-layers
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 0 0 A --walk-layers-capture /path/to/new.png 32
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 3 255 A --inspect
```

The layered mode loads the five Bright Moon Cottage maps or six Moonlight Tower
maps into a supplied scene graph. It shares the control/motion code with the
single-chunk prototype and uses the same keyboard controls. Starting positions,
camera rest points and host timing remain development settings. The loader
currently marks imported maps ready synchronously; it does not implement the
original asynchronous stream lifecycle. Body collision still queries the current
owner's chunk; surrounding-body traversal and original hit notifications remain
open. Spatial scene streaming is the next requirement.

## Evidence

The synthetic reference fixture records 1,108 original CPU location checks,
3,690 query selections and 144 placement checks. These execute the original
lookup routines and their native virtual methods in supplied allocated slot
graphs, with no function-result stubs or GTE interception. The checks include
permuted slot labels, unready slots, signed map indices, layer boundaries,
out-of-scene positions and non-default mode values. Query selection's `mode == 1`
condition remains distinct from placement/ownership's `mode != 0` condition.

Separate integration checks exercise readiness, first-hit layer order,
movement-to-floor correction and inconsistent loaded state. They validate the
Rust integration; they are not a native execution of the complete floor/actor
graph. An original-input check resolves all 418 maps against fourteen executable
descriptors and imports both layered stages:

```sh
LSD_GAME_FILES=/absolute/path/to/files cargo test -p lsd-assets --test native_scene_content --locked -- --ignored
```

Exact counts, hashes and validation scope appear in
[scene-recovery-checks.json](scene-recovery-checks.json). Original stream allocation,
failure paths, actor lifetimes, body ordering, linking, NPC scripts, camera/session
timing, native rendering, sound and whole-game comparisons remain unfinished.
Whole original feature acceptance remains 0/40 provisional groups.
