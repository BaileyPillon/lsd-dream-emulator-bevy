# Actor journal consequences

The day profile now delivers entity journal notifications during the emitting
actor's interaction update. The original mood, score, recall, link and dream-clock
rules run with the same RNG used by subsequent actors. This advances encounter
behavior without accepting a whole NPC encounter or whole-game feature group.

## Recovered policy

The journal wrapper invokes the inherited handler first, then filters the
sender's low 20 type bits. Entity effects apply only while link state is zero.
The Rust policy retains these gates and the original signed property getters.
The inherited code-four handler now stores the sender before either guard.
The earlier snapshot did not measure this base contact field and its adapter
omitted that mutation. [Entity body routing](ENTITY_BODY.md) records the
correction and the connected acknowledgement consequences.

| Notification | Consequence |
| --- | --- |
| 4 | Route acknowledgement back to the emitting actor |
| 9 | Outside recall mode, log the instance mood, add the signed unlock score and attempt a 16-second recall capture |
| 10 | Decode the requested stage and run the direct dynamic-link handler |
| 11 | Outside recall mode, select the event video and expire the dream clock |
| 12 | Outside recall mode, expire the dream clock |

Score addition wraps as signed 32-bit arithmetic. Recall capture retains the
original RNG gate, record limit, current spawn and angle conversion. Direct
dynamic links select a spawn and emit event 12 with its sound request. The map
observes the negative requested stage before the destination is committed and
chooses its state-six fade. Cancellation restores the source stage while
retaining spawn/RNG/penalty changes made before the cancelled notification.

`EncounterScene::tick_with_events` dispatches notifications immediately.
`StageLinkSession` supplies the current day, player key, link/map state and
shared RNG, then consumes newly emitted effect commands in that host tick.
Deferring the entire actor queue until after callbacks would change RNG order.

## Evidence

The 336 owned scenarios execute the identified original journal wrapper,
entity getters, mood logging, recall storage, direct dynamic-link selection and
link execution using real entity/model construction. They vary event/type/state
gates, recall mode, signed fields, score overflow, clock widths, record capacity,
attachment and cancellation. External key, notification and audio services and
initialized dream state are supplied. Original function outputs are not replaced.

The original checkpoint comparison checks state, mood, score, recall records, spawn, RNG,
penalty and request order. Acknowledgement routing is compared; the native
fixture's inherited body-target observation is explicitly excluded from the
accepted state comparison. The additional coupled comparison now accepts both
actor and DreamSys contact fields and recursive journal consequences in a
supplied original parent subscription graph; real geometry remains unfinished.

A private integration check loads the original day-one Futon model and properties
and supplies a nearby player position. It verifies one mood contribution, an
instance score of -50,000, immediate recall RNG use and no duplicate contribution
on the following tick. Its rendered capture retains the same scene as the previous
callback checkpoint, with zero pending trigger notifications. Idle and legacy
movement captures remain byte-identical; day expiry removes all actor render
instances. See [journal checks](actor-journal-checks.json) for input identities,
hashes and the validation scope.

```sh
cargo test -p lsd-core --test native_actor_journal --locked
cargo test -p lsd-assets --test actor_journal_integration --locked
LSD_GAME_FILES=/path/to/extracted/files cargo test -p lsd-assets --test native_actor_journal_scene --locked -- --ignored --nocapture
```

CI uses owned numeric fixtures without original game assets. Original executables,
tables, assets, detailed original observations and captures stay private.

## Remaining requirements

Implement inherited geometry/body queries, movement collision consequences and aim
rotation, the remaining 95 callbacks, Gray Man activation, cinematic playback,
full scene construction/lifetimes and native scheduling. Compare original GPU
and SPU output with the running port. Notification 4 currently queues an
acknowledgement request when no actor subject is available. Bound actor delivery
now stores its permitted contact and handles the resulting notification before
returning. Geometry queries and aim requests remain pending. Video selection
sets descriptors and clock state; it does not play a movie. Sound requests do
not establish audio playback. Whole NPC encounters and whole-game feature groups
remain unaccepted. Optional AI analysis/mods remain separate and make no model
calls in the default simulation. The matching C checkout remains paused.
