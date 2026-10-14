# Animal actions and player-facing policy

The day profile now applies the original object-facing operation when an
enabled entity has a negative interaction distance. The operation uses the
actor's local position and the player's cached world position, retains the SDK
integer angle lookup and degree conversions, clears pitch/roll and invalidates
the coordinate cache. It happens during interaction before the action callback.
Waiting notifications still perform interaction/facing while holding action
and animation counters.

The scene adapter receives the imported aim table alongside the special model.
It applies facing to the actor immediately. The older request-only interface
remains available for controlled policy fixtures that do not supply an actor
transform backend. Unsupported target data remains an explicit fault.

## Original actions

Bear and Rabbit share an initial random choice: a roll of eight or nine triples
the first scale axis; five through seven select state ten. In state ten, ticks
from 201 onward apply the original world-axis backward translation.

Bear computes its sound-distance ratio after that shared update and requests
its two notes on animation frames zero and fifteen. From tick 321, it moves
sideways by sixty units and turns nine degrees according to two random calls.

Rabbit requests its note on frames seven and twenty-two. Ticks 300 through 319
move forward by sixty units; 321 through 339 turn without consuming randomness.
From 340, the callback uses two random calls for a sideways movement and turn.
The sideways movement emits notification seven before the random turn.

Peacock executes the original player-facing operation on every active action
update. It makes no random call or sound request.

All three run with original context/action/animation ordering, integer widths,
signed tick comparisons, fractional setter packets and synchronous notification
order. New AI behavior is not used to replace these recovered scripts.

## Evidence

Sixty owned action scenarios compare 3,380 states, including complete zero-to-370
trajectories, four seeds, frame-gated sounds, timer boundaries, negative ticks and
signed tick overflow. The original model/animation constructors and full actor
updates execute in the native runner. An owned 24-frame model and supplied
trigonometric/angle tables keep the public replay independent of game files.

Six further scenarios compare 1,570 states using locally imported original
encounter declarations, property rows, models and math tables for all three
callbacks. These detailed observations remain private. Public checks identify
their inputs, scope and hashes.

Forty-eight original interaction-policy cases cover link/trigger state, distance
sign, all quadrants, equal position and differing local/cached player positions.
The scene-adapter test compares waiting-state results to those original
observations, then compares its Peacock action result to the original action.
The adapter does not expose native function-call instrumentation.

Both recorded native outputs are reproduced identically by a second run.
The native graph has real model/entity/player construction and a valid transform
parent, with supplied target/view/cache and an external audio service.
Game-function outputs are not replaced. The semantic GTE runner does not accept
hardware timing, original GPU/SPU output or a natural encounter.

```sh
cargo test -p lsd-assets --test native_pursuit --locked
LSD_GAME_FILES=/path/to/local/files \
LSD_NATIVE_PURSUIT=/path/to/private/original-pursuit-cases.json \
  cargo test -p lsd-assets --test native_pursuit --locked -- --ignored --nocapture
```

See [animal-action checks](pursuit-checks.json) for input/function hashes,
regressions and the current validation results.

## Remaining parity work

Following [player-interacting NPC recovery](PLAYER_NPCS.md), there are now 19
implemented callbacks among the 107 enumerated distinct original action
callbacks; 88 remain pending. This count measures callbacks,
not whole NPC encounters or game completion. Reciprocal collision response,
natural transform/cache refresh, complete lifetimes, native rendering/audio and
the remaining project-objective requirements still need implementation and
comparison. All 40 provisional whole-feature groups remain unaccepted.

C decompilation stays paused and preserved. Original NPC behavior makes no AI
model calls; AI analysis and optional gameplay mods retain their separate scope.
