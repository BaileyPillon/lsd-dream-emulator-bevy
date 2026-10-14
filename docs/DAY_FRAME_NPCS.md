# Day, proximity and animation-frame actions

Gargoyle Pit, Mushroom, Human Face Flower, Giant Lips, Whale, Hopscotch Girl
and Corpse now run in the shared encounter scene. There are 71 implemented
callbacks among 107 distinct original actions, with 36 pending. Whole natural
encounters and game parity remain unaccepted.

## Shared services

The actor journal exposes the original day-of-year query to scripts. It returns
the signed day index plus one, with the original integer wrapping. The year
remains separate. Querying does not change either field.

Script proximity queries use the same unlock-height adjustment and range/radius
conversion as the existing interaction policy. Gargoyle Pit then uses the
identified player world matrix and view to choose its movement branch. Owned
scenarios include height boundaries and negative unlock values.

The original-code comparisons use a 72-frame owned animation and separately
imported original models. Initial frame positions are reached by executing the
original animation update with actions disabled, then supplying the declared
initial clocks and pose. Rust seeks through its real animation interpreter in
the same way. This provides defined inputs for frame-gated action comparisons.

## Actions

| Action | Recovered behavior |
| --- | --- |
| Gargoyle Pit | Paired sounds at the clip midpoint, a separate context sound period, proximity-dependent vertical motion, facing/chase phase and later turning movement. |
| Mushroom | Day-dependent scale, correctly ordered conditional RNG calls, periodic sound, and random animation pause/resume while its action continues. |
| Human Face Flower | Day-dependent/random scale, first-context sound and original target facing. |
| Giant Lips | Facing and sound at frame 38, signed vertical movement periods and notifying forward movement. |
| Whale | Three sound slots at frames 15 and 70, followed by shutdown after five animation durations. |
| Hopscotch Girl | First-tick random scale and frame-gated sounds, preserving the original single frame subtraction at 32. |
| Corpse | One local movement when its behavior tick is zero, including the original signed wrap behavior. |

## Evidence

| Original CPU comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Day query across independent years and signed day indices | 27 | 27 |
| Direct actions with owned model/properties/math | 125 | 2,989 |
| Direct actions with original models/properties/math | 120 | 3,194 |
| Complete entity updates with owned inputs | 125 | 2,989 |
| Complete entity updates with original inputs | 120 | 3,194 |

Both complete-update sets replay through the production encounter adapter:
2,864 resulting owned states and 3,074 original-model states. The table also
counts initial states. Owned cases include five additional unlock-height
variants; original property rows remain unchanged. Animation duration is read
from each actual model and determines Whale's complete trajectory length.

Comparisons retain local pose, movement amount, cache/contact state, separate
context/behavior/animation clocks, part state, RNG, sound requests and parent
notification order. Direct replay additionally compares day, proximity and
facing calls. Complete updates include original activation/perception before
the action. The live day service uses the actual actor journal dispatcher;
the shared test runtime also retains the earlier live-color comparisons.

```sh
cargo test -p lsd-assets --test native_day_frame_npcs --locked
```

Private original-content comparisons additionally require `LSD_GAME_FILES`,
`LSD_NATIVE_DAY_FRAME_NPCS` and `LSD_NATIVE_DAY_FRAME_ENCOUNTERS`. Original model
bytes, property rows and detailed original-model observations stay private.
Public fixtures use owned inputs. See [day/frame checks](day-frame-checks.json)
for identities, repeated recordings and wider validation.

Original constructors, day/proximity queries, actions, complete entity updates,
animation and sound contexts execute without replacing game-function outputs.
Behavior start, day/year, seed, animation seek/pause, initial clocks and static
cached-world/local player and view inputs are supplied. The player is owned and
meshless. These checks do not accept reciprocal original-player collision,
natural activation, full scene/cache/residency lifetimes, scene scale effects,
native GPU/SPU output, flags or timing. Whole NPC encounters and all 40
provisional whole-feature groups remain unaccepted. C remains paused; original
actions make no AI calls, with optional AI tools and mods separate.
