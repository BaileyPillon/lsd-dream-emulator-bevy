# Actor collision bindings and prepared clouds

Imported actors now keep the collision geometry copied from their original root
part during construction. The geometry includes model bounds and ordered faces.
Prepared clouds retain the original cuboid expansion, signed movement extension
and saturated cached-world matrix application. Live collision query/response
integration remains unfinished; whole encounters and feature groups remain
unaccepted.

## Implementation and evidence

`ModelActor::collision_primitive` records the initial root binding separately
from the animation part's current render primitive. `Mom::collision_model` imports
that TMD object's vertex bounds and ordered triangle/quad faces. `SceneActor`
retains the imported geometry throughout its existing bind/retire/rebuild lifetime.

The canonical comparison runs real original allocator, MOM, model, SDK and actor
constructors on 100 identified local model inputs. It records the collision
primitive, root render primitive and native bounds, then repeats them over eight
animation steps: 900 observations. The copied collision binding remains unchanged
in these sequences. This is a measured construction/update subset, not proof that
every possible root rebind or resource lifetime is recovered.

`CollisionModel::prepared_cloud` expands the imported bounds to eight signed
halfword corners. It can extend forward or strafe corners using the original
ordering and wrapping arithmetic, then apply a supplied cached world matrix.
504 owned CPU cases compare the original bounding-box, cuboid, extension and
in-place SDK vector routines. They include signed extreme bounds, extension
wrapping, zero/rotation/scale/shear matrices and IR saturation. Original function
outputs are not replaced. See [geometry checks](actor-geometry-checks.json).

```sh
cargo test -p lsd-core --test native_prepared_cloud --locked
LSD_GAME_FILES=/path/to/extracted/files LSD_NATIVE_ACTOR_GEOMETRY=/path/to/private-observations.json cargo test -p lsd-assets --test native_actor_geometry --locked -- --ignored --nocapture
```

The canonical construction comparison supplies initialized CD and an SDK unit
matrix. In-memory file data is isolated from the allocator arena and SDK globals.
The earlier exploratory body probe placed model data over SDK BSS and omitted
SDK transform initialization. Its cached world matrix was zero and collapsed the
cloud. Corrected exploration uses separate file storage and initialized SDK/cache
inputs; original `GsGetLw` and cloud routines run. Those explorations still produced
no contacts and are not collision acceptance evidence. Earlier animation and
constructor snapshots did not accept native world cache or live collision output.

Original models, vertices, rows, detailed canonical observations and captures
remain private. CI uses owned numeric input/output data. The local workspace
passes 117 tests with 22 opt-in tests ignored. Canonical regression and five
rendered checks pass; idle, Futon, Yokai and legacy captures remain unchanged,
and dream expiry removes actor render instances. The paused C checkout is
preserved.

## Remaining requirements

Execute pending inherited queries against prepared actor/player clouds and real
model faces, with original transform ancestry, cache refresh and contact callback
feedback. Implement movement correction, ground reconciliation, natural scene
ownership/cadence and complete geometry/resource binding behavior. The Yokai
development capture still exposes 48 pending inherited queries. This count and
the geometry comparisons do not establish physics completion. Original GPU/SPU,
remaining NPC callbacks and the other full-game requirements remain open; optional
AI tools/mods stay separate.
