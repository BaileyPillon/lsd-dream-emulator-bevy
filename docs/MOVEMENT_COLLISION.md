# Movement and collision recovery

The port now contains the original pad notification order, look/turn updates,
movement pulses, speed selection and head-bob state. Its geometry code uses
integer SDK rotation and square-root rules, segment queries and the original
collision-cloud pattern. A single-chunk prototype connects these pieces to
the imported stage geometry.

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 0 0 A --walk
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 0 0 A --walk-capture /path/to/new.png 32
```

W/S moves forward/backward, A/D turns, Q/E strafes, Shift selects running,
R/F changes look state, and Space sends the original action-button event.
The prototype finds a floor in the selected map and loads player bounds from
the original `DREAME5.TMD`. Original starting-stage selection, stream traversal,
link dispatch, NPC actions, audio and game menus remain open. Its 15-step/second
host schedule and supplied camera rest points are development settings pending
the complete original session scheduler and camera setup.

## Recovered behavior

Pad events distinguish pressed, held and released bits, with release taking
priority. The game delivers notifications in descending table-index order.
First press and held notifications have different codes. Look and turn updates
precede movement; pause, input lock and auto-move lock preserve their original
conditions. Movement uses four-step pulses, the original direction-dependent
speed table, signed rotation updates and the view's paired head-bob offsets.

Math tables are read from the user's canonical executable. Rotations retain
the SDK's intermediate rounding. Geometry queries preserve wrapped halfword
differences, low product words, truncated plane normals, Q16 fractions and the
expanded face bounds. The original uses those bounds instead of a barycentric
triangle test. Collision-cloud queries use the midpoint and two corner paths,
including the original contact overwrite behavior.

## Evidence and scope

The public reference fixtures contain 30 control sequences covering 960 steps,
70 rotation cases, 16 square-root cases, 540 face queries and 480 collision-cloud
queries. The control runs execute original CPU routines in supplied graphs,
with a named hardware-input provider and a captured motion boundary. The
rotation routine executes its original integer CPU code. Geometry runs use
the original CPU routines with explicit software COP2 transfer, cross-product,
square and leading-bit semantics. They do not verify GTE hardware timing or
the complete register/flag behavior of a PlayStation.

Those COP2 semantics were checked against the
[PSX-SPX GTE reference](https://psx-spx.consoledev.net/geometrytransformationenginegte/).
Original function hashes, fixture hashes and exact coverage appear in
[movement-collision-checks.json](movement-collision-checks.json).

A local decoder-to-query audit matches one vertical segment probe for each of
13,037 static TMD object instances across all 418 LBD maps and the two standalone
TMD files. This covers 20 static primitive variants and 111,177 packets reached
by the original iterator. The inputs and detailed observations stay private.
Set `LSD_GAME_FILES` and `LSD_NATIVE_GEOMETRY_CASES` to absolute paths, then run:

```sh
cargo test -p lsd-assets --test native_geometry_corpus --locked -- --ignored
```

One probe per object establishes this dataset comparison, not all collision
scenarios. Animated-model collision, original scene query ordering, actor and
resource lifetimes, neighbor-stream handoff, link outcomes, floor effects,
hardware/GTE validation and full-game comparisons still require work. The
chunk prototype reports contacts to the future scene/link layer and introduces
no substitute wall push-out. Full original feature acceptance remains 0/40
provisional groups.
