# Original box packets and integer composition

The core now reproduces the original `GsSortBoxFill` packet fields: draw mode,
rectangle command/color, viewport-centered signed-short coordinates and unsigned
dimensions. Hidden boxes emit no packet. Original execution confirms that the
recursive sort visits the gradient's children before its root, and ordering-table
insertion makes the root draw first, followed by children in creation order.
The Rust gradient representation follows that observed draw order.

The rectangle compositor interprets signed eleven-bit GPU coordinates, masks
dimensions to the rectangle command's widths, clips to its screen-local drawing
area and converts colors to RGB555. It implements the four integer blend modes
with saturation. Black in this framebuffer is opaque; texture transparency is
a separate operation. Rectangles do not apply polygon dithering.

The command, coordinate and blend rules are described in the
[PSX GPU reference](https://psx-spx.consoledev.net/graphicsprocessingunitgpu/).
The [DuckStation software rasterizer](https://github.com/stenzek/duckstation/blob/master/src/core/gpu_sw_rasterizer.inl)
provides a primary implementation reference for five-bit blending. These sources
support the mathematical model; they are not observations of LSD's original
GPU output in the campaigns below.

## Running scenes

The day profile now renders background boxes into a native-size RGB555 image.
World geometry renders into a separate 320x240 scene target. A custom Bevy shader
then quantizes the scene input and applies the foreground box packets using
integer arithmetic, with explicit sRGB conversion at the image boundaries.
The environment overlay therefore follows its original blend mode rather than
ordinary Bevy alpha blending. It also retains the original short/eleven-bit
coordinate and dimension interpretation.

Generation packet state, meshes and palette updates still share the recovered
day/scene logic. The world uses Bevy projection, materials and geometry drawing.
This change does not replace that path with the complete original GTE/GPU renderer.

For reproducible development scenarios:

```sh
cargo run -p lsd-bevy --features desktop --bin lsd-stage -- \
  /path/to/extracted/files 0 0 A --dream-scenario-capture day08.png 32 8

cargo run -p lsd-bevy --features desktop --bin lsd-stage -- \
  /path/to/extracted/files 0 0 A --dream-scenario-capture day17.png 32 17
```

The supplied one-based day is 1-365 and uses newly initialized prior history and
seed one. It is a scenario input, not a recovered playthrough or save. Day eight
constructs 18 gradient boxes; day seventeen constructs the environment overlay
in the tested stage-zero start. Day seven follows the original special-day/media
result, so it does not construct the expected playable scene. Media presentation
remains unfinished. Captures save both the final PNG and a `.source.png` containing
the Bevy scene before the foreground compositor. Existing outputs are preserved.

## Verification and limits

- 144 layer packet cases compare original recursive sort, SDK packet bytes and
  ordering-table draw order with Rust.
- 72 individual box cases cover blend flags, absolute/relative coordinates and
  short dimensions; 72 corresponding hidden cases emit no original packet.
- Owned framebuffer rows compare the core with a separately authored documented
  rectangle model. They are not native GPU framebuffer observations.
- The headless `lsd-box-check` renders 128 owned boxes. Its blend plate covers
  all 4,096 back/front/mode combinations for each five-bit channel. The blend
  and opaque plates each compare 76,800 pixels with the core: zero differences.
- Three 32-tick day scenarios compare 230,400 final pixels with the captured Bevy
  input and the integer compositor model: zero differences. Day eight's gradient
  is active in state but room geometry can occlude it; the capture does not prove
  an original visible-gradient framebuffer match.
- The original first-dream/expiry checks still pass, the expiry capture clears
  generated objects/layers, and the older linked-scene capture is unchanged.
- Workspace checks: 102 passed, 0 failed, 15 ignored. Original-content layer and
  first-dream checks, all-feature clippy and desktop builds also pass locally.

The native packet harness runs the original allocation, layer constructors,
recursive sort, linked-list traversal and SDK packet builder. Viewport/ordering
table inputs are owned; coordinate matrices are prevalidated identity inputs,
so that campaign does not verify GTE transforms or original timing. No original
function output is replaced.

VRAM masking, interlacing, texture cache behavior, full model/sprite ordering,
original projection/lighting, textured triangles and original GPU framebuffers
remain separate requirements. No complete original feature group is accepted:
the tracker remains 0/40 provisional groups. NPC behavior, audio/media, UI,
persistence and complete native-session comparisons remain unfinished. Original
behavior and optional AI tools/mods stay separate, with zero AI model calls.
The matching C checkout stays paused and preserved.

See [box-rendering-checks.json](box-rendering-checks.json) for source, fixture,
native-function, capture and validation hashes. Original content stays private.
