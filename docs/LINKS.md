# Original link rules

The local executable loader now reads 74 triggers: 17 wall links, 32 tunnels,
17 instant teleports and 8 staircase nodes. It also reads 239 random spawn
entries, their position adjustments, referenced destinations and eight
referenced facing/transform records. Original table contents stay in the user's
local executable.

The deterministic Rust resolver preserves first-match ordering, negative-tile
wildcards, teleport enable state, the cottage-only stair gate and navigation
challenge marking. Failed valid lookups preserve the output and link globals.
Facing uses the original signed halfword difference and inclusive 44-degree
window, with one 360-degree adjustment.

Random selection uses the original RNG. Normal selection consumes one result
for a destination among six stages and another for its spawn entry. A negative
fixed-stage request consumes only the entry-selection result. Dynamic links
retain stage/height/key eligibility, the odd-tick destination and the wrapping
penalty increment. Rejected requests consume no RNG results.

Transition policy assigns the event before notification. A callback can cancel
it before the destination is committed. Successful flashback transitions reset
the dream timer before the optional sound callback.

The movement/link dispatcher computes distance before its hook, then queries
stairs, teleport and tunnel callbacks in order. It reads auto-move state after
those callbacks, saves the transform before normal motion, applies the cottage
fall gate afterward and finally clears the transform marker. The host interface
allows callbacks to mutate live state at each original boundary. It does not
supply stand-in original actor or stair implementations.

## Evidence and remaining work

The numeric fixture contains 1,384 owned-table native cases: 350 static lookup
and wrapper cases, 102 random selections, 336 dynamic checks, 136 facing checks,
36 transform selections, 24 transition executions and 400 dispatcher cases.
The original MIPS functions execute locally. Lookup, selection and facing
functions run without function-result stubs. Transition and dispatcher fixtures
use explicitly supplied virtual notification, sound, actor and action
callbacks; these are checks of policy and ordering.

A local test additionally compares the imported original catalogue and 148
original-table lookup cases plus 102 original-table random selections against
native execution. Those original inputs and observations stay private and the
test is skipped by public CI.

See [link-recovery-checks.json](link-recovery-checks.json) for identities,
function ranges, fixture/source hashes, counts and exact scope. Invalid
catalogues or movement indices return Rust errors; the comparison covers valid
reference inputs, not original out-of-bounds memory access.

Natural actor keys, contact-to-link callbacks, transform save/restore lifetimes,
complete stair motion, teleport movement/time bonuses, tunnel traversal and
visible map transitions still need integration. Runtime timing, RNG calls from
the rest of the game and full session comparisons remain open. This work does
not accept a whole feature group; original acceptance remains **0/40 provisional
groups**. The matching C project remains paused.
