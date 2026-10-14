# Body transforms and native hit checks

The Rust geometry query now applies the original fixed-point matrix rules to
body collisions. It transforms the actor's world-oriented collision cloud and
position difference into the target's local coordinates, adds the offset with
signed halfword wrapping, checks aggregate bounds and runs the existing face
query. Differences outside the original 16,384-unit gate are rejected.

Inverse rotations start with the local node and multiply each parent in order.
Each matrix multiplication retains the SDK's shift and saturated short results.
Static map tiles use this shared query with their yaw rotation. The generic API
supports supplied parent chains; natural actor construction and runtime parent
management still need implementation.

## Original comparisons

The identified Japanese executable is executed locally with a MIPS CPU backend
and an explicit software GTE bridge. The original SDK routines produce 24 matrix
application cases and 16 matrix multiplication cases, including saturated
halfwords. Four original short-vector-loop runs agree with the SDK application
outputs. Rust matches the public synthetic numeric fixtures.

A separate fixture executes the original hit path through parent rotations,
vertex bounds, face geometry and tile/actor notifications. It supplies allocated
transform graphs with cached world positions, one synthetic quad and an
eight-point collision cloud. The 120 cases vary local and parent cardinal
rotations, position and the midpoint filter. The native path reaches 27 original
functions, produces 36 hits and uses no function-result stubs or captured virtual
callback boundaries. Rust's geometry query matches all 120 hit results, observed
through the native actor's contact assignment.

The Rust fixture comparison checks hit presence. It does not compare the complete
native callback workspace, full actor state or contact coordinates. General
angles have earlier isolated rotation observations, but the new complete hit-path
fixture uses cardinal rotations only.

The bridge implements the rotation/no-translation MVMVA mode used by these SDK
routines, alongside the existing OP/SQR and register-transfer operations. It does
not establish hardware timing or complete GTE flag behavior. The instruction
reference is [PSX-SPX's GTE documentation](https://psx-spx.consoledev.net/geometrytransformationenginegte/).

See [body-transform-checks.json](body-transform-checks.json) for executable and
function identities, fixture hashes, source hashes and validation scope. Local
validation also checks original-asset streaming and two Bevy capture regressions.
Original captures and assets stay private.

Natural allocation, real session actor graphs, link actions and their RNG calls,
floor effects, original NPC scripts and complete rendering/audio comparisons
remain open. Whole original feature acceptance remains **0/40 provisional
groups**. C recovery stays paused and is tracked separately.
