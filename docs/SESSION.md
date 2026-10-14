# Original session policy and destination scenes

The Rust session policy implements original map-parent routing, link codes,
session ticks, map opening order and end-day result selection. Rebuild link codes
set state 3. A session tick first calls its base scene service, then reads the
current state; state 3 stops and destroys the map, reads the destination, creates
it, attaches it and runs it before setting state 2. A base callback can change the
state before this decision.

Original end-day routes also retain their different mode/result behavior and
cinematic sign check in the core policy. Their day/cinematic services are supplied
boundaries in native comparisons. The inspector keeps those requests pending
until the real day services are implemented; it does not fabricate a day result.
Original startup, generation, mood scoring, menus and persistence remain open.

The inspector's loaded-scene consumer now connects effect completion to this
rebuild policy. It loads the destination maps, places the actor using the decoded
grid/spawn rules, applies a target rotation when paused, resets player motion,
and starts the decreasing effect cycle. The normal motion reset preserves the
dream timer and RNG. It retains the original signed angle conversion, callback
mode, pause state, direction, link interval and auto-action clearing.

The GPU model cache now includes the scene rebuild generation. Reusing a map
number after a transition therefore rebuilds the model/material handles instead
of displaying the previous stage. The selected texture variant is reloaded from
the destination stage. Original day/RNG-driven texture selection is unfinished;
the inspector continues using its supplied A-D selection.

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A --link-rebuild-capture /path/to/new.png 0 24
```

This development capture supplies wall-trigger zero as a cached collision
contact, executes the original wall/link policy and drives the effect/session
consumer with idle input. It does not verify a natural collision encounter.
The ordinary `--walk-links` mode also consumes rebuild events reached during
movement. GPU effect blending still requires implementation, so the scalar fade
controller does not currently make the framebuffer visibly fade.

## Evidence and limits

Native CPU execution supplies 756 session link cases, 252 tick cases and 40
sender-routing cases. They cover rebuild/end-day/ignored codes, signed cinematic
variants, state changes during the base tick, and stop/destroy/create/attach/run
order. Original current-map and routing routines execute directly. Parent/base
scene operations, scene construction/destruction and day/cinematic services have
explicit supplied boundaries.

Another 160 native cases compare scene-motion reset fields and actual original
angle getters/setters. Chunk mood accounting is supplied; the runtime emits a
mood request for the future scoring consumer. Unmodelled actor/cache ownership
and preserved low cinematic fields remain outside this reset integration.

Seventeen private original-input scenarios provide permanent-wall contact keys,
then compare destination placement, deferred rebuild, native motion reset,
effect completion and preserved RNG/timer state using imported maps and tables.
Two Bevy captures show the destination geometry after supplied requests. Original
images/assets remain private. The unchanged traversal capture guards the path
which reaches no link.

The `--link-rebuild-live-capture` option uses the same arguments and runs the
24 host ticks while Bevy's renderer is active. It checks the resident mesh/texture
cache change as well as the session state. Both live final images matched their
pre-render final-scene snapshots. These are port rendering checks, not original
GPU or visual-fade comparisons.

The scene host uses a supplied zero origin, synchronous local reads and a
development callback order. Original generation-driven scene configuration remains open.
Original CD timing, complete actor/parent destruction and reattachment, linked
flashback setup, every special scene, day preparation, GPU effect blending,
audio, NPCs and full original-session comparisons remain incomplete. These
checks do not accept a whole original feature group.

See [session-recovery-checks.json](session-recovery-checks.json) for fixtures,
hashes and exact validation. Whole original acceptance remains **0/40 provisional
feature groups**. The matching C decompilation remains paused.
