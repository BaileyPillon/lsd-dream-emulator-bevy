# Shared scene clock

The loaded Bevy scene now uses the original clock order rather than advancing the
player before the actor and generation channels. `StageLinkSession::step_frame`
selects one signal and traverses a live subscriber list. Existing actor callbacks
run before map generation, stream work, render-state capture, the session counter
and player callbacks. Pad polling/dispatch and session handoff happen afterward.
Active color effects retain their own clock membership.

The static role order comes from the read-only original-emulator graph recorded
in [frame order](FRAME_ORDER.md). Actors and effects prepend when attached and
remove their subscriptions when detached. Registration changes are consumed
after each callback. A new head does not interrupt the current cursor, so an
actor constructed during stream work renders its initial pose but gets its first
update on the next notification.

Stream selection and cached-key refresh use the player's position before that
player's current callback. The renderer phase similarly records player/camera
state before the callback. Bevy's camera uses that snapshot. Original GPU packet
sorting, draw/swap and cross-map residency still need implementation.

`LinkedPlayer::begin_frame` clears reports once before traversal. Player work
does not clear earlier actor commands. Session handoff preserves earlier journal
events, handles pending day/link services and retires old actor subscriptions
when rebuilding a map. A new dream resets clock flags, memberships and counters.

## Evidence

8 scenarios execute the original display timer, live clock, owned MOM/TMD/entity,
DreamSys/pad/scene/renderer constructors and entity callbacks. A supplied MIPS
stream callback calls original constructors and attaches three entities while
clock notification is in progress. The comparison includes 192 display events
and 24 actor births, covering waiting, normal and finished signals.

The Rust comparison checks membership, callback order, actor clocks/context,
animation parts, shared RNG and sound-request order. No new entity updates on
its birth notification. Finished entities detach and retire during traversal.
The real renderer remains inactive in this owned graph. The external unavailable
audio backend and firmware controller boundary are supplied; game function
outputs are not replaced. The player has a real transform-parent binding and
provided target/cache state. This is separate from imported map/CD/GPU parity.

An opt-in test using the original local corpus runs the actual imported-scene
host. It compares static roles to the original-emulator observation, checks zero
ticks on newborn actors, a shared waiting signal that holds actor/player clocks,
Futon journal events surviving handoff, and render state preceding the player's
look callback.

```sh
cargo test -p lsd-assets --test native_shared_clock_birth --locked
LSD_GAME_FILES=/path/to/local/files cargo test -p lsd-assets \
  --test shared_scene_frame --locked -- --ignored --nocapture
```

See [shared-frame checks](shared-frame-checks.json) for hashes and validation.
The workspace passes 127 tests with 25 local-data tests ignored. Original-data
regressions, all-feature Clippy, formatting and desktop builds pass. Four
development captures are unchanged; the walking capture changes with stream and
render work occurring before player movement. The supplied Yokai still runs
48 body queries with no pending notifications and no contacts at its provided
position. Original assets, RAM, firmware and captures remain private.

## Remaining fidelity work

One logical clock now coordinates the loaded simulation. Native CD completion,
waiting triggers, display/GPU blocking, original matrix/cache refresh and full
render residency are not accepted. The map display/texture readiness policy,
additional subscriber types and NPC-requested births/destruction also need full
native lifecycle work. Model movement response, the remaining 95 callbacks,
original GPU/SPU and playback, menus, persistence, media and endings remain
unfinished.

The shared order and owned birth comparisons do not accept whole encounters or
any of the 40 provisional feature groups. The default original behavior makes no
AI model calls. C decompilation remains paused and preserved.
