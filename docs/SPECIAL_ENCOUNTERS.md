# Special encounter activation

The original game loads one shared special model during setup and creates one
dormant entity 98 in each scene. Construction disables its link and action;
it has no clock subscription until activated. The Bevy day profile now imports
the model path, placement offset and SDK angle table from the identified local
executable and model. Original content is not distributed in this repository.

When a matching chunk fails its day gate, the original encounter program consumes
one random value outside stage zero. An even day and a value divisible by twelve
activate the special instance. Activation stops an attached instance, transforms
the supplied offset through the player's matrix, adds its cached world position,
starts the same actor, then faces it toward the player. Repeated activation
preserves the instance identity and prepends its clock subscription again.

PsxAimTables implements the original SDK ratio lookup, quadrant rules and
overflow-avoidance branch. Object facing preserves the original zero-difference
fallback, integer degree conversion and second conversion through the object's
angle packet. This is separate from the pending continuous-aim interaction
requests on other NPCs.

Scene creation also initializes teleport availability for stages three and
eleven before encounter groups can enable it. Releasing an ordinary map slot
preserves the special instance. Scene rebuild/teardown removes it and the next
scene receives a fresh dormant actor.

## Evidence

The native comparison executes original model/entity constructors,
link_spawn_special_entity, link_activate_object, stop/start, local-to-world,
object-facing and ratan2 routines. It checks two dormant constructions and
twelve activations across supplied player poses and rotations. Six activations
use an owned model and lookup table; six use the original local special model
and angle table. It compares actor pose, multipart state, sound context,
visibility/action flags and clock membership. A further 425 SDK angle cases use
an owned lookup table to exercise quadrants and the two integer ratio paths.

The local-content tests verify the imported rotation matrices and model
identity. A loaded stage-two scenario also reaches an original even-day chunk
gate with a supplied seed, activates one special instance, avoids repeating
the gate for an unchanged resident chunk, preserves the actor on ordinary slot
release and recreates it on scene rebuild.

```sh
cargo test -p lsd-assets --test native_special_encounter --locked
LSD_GAME_FILES=/path/to/local/files cargo test -p lsd-assets \
  --test native_special_encounter --locked -- --ignored --nocapture
```

The scope and input/code hashes are recorded in
[special encounter checks](special-encounter-checks.json). The native graph has
a real transform parent with supplied player pose/cache state and an external
sound boundary. Its GTE runner preserves the relevant integer operations;
hardware flags and timing require separate comparisons.

## Remaining work

The Gray Man's named action callback remains pending. Pursuit movement,
synchronous contact consequences, its actor-attached color effect, the
flashback-count reset and complete effect/actor lifetime still need to be
connected and compared. The activation test does not invoke that callback.
Activation therefore does not accept a complete Gray Man encounter, its rendered
appearance, original GPU/SPU behavior or a whole game feature group.

The implemented callback count remains 12 of 107, with 95 pending.
All 40 provisional whole-feature groups still need acceptance. The existing
day-profile captures remain regression evidence for their supplied scenarios;
they do not accept the newly activated Gray Man's rendering.
Original behavior makes no AI model calls. C decompilation remains paused.
