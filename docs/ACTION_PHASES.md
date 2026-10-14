# Original NPC sound, movement and event phases

Twelve further original action callbacks now run in Rust: Ferris Wheel, Bell
Striker, Elephant, Minotaur, Giant Minotaur, Boatman, Tea Doll, Buddha, Ox Cart,
Opera Singer, Marching Band and Gears. With [actor floor recovery](ACTOR_FLOOR.md), 48 of
107 distinct original action callbacks are implemented; 59 remain pending.
These counts measure callbacks. Whole encounters and game completion still
require separate acceptance.

## Original event branches

When triggered, Ferris Wheel starts a thirty-tick action window. An action
press during that window clears movement callbacks while retaining the view
callback, resets its behavior tick and enters the ride state. Consecutive ride
updates move the player by the original local-axis amount. Tick one hundred
requests the end of the dream. With no action press, the window's expiry
requests the event video and changes the actor to its terminal action state.

The player movement service uses the current integer player rotation and the
original signed-halfword movement amount. The production encounter dispatcher
updates the live player position and retains that amount. Scene-link reset
continues to restore the normal player callbacks.

Bell Striker increments its state when the sound-context tick returns to zero.
The context wraps at the original animation-duration boundary. In state
thirty-six, each active action consumes the original random check and may
request an event video. The shared journal applies the original cinematic
selection and dream-timer consequences immediately.

These callbacks request media through the journal. Original movie decoding,
playback and complete cinematic transitions remain unfinished.

## Sound, movement and animation

| Callback | Recovered action |
| --- | --- |
| Elephant | Absolute double scale and world-axis descent |
| Minotaur | Half-duration sound period and forward movement |
| Giant Minotaur | Three initial notes and forward movement |
| Boatman | Animation-frame note and forward movement |
| Tea Doll | Ten-context-tick note and forward movement |
| Buddha | Initial random scale change, four-context-tick note and movement |
| Ox Cart | Half-duration sound phases and synchronous movement notification |
| Opera Singer | One-hundred-twenty-context-tick note |
| Marching Band | Initial animation pause, timed resume/note, later pause/stop |
| Gears | Last-frame note, four-context-tick loop and two-hundred-tick note |

All use the original sound-context update after the callback, separate behavior
and animation clocks, signed remainder semantics, wrapping state/tick changes
and notification ordering. New AI behavior does not replace original scripts.

## Evidence

| Comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Owned model/property/math/link inputs | 215 | 8,115 |
| Imported original model/property/math/link inputs | 215 | 11,616 |
| Production Ferris Wheel scene adapter | 2 | 280 |
| Production player movement adapter against original scene-node observations | 144 | 144 |

Action trajectories cover four seeds, full Bell Striker cycles, Ferris Wheel
ride/video/end paths, held and early action inputs, trigger gating, context
boundaries, frame-gated notes, animation pause/resume, negative ticks and
signed overflow. Their snapshots compare actor/part pose, cache, animation,
context, RNG, sound, notifications and player callbacks, pose, movement amount,
spawn/link/timer/penalty/cinematic state after each full actor update.

The scene-adapter comparison uses the production encounter dispatcher and
player movement binding for both complete recorded Ferris Wheel branches. Its
trigger, initial pose and target sequence are supplied inputs. It does not
establish natural encounter activation or original frame/render timing.

Both native runs reproduce identical observations and function-entry counts.
The original graph uses the real allocator, model/entity/player construction,
original full actor updates and the direct DreamSys journal, with a base Scene
parent and external sound services. No original game function outputs are
replaced. Target/cache state and the initial action/trigger/context state are
supplied. The semantic GTE bridge does not prove hardware flags or timing.

```sh
cargo test -p lsd-assets --test native_action_phases --locked
LSD_GAME_FILES=/path/to/local/files \
LSD_NATIVE_ACTION_PHASES=/path/to/private/original-action-phase-cases.json \
  cargo test -p lsd-assets --test native_action_phases --locked -- --ignored
```

Public replay uses owned inputs. Original models, property rows and detailed
original-model observations stay local. See [action-phase checks](action-phase-checks.json)
for identities, hashes, counts and the wider regression results.

Natural NPC encounters, inherited floor/collision response, full transform/cache
lifetime, MapScene fades, GPU/SPU output and the remaining project objective
still require implementation and comparison. All forty provisional whole-feature
groups remain unaccepted. The C project stays paused and preserved. Original
behavior makes no AI model calls; optional AI tools and mods retain their own
scope.
