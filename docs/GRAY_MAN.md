# Gray Man action and retained effect

Entity 98 now executes the original Gray Man callback in the default day profile.
It moves thirty original units along its forward axis and emits movement
notification six. When triggered, it creates or reuses its owned effect,
sets color step ten, starts the decreasing color cycle with variant seven and
mode zero, hides/stops its own action and resets the player's flashback count.
The callback still completes its movement/notification after stopping itself.

Stopping inside the callback preserves the native sound-context behavior:
note handles stop immediately, the remainder of the context update finishes,
and the context/action counters advance once. Animation remains stopped.
The reset changes only the count; stored recall records and the recall index
survive. Interaction/journal work can update a record before the reset occurs.

The default entity effect is a full-screen rectangle. Its imported dimensions
and relative offset resolve to the original 320-by-240 viewport. Variant seven
uses additive RGB555 blending and starts with white in the canonical palette.
The effect has an actor transform binding and a separate clock subscription.
It continues after the actor hides, waits until the next notification when
created during a callback, then detaches its clock on color-cycle completion.
The owning entity retains the allocation for a later trigger.

Rebinding an already active effect does not restart its cycle. The original
entity helper still sets its step to positive ten before the start method
ignores an active state. The port preserves this less obvious reuse behavior.
Reusing a completed effect starts a fresh cycle. Owner destruction removes
the effect's clock membership and its render packet.

## Comparisons

Ten constructed original graphs cover owned and canonical models/palettes,
normal motion and trigger, waiting signals, active/completed reuse, allocation
failure and owner destruction. The comparison includes 350 state observations,
22 original Gray Man callback calls, 312 real clock advances and 316 original
SDK rectangle-sort calls. Original allocator failure is induced with a supplied
empty free list; game-function return values are not replaced.

The Rust replay checks actor pose and animation parts, all behavior-context
words, note-stop order, flashback count/index/storage, effect color/life/flags,
clock traversal/membership and the exact four rectangle command words.
The native graph supplies player target/view/cache, trigger flags, viewport and
an external sound boundary. It runs original CPU routines through the existing
semantic GTE backend; native GPU/SPU execution and hardware timing are separate.

An opt-in loaded-scene test supplies a nearby actor position and reaches the
original interaction gate without setting the triggered flag. It checks the
real day-service reset, waiting/action separation, delayed effect birth,
retained recall storage/index and completion with the actor still owning the
inactive effect. This supplies placement rather than replaying a natural
original encounter.

```sh
cargo test -p lsd-assets --test native_gray_man --locked
LSD_GAME_FILES=/path/to/local/files cargo test -p lsd-assets \
  --test native_gray_man --test loaded_gray_man --locked -- --ignored --nocapture
```

The development capture supplies a nearby Gray Man and a flashback count of ten:

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- \
  /path/to/extracted/files 0 0 A --gray-man-capture /path/to/new.png 2
```

Two simulation steps show effect birth; fourteen show its decreasing color;
thirty-two show completion. The counter, effect state/life/color and original
action status are printed for the supplied scenario. The render adapter consumes
the same native-compared rectangle packet through the existing integer box
shader. See [Gray Man checks](gray-man-checks.json) for hashes and validation.

## Remaining parity work

This implemented a thirteenth distinct original action callback. With the
subsequent [animal action work](PURSUIT.md), [player-interacting NPC work](PLAYER_NPCS.md)
and [sound, motion and event work](ACTION_PHASES.md), 41 of 107 callbacks are
implemented and 66 remain pending. Natural spawn-to-contact trajectories,
reciprocal collision/movement response, full transform/parent/cache lifetimes,
original GPU/SPU output and timing still require comparison. The effect packet
and Bevy captures do not establish original framebuffer parity.

All 40 provisional whole-feature groups remain unaccepted. Menus, persistence,
media, endings and the other project-objective requirements are still open.
Original NPC behavior uses no AI model. AI analysis tools and optional gameplay
mods remain separate. C decompilation stays paused and preserved.
