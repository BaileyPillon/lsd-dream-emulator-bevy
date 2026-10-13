# Original animation playback

`lsd-core::AnimationPlayer` now executes decoded original animation records.
The Bevy model inspector uses this integer state for motion, primitive binding
and part hierarchy. Original NPC scripts and the game's complete scene system
still need implementation.

```sh
cargo run -p lsd-bevy --bin lsd-model --features desktop --locked -- /path/to/files 0 0 0 0 A
cargo run -p lsd-bevy --bin lsd-model --features desktop --locked -- /path/to/files 0 0 0 0 A --inspect
cargo run -p lsd-bevy --bin lsd-model --features desktop --locked -- /path/to/files 0 0 0 0 A --capture /path/to/new.png 15
```

Arguments after the extraction root select stage, map, embedded model, animation
and texture variant. Space pauses the inspector, N steps one original actor
animation tick, and WASD moves its camera. Captures apply the explicit tick
count before rendering and preserve existing output files. Game-derived images
remain private.

## Recovered rules

- Child lookup keeps declaration order and uses the first matching type.
  Unknown targets skip the record. Matching targets invalidate the coordinate
  cache even for declaration/end-marker records.
- Attribute commands apply `(old & mask) | bits`.
- Angles use signed division by 360. Relative angles wrap to a halfword before
  signed remainder by 4096; absolute angles store the quotient directly.
- Relative scale uses the low 32-bit product followed by signed division by
  4096. Absolute scales sign-extend packed halfwords. Positions wrap as 32-bit
  values, and position commands update the coordinate's separate translation.
- Primitive selection is one-based. Zero leaves the current binding intact;
  subsequent selections are ignored after a primitive is attached.
- Parent commands retain the first parent. Initial parenting clears coordinate
  translation while retaining the motion state's position. Child lists prepend
  the new child, matching the successful native list-insertion path.
- Selecting a clip applies frame zero immediately and leaves its cursor at
  frame zero. The first enabled tick applies that frame again. Clips with one
  frame hold their initialized state. Pausing preserves the frame cursor.
- The actor timer wraps at 32 bits. An enabled action callback runs after its
  increment and before animation; the actor coordinate cache clears afterward.

The source anchors are original `func_80066340`, `func_800662BC`,
`func_80066214`, `func_80065FD8`, `func_8001D0EC` and their native dependencies.

## Native comparisons

Fresh original PS1 execution generated observations for all 122 animation
entries in the 111 local MOM instances: 4,260 frames and 26,797 records.
Rust matches 10,228 observed states through selections, disabled/playing ticks,
pause/resume, loop wraps and timer-wrap boundaries. Each comparison checks
every supplied part's attributes, coordinate cache, signed angles, scale,
motion/coordinate positions, primitive, parent and child ordering, plus actor
timer, cursor index and enable state.

The native runs execute the original interpreter, offset/record wrappers,
type lookup, allocator, parent lists and SDK TMD mapping/linking. No dependency
output stubs run. The same comparisons pass with clips from the Rust MOM
decoder and hash-identified Japanese-labelled inputs. Raw original inputs and
observations stay private.

Public CI uses nine synthetic sequences with original-generated numerical
observations, plus focused tests of malformed input and action-callback order.
The two original-input tests are explicitly ignored in ordinary CI. For a local
audit, set `LSD_GAME_FILES` and `LSD_NATIVE_ANIMATION_CASES` to absolute paths;
the latter must name the associated private observations, then run:

```sh
cargo test -p lsd-assets --test native_animation_runtime --locked -- --ignored --nocapture
```

See [animation-runtime-checks.json](animation-runtime-checks.json) for function,
fixture and input-evidence hashes, counts and scope.

## Remaining work

The comparisons use persistent, supplied actor/child/model graphs with original
action callbacks disabled. Natural object construction, allocation failures,
resource destruction, broader animation-switch histories and complete NPC
encounters remain verification requirements. The inspector's rest state and
60 ticks/second playback are development settings; original constructor and
game cadence need separate evidence.

Bevy uses a float transform/material adapter for inspection. Original fixed
rotation matrices, GTE effects, attribute-dependent drawing, transparency,
palette shading, projection and draw ordering remain rendering work. Rendered
model captures establish integration, not original visual parity. No complete
original feature group is accepted by these checks.
