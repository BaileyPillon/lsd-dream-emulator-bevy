# Movement actions and complete entity updates

Tennyo, Bird, Rocket, UFO, Shark, Fish and Mirage now run in the shared encounter
scene. There are 71 implemented callbacks among 107 distinct original actions,
with 36 pending. Complete natural encounters and whole-game parity remain
unaccepted.

## Original actions

| Action | Recovered behavior |
| --- | --- |
| Tennyo | Original target facing, initial vertical movement, triggered sideways phase and tick-65 reset, and later movement phases. Context and behavior clocks remain distinct. |
| Bird | Twofold scale, two-degree yaw steps, forward movement and the 15-context-tick sound period. |
| Rocket | Three movement speeds, periodic and three-slot sound phases, and the original one-eighth-degree pitch packet from tick 300. |
| UFO | Forward movement, three vertical phases and the 70-context-tick sound period. |
| Shark | Turn windows, speed change at 800, and shutdown at 851 followed by the final movement notification in that same update. |
| Fish | Original local Y/X/Z movement order, signed remainder periods and additional coordinate translations. Negative counters retain the original remainder behavior. |
| Mirage | Initial twofold/sixfold random scale, persistent facing and movement only inside the original distance gate. |

## Interaction before actions

The original `entity_update` checks activation, behavior gating, proximity,
triggering and facing before the inherited action/animation update. The port's
encounter adapter follows that same order. Both direct action recordings and
complete original entity-update recordings are compared separately.

This distinction changes the longer Tennyo trajectory: it reaches the supplied
player's proximity region and enters its triggered movement loop. An initial
comparison against an action-only recording exposed that omitted interaction
phase. The complete original update records the transition, and the production
adapter matches it. The action-only scenario remains a separate comparison.

## Evidence

| Original CPU comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Direct actions with owned model/properties/math | 85 | 2,921 |
| Direct actions with original imported models/properties/math | 85 | 2,921 |
| Complete entity updates with owned inputs | 85 | 2,921 |
| Complete entity updates with original imported models/properties/math | 85 | 2,921 |

All complete-update scenarios run through the actual production encounter
adapter. Its owned and original-model runs each compare 2,836 resulting states;
the table above also counts each initial state. Original model collision bindings
are prepared through the production importer. Direct action replay compares
local virtual-call and facing-call instrumentation as well as resulting state.

The comparisons include complete movement cycles, turn and shutdown boundaries,
negative ticks, signed overflow, independent context/behavior clock values,
triggered states, both Mirage RNG branches and its distance threshold. They
compare pose, cache state, movement amount, contacts, behavior and animation
clocks, part state, RNG, sound requests and parent notification order.

```sh
cargo test -p lsd-assets --test native_motion_npcs --locked
```

Private original-model comparisons also require `LSD_GAME_FILES`,
`LSD_NATIVE_MOTION_NPCS` and `LSD_NATIVE_MOTION_ENCOUNTERS`. Original models,
property rows and detailed original-model observations remain private. Public
fixtures use owned inputs. See [movement NPC checks](motion-npc-checks.json)
for identities, repeated recordings and wider validation.

The original constructors, perception queries, complete entity updates, actions,
transforms, animation and sound contexts execute without replacing original
game-function outputs. Behavior start, initial clocks/state/seed and static
cached-world/local player coordinates are supplied. Complete-update inputs also
supply an identity cached player matrix and explicit view coordinates.

The player model is owned and meshless. These comparisons do not accept
reciprocal collision with the original player mesh, full natural activation,
MapScene transitions, moving-player/cache/residency lifetimes, native GPU/SPU
output, hardware flags or timing. Separate body-query comparisons remain in
the earlier evidence. Whole NPC encounters and all 40 provisional feature groups
remain unaccepted. C stays paused; original actions make no AI calls, with
optional AI tools and mods kept separate.
