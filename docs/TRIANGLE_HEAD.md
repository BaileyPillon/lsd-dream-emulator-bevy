# Triangle Head and player callback restoration

Triangle Head now runs in the shared encounter scene with its original frame
sounds, movement, distance gates, RNG branches, negative scene-scale request and
video/end notifications. There are 59 implemented callbacks among 107 distinct
original actions, with 48 pending. Whole natural encounters remain unaccepted.

## Original action and services

The action checks animation frames modulo 30 for sound, starts facing the player
at behavior tick 101, then moves before checking distance. At tick 300 it clears
movement while retaining the camera callback when the distance gate passes.
At tick 500 it requests the original `set_callbacks(1, 1)` operation.

The restore service selects normal movement first, then normal view updates.
Leaving movement mode 2 also clears its reset-view and active-view flags. These
flags are independent of ordinary camera updates. The service requires a bound
view callback and reports missing player services if none is available.

The close-range branch preserves the first and conditional second RNG calls,
second sound slot, behavior tick reset and scene shrink request. The later
state-10 branch retains its tick-70 RNG draw and distinct video/end notifications.
The action uses the current animation frame, behavior tick and sound context
counter separately, including signed clock wrapping.

## Evidence

| Original CPU comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Direct action with owned model/properties/math | 52 | 1,947 |
| Direct action with original model/properties/math | 52 | 1,947 |
| Complete entity updates with owned inputs | 52 | 1,947 |
| Complete entity updates with original inputs | 52 | 1,947 |
| Callback restore followed by three player clocks | 24 | 72 resulting player states |

The complete entity-update recordings include activation/perception before the
action and replay 1,895 resulting states per input set through the production
encounter adapter. Comparisons retain pose, animation parts, separate clocks,
RNG, sound requests, scale requests, callback requests/state and notifications.
Scenarios include deliberate even, odd/even and odd/odd RNG branches, independent
frame/context counters, distance boundaries, state transitions, callback restore
from movement modes 0–2 and view modes 0–3, and clock wrapping.

The player-clock recordings run the original callback setter and subsequent
timer callbacks, with both input-lock states. They verify normal view and
movement updates after restoration, not just callback flags. External sound
backend observations remain distinct from control-state comparisons and do not
establish SPU playback. Other callback modes still need their own runtime work.

A loaded-stage test imports the original Triangle Head model and properties.
It observes movement stopping while the view continues, then resumes movement
by 24 units in the same shared clock as the restore request. A supplied close
branch also drives the real scene-scale service through its zero-height
completion. These are controlled starting states, not natural encounters.

The shrink inspector profile supplies a negative scale request. Captures check
unit, half-height, completed zero-height and retained zero-height map geometry;
the existing idle profile remains a separate regression check. Original GPU,
GTE flags, SPU, draw order and global cadence remain unaccepted.

```sh
cargo test -p lsd-assets --test native_triangle_head --test restored_player_callbacks --locked
cargo run -p lsd-bevy --features desktop --bin lsd-stage -- <extracted-files-root> 0 0 A --scene-shrink-capture <new.png> 65
```

Original-content tests additionally require `LSD_GAME_FILES`,
`LSD_NATIVE_TRIANGLE_HEAD_NPCS` and `LSD_NATIVE_TRIANGLE_HEAD_ENCOUNTERS`.
`loaded_triangle_head` requires original content. Original model/table bytes,
property rows and detailed original-model observations remain local. Original
NPC behavior uses no AI model calls; optional AI tools and mods remain separate.
