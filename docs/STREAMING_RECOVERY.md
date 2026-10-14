# Spatial stream recovery

The viewer now uses the recovered seven-slot neighbor and rebinding logic.
It keeps resident maps while their direction labels change, positions requested
neighbors in the staggered grid, and releases geometry before replacing a slot.
Maps in nested disc directories resolve through the original path catalogue.

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 3 136 A --walk-stream
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/files 3 136 A --walk-stream-capture /path/to/new.png 1800
```

W/S, A/D, Q/E, Shift and R/F retain the movement prototype's controls. The streamed
mode works with all fourteen stage descriptors. Spatial stages rebind when the
player moves into a resident neighbor; layered stages retain their imported
layers. The scene's mesh cache keeps resident maps and drops released maps.

## Recovered protocol

`StreamGridTables` reads direction masks, neighbor steps, placement deltas and
reuse selections from the user's canonical executable. The port does not
bundle those original tables. `StreamProtocol` implements the original request
and completion state updates and emits ordered scene notifications, load,
cancel, bind and release events. It keeps the original halfword counter wrapping
and the path where an idle failed load clears a slot flag without decrementing
the outstanding count. Completion code 2 has distinct behavior from other codes.

The host adapter decodes local files and provides completion on a following
host step. It builds collision geometry and Bevy meshes when a bind event
arrives. This completion schedule is a development setting. Original CD timing,
scene notifications, actor/MML lifetime callbacks, original input suspension
and full session scheduling still need implementation. Body collision still
uses the recovered surrounding rectangles and attribute gate; see
[body dispatch](BODY_DISPATCH_RECOVERY.md). Complete actor callback behavior
remains open.

## Evidence and limits

The reference observations cover 309 native boundary masks, 2,163 native
neighbor/path lookups, 308 slot rebindings and 32 protocol sequences containing
128 operations. Rebinding captures the map-request submission boundary. Protocol
runs execute the original request/count/completion routines in supplied graphs;
notification, I/O and bind/release callbacks are captured, and I/O field changes
are supplied stimuli. These checks do not execute the complete CD/actor graph.

The public mask and protocol fixtures use numeric inputs. Original direction
tables and the detailed table-driven observations stay local. To compare those:

```sh
LSD_GAME_FILES=/absolute/path/to/files LSD_NATIVE_STREAM_CASES=/absolute/path/to/observations.json cargo test -p lsd-assets --test native_stream_grid --locked -- --ignored
LSD_GAME_FILES=/absolute/path/to/files cargo test -p lsd-assets --test native_streamed_stage --locked -- --ignored
```

The original-input integration check initializes all fourteen stage descriptors,
crosses four spatial map boundaries,
confirms resident-neighbor retention and verifies that requested maps bind and
clear pending counts. Bevy captures also exercise the streamed scene. These are
bounded integration checks, not full original gameplay acceptance. See
[stream-recovery-checks.json](stream-recovery-checks.json) for hashes and exact
scope. Original feature acceptance remains 0/40 provisional groups.
