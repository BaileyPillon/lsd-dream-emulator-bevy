# Cached world state during movement

Moving-actor collision notifications now carry their cached world position and
matrix separately from the new local pose. The previous adapter prepared its
cloud and relative position from that new pose. Controlled original CPU movement
traces show that the original notification reads the earlier cached world state.
Local movement invalidates the cache flag; it does not refresh the cached matrix
or position before notifying parents.

`BodyWorldFrame` names the supplied cached matrix/position. `PreparedActorCloud`
retains that position beside the transformed corners. The journal geometry query
reads it directly. The encounter adapter holds its supplied frame at actor update
entry through script movement and pose changes. This corrects within-call cache
use; it does not establish the complete original renderer/frame schedule.

The scene-node margin at offset `0x54` is a signed word, initialized to zero.
The original wrapper selects the moving side of the cloud using the movement
halfword's sign, then adds or subtracts the margin with word wrapping. Margin
adjustment can reverse the extension's sign without changing the selected side.
The port now retains that distinction and the halfword corner wrapping. Strafe
reorders corners; original event eight still uses the forward cloud extension.

## Evidence

270 cases execute complete original X/Y/Z movement methods and the scene-node
cloud wrapper with real owned MOM/entity construction. They compare differing
current/cached rotations, unit/half/six-times scale, movement truncation, positive/
negative margins and signed word overflow. Each captures cached and local state,
the transient notification cloud, and separate SDK/matrix refresh observations.
`GsGetLw` alone reads an existing local matrix. Rebuilding that matrix from Euler/
scale fields is a separate step; both original steps execute in the comparison.

32 additional owned cases execute full movement, player geometry, contact gates
and recursive link/video/end journal consequences. Twenty produce a contact.
16 private cases repeat the full path with original Yokai and `DREAME5.TMD` meshes;
twelve produce a contact. Positions/properties, identity parent ancestry, initialized
cache epoch and external notification/audio services are supplied. Original
function outputs are not replaced; these comparisons accept the identified path.

Evaluating the same cloud at the new local position disagrees with original
contact output in 20 of the owned scenarios and eight of the original-mesh
scenarios. This comparison detects the previous adapter bug and distinguishes
the corrected cache input from a superficially similar fresh-transform query.

```sh
cargo test -p lsd-assets --test native_actor_cache --test native_actor_player_query --locked
LSD_GAME_FILES=/path/to/files LSD_NATIVE_MOVING_ACTOR_PLAYER=/path/to/private-observations.json cargo test -p lsd-assets --test native_actor_player_query --locked full_original_yokai_movement_pair_contacts_use_pre_movement_cache -- --ignored --nocapture
```

See [cache checks](actor-cache-checks.json) for hashes and scope. Public fixtures
contain owned meshes/numeric inputs; original assets, canonical geometry
observations and captures stay private.

The workspace passes 121 tests with 24 opt-in tests ignored. Canonical regression,
formatting, all-feature Clippy and desktop build checks pass. Five rendered
profiles retain their previous images. The supplied Yokai profile still executes
48 queries with zero pending notifications and zero contacts; positive contact
evidence comes from the separately supplied pair scenarios above.

## Remaining requirements

Recover natural transform parent graphs, SDK cache invalidation/refresh across
the actual frame schedule, parent rotation/translation/scale and initial/reused
cache storage. The loaded adapter currently supplies an identity parent frame
derived at update entry. The comparisons supply cache/refresh timing explicitly.
Implement reciprocal player/actor delivery, movement correction and ground
reconciliation, then verify natural encounters. Keep original GPU/SPU, remaining
95 callbacks, audio, menus, persistence, media and other full-game requirements
open. Whole feature acceptance remains 0/40 provisional groups and whole NPC
encounters remain unaccepted. Default gameplay makes no AI model calls; the C
checkout remains paused and preserved.
