# Actor clock subscriptions and frame signals

Actors now update in their original clock attachment order. Each new attachment
prepends a member, so the most recently attached actor runs first. Stop removes
the member, and restart places it at the front again. Resident storage remains
separate from subscription order; reversing constructor order alone would fail
the restart case and assign RNG results to the wrong actors.

The original binding path is `map_scene` setup to `link_spawn_special_entity`,
`link_create_object`, `entity_start` and `func_80065918`. The legacy
`g_LinkSceneData` argument is the shared frame-phase object passed as the actor's
third start argument. That attachment subscribes the entity's update handler.
The separate player-system attachment retains target/journal ownership.
Original `base_class_add_parent` prepends and `frame_phase_notify` traverses the
resulting list. The port now tracks that actor subscription list explicitly.

| Frame signal | Actor consequence |
| --- | --- |
| Code 2 | Interaction policy, then action/sound and animation updates |
| Code 3 | Interaction policy; action/animation clocks remain unchanged |
| Code 4 | Interaction policy, then actor cleanup and subscription removal |

`FramePhase` selects one signal before dispatch. A nonzero finished flag takes
precedence over waiting. A normal tick increments the signed phase counter with
word wrapping; waiting and finish signals leave it unchanged. Reset clears flags.
`StageLinkSession` supplies this clock to its encounter channel. The current
loaded profile supplies normal tick signals; waiting/finished paths are available
to its scene host. Automatic native CD/wait/renderer synchronization remains open.

## Evidence

32 scenarios construct 96 real original owned entities and original frame clocks.
They use Television, Futon, Drumstick and empty callbacks in three-actor groups,
four seeds, normal/random interaction policies, stop/restart, waiting, finish and
counter wrap. Twelve operations plus initial state give 416 comparisons. Original
entity start/stop, linked-list subscription, clock notification, actor update,
callback, animation and destruction routines execute. Comparisons include live
membership/order, each actor's clocks/context/parts, RNG and sound requests.
Finished original notifications remove actors during the native traversal.

36 scalar cases observe the notification code from the actual original function
and compare counter/flag behavior. Original function outputs are not replaced.
Owned MOM primitives are empty; typed root/target coordinates, initialized SDK/CD
and external audio services are supplied. The controlled target is outside the
interaction trigger radius, while remaining within the sound callback range.
Target/perception geometry and natural encounters are separate requirements.

```sh
cargo test -p lsd-core --test native_frame_phase --locked
cargo test -p lsd-assets --test native_actor_clock --locked
```

See [clock checks](actor-clock-checks.json) for hashes and scope. Public fixtures
contain owned input data. Original assets and captures stay private.
The workspace passes 123 tests with 24 opt-in tests ignored; canonical regression,
all-feature Clippy, formatting and desktop build checks pass. Five rendered
profiles retain their previous images, and dream expiry removes actor render
instances. The supplied Yokai profile retains 48 executed body queries and zero
pending notifications; its position produces zero contacts.

## Remaining schedule and fidelity work

The port's clock currently covers the encounter slice. Recover and connect the
actual relative order of player, map, generation, renderer and CD subscribers,
their attach timing and global phase ownership. The later
[live traversal recovery](FRAME_CURSOR.md) replaces the initial membership
snapshot with a cursor and retires finished actors during iteration. Supplied
mutation callbacks compare list behavior separately. Original NPC-requested
births, detach/restart and destruction of other actors still need connection to
the scene graph and full resource lifetimes.

Recover natural transform/cache scheduling and parent graphs, reciprocal body
delivery, movement correction and ground reconciliation. Complete the remaining
95 NPC callbacks, original GPU/SPU, audio, menus, persistence, media and other
full-game requirements. Whole feature acceptance remains 0/40 provisional groups
and whole NPC encounters remain unaccepted. Default gameplay makes no AI model
calls; the C checkout remains paused and preserved.
