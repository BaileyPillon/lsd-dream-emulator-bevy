# Moving actors and player geometry

Moving actors now deliver a prepared collision cloud to the imported player
mesh during their notification. A hit stores the player's actor contact before
calling the entity's signed stage gate. Its link, video or dream-end notification
runs immediately with the shared RNG. No deferred request substitutes for this
query when the loaded profile supplies both geometry and math services.

The day profile now uses `DREAME5.TMD` for player bounds and faces. Original
`game_flow_construct` passes that model to DreamSys. The dream session separately
loads `DREAMER.TMD` for scene/generated helpers. The earlier profile used its
flat object-zero bounds for the player. That choice also prevented exploratory
moving-actor/player contacts. The current asset choice follows the original
construction roles; original data is still imported locally.

`InteractionHost` uses the copied actor root geometry, signed movement halfword,
forward/strafe corner ordering and saturated rotated/scaled cloud. The cloud
exists only in the synchronous call. `ActorJournalDispatcher` runs the existing
inverse-rotation, distance gate, bounds and ordered face query before journal
sender/state filtering. Code-four feedback keeps receiver and actor contacts
separate, including rejected entity gates and busy link state. Callers without
the necessary services retain explicit pending requests.

## Verification

300 original CPU scenarios use real owned MOM/model/entity constructors, valid
identity transform ancestry and an original linked-list parent subscription.
They vary event 5/6/7, forward/strafe clouds, distance rejection, low-height
filtering, signed stage gates, stage-127 video/end and busy link state. Forty
scenarios produce a player contact. Rust comparisons include both contacts,
prepared cloud, link state/stage, spawn, clock, cinematic, penalty and RNG.
Ten SDK `ScaleMatrix` cases compare signed column scaling and 32-bit overflow.

Twenty private scenarios use the original Yokai MOM and `DREAME5.TMD` player
mesh with supplied properties/positions and identity cache/ancestry. Four
produce a contact. Original geometry, entity handlers and recursive journal
dependencies execute; original function outputs are not replaced. Initialized
DreamSys state, current cached positions and external notification/audio services
are supplied. These are controlled mesh-pair comparisons, not natural encounters.

```sh
cargo test -p lsd-assets --test native_actor_player_query --locked
cargo test -p lsd-core --test native_scale_matrix --locked
LSD_GAME_FILES=/path/to/files LSD_NATIVE_ACTOR_PLAYER=/path/to/private-observations.json cargo test -p lsd-assets --test native_actor_player_query --locked -- --ignored --nocapture
```

See [query checks](actor-player-query-checks.json) for measured identities and
validation. Public fixtures contain owned inputs. Original assets, canonical
geometry observations and captures remain private.

The local workspace passes 119 tests with 23 opt-in tests ignored, plus the
canonical regression suite, formatting, all-feature Clippy and desktop builds.
Five rendered profiles match their previous images and expiry retires actor
instances. The supplied day-eight Yokai profile now executes 48 inherited
queries over 63 active ticks, leaving zero pending notifications. That placement
produces zero contacts; positive contacts come from the separately identified
supplied pair scenarios above. Query execution alone does not accept physics.

## Remaining work

The loaded adapter currently derives world pose from its supplied scene/player
state. Recover natural transform ancestry, SDK cache refresh timing and native
frame scheduling before accepting their fidelity. Implement actor/player movement
correction, ground reconciliation, reciprocal player/actor queries and complete
scene/resource lifetime. Keep the remaining 95 callbacks, original GPU/SPU,
audio, menus, persistence and other full-game requirements open. Whole feature
acceptance remains 0/40 provisional groups, whole NPC encounters remain
unaccepted, and default gameplay makes no AI model calls. The C checkout remains
paused and preserved.
