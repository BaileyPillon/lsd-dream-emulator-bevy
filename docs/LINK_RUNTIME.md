# Dream link handlers and actor keys

Rust now implements the original dream-system handlers for tunnels, instant
teleports, dynamic links and stairs. Tunnel eligibility retains the existing
transition, facing and forward gates. Teleports preserve the original handled
result when notification cancels the transition; successful teleports clear the
event, move the actor and apply the cottage time bonus in original tick units.
Dynamic links preserve their RNG calls, transform clearing and default-transform
selection state.

All four stair scripts retain their normal/fast completion thresholds, turn
windows, look windows, first-step translation, step wrapping and movement
requests. Stair initiation applies the source rotation and immediately runs its
first step. Completion unlocks input and restores the previous fast movement
direction. The stair handler returns zero throughout, allowing the remaining
link dispatcher to continue in the original order.

`DreamLinkEffects` exposes the original notification, sound, rotation, translation
and relocation boundaries. Effects can mutate live state. This interface has no
default stand-in game-flow implementation. The original stair targets and turn
packets are loaded locally with the other link tables, and the native action
table is checked before selecting a Rust script.

Actor keys retain separate current and contact caches. A current-key query can
return the existing cache or refresh it from world coordinates; a failed refresh
preserves the cache. Body dispatch stamps the current cached chunk with each
candidate cell while retaining the position tail. The chunk is retained even
for a neighboring slot. Spawn placement preserves signed packed coordinates and
the original spatial/layered rules.

## Native evidence

The new runtime fixture contains 512 handler cases, 400 stair boundary ticks and
8 persistent stair sequences covering 582 states. These execute the original
dream handlers, the four complete scripts, rotation conversion/setters, position
setters, actor spawn-to-world conversion and time-limit helpers. The graphs and
tables are synthetic and owned by the project. Transition notification is an
explicit accept/cancel boundary; it does not execute original game-flow state.

A separate key fixture checks 172 original cache-access/refresh cases, 144 actor
spawn-to-world placements and 16 body-dispatch stamps. Body tile notifications
are supplied sinks, so these stamp observations do not accept geometry-to-link
callbacks. Rust matches cached values, placement, handler/control state, RNG
state, transform selection, movement and callback order within those scopes.

See [link-runtime-checks.json](link-runtime-checks.json) for executable/function
identities, fixture/source hashes, counts and validation. The original C checkout
remains paused and preserved.

The subsequent [linked scene integration](SCENE_LINKS.md) connects these handlers
to streamed movement and ports the wall/floor and map-notification policies.
Natural construction and cache lifetimes, complete transform state ownership,
original transition scheduling, content
rebinding, audio and full dream encounters remain open. Original whole-game
acceptance remains **0/40 provisional feature groups**.
