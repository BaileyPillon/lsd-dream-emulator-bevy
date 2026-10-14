# Twelve further movement, sound and animation actions

The shared scene now implements Ship, Palace Maiko, Acrobats, Light, Ring,
Hoop Girl, Giant Hoop Girl, Small Elephant, Bridge Maiko, Question Mark Penguin,
Standing Penguin Flock and Giant Triangle Head. The current count is 71
implemented callbacks among 107 distinct original actions, with 36 pending.
Whole natural encounters and the 40 provisional whole-feature groups remain
unaccepted.

| Action | Recovered behavior |
| --- | --- |
| Ship | Signed context remainder, sound start/stop windows, live distance ratio and local forward movement. |
| Palace Maiko | Conditional first-tick RNG, half scale, ordered vertical movement/absolute rotation, long movement and tick-2000 turn. |
| Acrobats | Live distance ratio and signed context sound period. |
| Light | Conditional RNG state, delayed forward movement and ordered vertical move/facing at tick 502. |
| Ring | First-tick RNG branch, signed direction choice, half-turn at 300 and movement cutoff at 600. |
| Hoop Girl | First-context RNG and sound, signed-halfword sideways displacement, delayed facing and continuing forward movement. |
| Giant Hoop Girl | Complete Hoop Girl action followed by absolute scale six. |
| Small Elephant | Repeated half scale and forward movement. |
| Bridge Maiko | Fractional scale, frozen animation with continuing action, distance-triggered tick reset and a rotating state cycle. |
| Question Mark Penguin | The original Standing Penguin action, checked through its own callback, model and property binding. |
| Standing Penguin Flock | Conditional early return and ordered second RNG draw for turn direction. |
| Giant Triangle Head | Absolute scale six through its own original callback and model binding. |

Hoop Girl's sideways amount is converted to a signed halfword before rotation:
51,200 becomes -14,336 and 102,400 becomes -28,672. Bridge Maiko checks its newly
reset behavior tick in the same action. Flock's early return skips its second
RNG call while the common sound/animation update continues. These details are
retained rather than replaced by floating-point movement or simplified timing.

## Evidence and integration

| Original CPU comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Direct actions with owned model/properties/math | 222 | 11,200 |
| Direct actions with original models/properties/math | 222 | 11,200 |
| Complete entity updates with owned inputs | 222 | 11,200 |
| Complete entity updates with original inputs | 222 | 11,200 |

Production encounter replays compare 10,978 resulting states per input set; the
table also counts initial states. They retain local pose, animation parts,
separate clocks, movement amount, RNG, sound/voice requests, notifications and
callback state. Scenarios deliberately cover RNG branches, signed wrapping,
independent sound-context counters, state and frame boundaries, long movement
trajectories, frozen animation and the distinct wrapper callbacks.

Some original properties request a direct link during perception. The replay
retains that link state across calls and uses one ordered test sound backend
for link and actor requests, including voice identities. Perception day/recall
consequences remain separate identified-state comparison scopes. The live port
still queues sound requests; this evidence does not establish SPU playback.

A loaded-stage test imports all twelve original models and unchanged property
rows into controlled starting states. It executes their actions through the
production journal, collision and scene services, checks ordinary and giant
Hoop Girl's halfword movement, and runs the shared clock. Ship moves 30 units
over three frames while the renderer receives its player snapshot. These
supplied placements do not accept natural spawning or complete encounters.

Independent repeat recordings use filtered event observers while retaining
the original instruction/GTE observer and checking code ranges at block
boundaries. All owned observations, private original-model observations and
function-entry counts match the unfiltered recordings byte for byte. This is
an analysis speed improvement, not an original hardware timing claim.

```sh
cargo test -p lsd-assets --test native_additional_actions --locked
```

Original-content comparisons require `LSD_GAME_FILES`,
`LSD_NATIVE_ADDITIONAL_ACTIONS_NPCS` and
`LSD_NATIVE_ADDITIONAL_ACTIONS_ENCOUNTERS`. `loaded_additional_actions` also
requires original content. Original asset bytes, property rows and detailed
original-model observations remain local. Original NPC behavior uses no AI
model calls; optional AI tools and mods remain separate.

Natural encounters, complete body/scene lifetimes, original GPU/GTE flags,
audio playback and global timing still need acceptance. Callback counts do not
measure whole-game parity.
