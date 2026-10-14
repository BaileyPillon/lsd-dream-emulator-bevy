# Body dispatch and tile attributes

Streamed body queries now use the original surrounding-cell rectangles. Spatial
rectangles split across staggered neighbor slots; layered rectangles clamp at
the map edges. Ready slots are traversed in the original order, with ascending
rows and each cell's chain visited before the next cell.

The original dispatcher tests bit `0x80` in the node's attribute halfword at
offset `0x36`. Collision tiles now retain their original flags and category,
and body queries use that gate. Floor queries retain their separate rules.
Every eligible candidate is tested; later successful contacts replace earlier
ones, matching the original code-4 contact assignment. Scene/actor side effects
after notification still require implementation.

## Checks

The numeric fixtures record 144 original body-selection cases and 740 dispatched
nodes in supplied graphs, including chained tiles, edge/corner splits, permuted
slot labels and unready slots. Tile notification is captured at its virtual
boundary; sender and message arguments are checked. A separate scalar run checks
contact assignment. These observations do not execute the entire native
transform, geometry and actor callback graph.

Another fixture records 168 executions of the original tile reader, including
primary and chained entries, signed heights, wrapped headings, flags/category
copying, absent records and termination. Primitive lookup uses an explicitly
supplied synthetic boundary. Existing Rust tile decoding is compared with the
reader's output.

An integration test places two eligible walls across a map boundary and checks
the final contact and flag gates. Original-input stream checks and Bevy captures
exercise the updated queries. See [body-dispatch-checks.json](body-dispatch-checks.json)
for source hashes and exact scope.

Natural native binding/allocation, actor/MML lifetimes, complete body transforms,
link outcomes, floor effects, session timing, original NPC behavior and end-to-end
game comparisons remain open. Whole original acceptance remains 0/40 provisional
groups.
