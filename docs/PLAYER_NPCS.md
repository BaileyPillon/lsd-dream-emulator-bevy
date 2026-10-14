# NPC actions that read or disable player controls

Dog, Gargoyle and Demon King now execute their recovered action scripts in
the loaded day profile. They share the original player action state, integer
facing, animation/context ordering and synchronous notification services.
Following [sound, motion and event recovery](ACTION_PHASES.md), there are 34
implemented action callbacks among 107 distinct original callbacks; 73 remain
pending. These counts do not measure whole encounters or
game completion.

## Original behavior

Dog chooses one of two movement speeds on its first sound-context tick. It
faces the player while approaching, preserves the original periodic note and
stop requests, and checks the action flag on even behavior ticks. Reading that
flag does not consume it. The reaction resets the behavior tick to minus one,
then rolls nine degrees and translates eight world units per reaction tick.
At its stop boundary it stops the context and animation/action callbacks,
then consumes the original random choice for its final state. Link and trigger
flags remain governed by the separate interaction lifecycle.

Gargoyle requests its three starting notes and clears the player's movement
and view callbacks on sound-context tick zero. It faces the player and moves
forward ninety units per action without a movement notification. Behavior
tick thirty emits the original direct dream-link request.

Demon King faces the player and clears both callbacks on every action. It
preserves its three-note requests during the first three ticks of each
ten-tick context period. At the selected animation's duration it disables
animation and requests a direct dream link; the action callback stays enabled.

The session adapter exposes the actual action flag and callback state. A link
scene reset reinstalls the view callback. Clearing movement mode two also
clears the original reset-view and active-view flags. The common link-timer
update still resets the action flag and updates the dynamic-link gate while
input is unlocked, independently of whether movement callbacks are installed.

## Original-code comparisons

| Comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Owned models, properties, angle/math and destination tables | 27 | 1,377 |
| Locally imported original models, properties and tables | 27 | 1,377 |
| Player callback clear adapter | 12 | 24 |
| Player clock after clearing callbacks | 8 | 16 |

Action cases cover four seeds, held/early/late/absent action input, reaction
and link boundaries, disabled animation/action, signed tick overflow and four
target quadrants. They compare actor/part pose, cache, context, animation,
RNG, sound commands, notifications, action and callback state, destination,
spawn, dream timer and dynamic-link penalty after each full actor update.

Link requests execute the original DreamSys receiver, destination selector
and link sound. The Rust replay uses the corresponding actor journal and
link runtime. The constructed original graph has a base Scene parent, whose
notification is observed but has no MapScene fade handler. It does not prove
the full map transition or fade. Separate existing map/session checks remain
identified by their own evidence.

The callback-clear cases use the production scene dispatcher. Clock cases
compare the actual linked-player callback gate with original code, including
retained versus cleared camera callbacks, both turn directions and input
locking. Controller state uses the original valid state values.

Both native recordings reproduce identical observations and function-entry
counts. Original game function outputs are not replaced. The runner supplies
target/cache state, a valid transform parent, initialized CD state and external
sound services, with its existing semantic GTE bridge. Original model/property
observations remain private; public replay inputs are owned test data.

```sh
cargo test -p lsd-assets --test native_player_npcs --locked
LSD_GAME_FILES=/path/to/local/files \
LSD_NATIVE_PLAYER_NPCS=/path/to/private/original-player-npc-cases.json \
  cargo test -p lsd-assets --test native_player_npcs --locked -- --ignored
```

See [player NPC checks](player-npcs-checks.json) for input/function identities,
validation and the preserved C checkpoint. Natural encounters, reciprocal
movement response, complete transform/cache timing, full MapScene effects,
rendering and SPU output remain unaccepted. All 40 provisional whole-feature
groups still require acceptance. C decompilation remains paused; recovered
original behavior makes no AI model calls. Optional AI tools and mods retain
their separate scope.
