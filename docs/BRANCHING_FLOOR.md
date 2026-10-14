# Branching movement actions

Giant Maiko, Fox, Standing Penguin and Sliding Penguin now run in the shared
encounter scene. With the subsequent [color-dependent actions](COLOR_NPCS.md)
and [movement actions](MOTION_NPCS.md), the current implementation count is 71
of 107 distinct original action callbacks, leaving 36 pending. The count does not
measure complete encounters or whole-game parity.

## Recovered behavior

Giant Maiko retains its route choice at tick 1560, state-specific turns and
player-action window from ticks 3421 through 3540. The action branch resets
the behavior tick and enters pursuit. Pursuit uses integer facing, half scale,
the distance gate for a video request, faster movement and floor correction.
Its alternate route omits the extra local-Y movement after a floor contact.

Fox preserves its initial random route, quarter turns, upward translation at
tick 64, later randomized movement notifications, fast spinning route and
random visibility. The same RNG stream supplies each call in original order.

Standing Penguin retains its random initial scale and vertical movement,
frame-independent sound period and turns at ticks 90, 160 and 220. Its forward
movement emits the original synchronous notification. Sliding Penguin retains
its turn at tick 84, nine-degree turns from 188 through 199, and shutdown at
200. Shutdown stops existing sound and behavior before the final note request
and forward/floor operation. The sound context finishes its current update
even after its active flag is cleared.

## Comparisons

| Original CPU comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Owned model/properties/math in eight floor graphs | 384 | 3,204 |
| Privately imported original model/properties/math | 384 | 3,204 |
| Production encounter adapter with supplied target and floor service | 11 | 1,048 |

The recordings compare local pose, cache invalidation, contact identity,
movement amount, context and behavior counters, animation and part state,
RNG state, local virtual calls, parent notification order, sound requests and
player-action/facing service counts. Scenarios cover route branches, action
window endpoints, distance thresholds, turn and shutdown boundaries, negative
counters and signed overflow. The encounter adapter additionally checks the
registered production action mapping and resulting scene actor state; local
virtual-call and service-count instrumentation belongs to direct replay.

The original allocator, model/entity/transform construction, action, animation,
sound-context and floor geometry routines execute without replacing original
game-function outputs. Static cached target/world state, floor graph readiness,
initial state/tick/seed and the original behavior-start call are supplied.
Sound services record requests; this does not establish native SPU output.
The semantic GTE bridge does not establish hardware flags or timing.

```sh
cargo test -p lsd-assets --test native_branch_floor --locked
```

Private original-model comparisons additionally require `LSD_GAME_FILES` and
`LSD_NATIVE_BRANCH_FLOOR`. Original bytes, property rows and detailed original
model observations remain private; public fixtures use owned inputs. See
[branching floor checks](branch-floor-checks.json) for identities, repeated
recordings and wider validation.

Natural activation and full spawn-to-contact paths, reciprocal player response,
complete map/cache/residency lifetimes, original GPU/SPU/media output and full
sessions remain unfinished. Whole NPC encounters and all 40 provisional
whole-feature groups remain unaccepted. C decompilation stays paused. Original
actions make no AI calls; optional AI development tools and mods stay separate.
