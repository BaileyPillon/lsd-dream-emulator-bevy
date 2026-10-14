# Dream layers, palette rotation and combined frame order

The Rust core now retains the original environment box and the 18 gradient
boxes, including their dimensions, flags, depths, parent relationships and
relative offsets. Camera eye/target Y differences control color and offset
updates. Signed division, wrapping arithmetic, byte color overflow and the
original behavior that retains the last update when the step is nonpositive
are preserved. Relative coordinates use percentages of half the viewport and
truncate to signed 16-bit drawing coordinates.

The first generation frame creates both sets of layers before building generated
objects. Subsequent work follows the original order: update gradient state,
update structures, issue palette copies, then visit the two proximity sound
slots. The location timer advances once, and structures consume the same RNG
used by the scene. Stopping generation clears layers and structures; the next
scene resets the retained renderer-color override.

## VRAM and Bevy integration

The routine named `dream_generation_update_border` copies strips within VRAM.
For the canonical edition, these strips occupy palette rows rather than a
visible screen border. Its three commands save the last column, shift the strip
right and restore the saved column at the left. The asset layer supports these
bounded, unmasked copies and keeps all other words intact. Unsupported wrapped,
empty or out-of-bounds rectangles return an error before mutation.

The overlap direction follows the reverse horizontal traversal described by
the [DuckStation software rasterizer](https://github.com/stenzek/duckstation/blob/master/src/core/gpu_sw_rasterizer.inl).
Transfer masks and wrapped copies remain separate work; this is not acceptance
of the full GPU transfer engine. The
[PSX GPU reference](https://psx-spx.consoledev.net/graphicsprocessingunitgpu/)
describes transfer coordinates, dimensions and mask state.

The `--dream` inspector consumes the copy commands and updates retained texture
image bindings without reconstructing resident geometry on every palette tick.
Background gradient boxes and the foreground environment box have separate
camera passes. The later [box rendering implementation](BOX_RENDERING.md) replaces
ordinary rectangle alpha blending with integer RGB555 composition and adds
original SDK packet/draw-order comparisons. Complete ordering with models,
original GPU framebuffer output and the rest of the renderer remain open.

## Evidence

- 12 owned-content layer construction cases, 132 update states, native cleanup
  and 12 border-command cases.
- 72 combined original generation sequences with 288 frame observations. These
  compare object poses, layer state, count/timer, RNG, trigger state, returned
  selection, palette command order and sound requests together.
- A private campaign repeats 12 constructions, 132 updates and 12 command cases
  with geometry and palettes loaded from the canonical local executable.
- A 40-tick palette regression checks a complete owned VRAM image, including
  scratch-column contents and preservation of every word outside the strips.
- Workspace checks: 101 passed, 0 failed, 15 ignored. Original-content layer,
  object and first-dream checks run separately with private inputs. All-feature
  clippy and the desktop build pass locally.

Original CPU execution uses successful real allocations, constructors, placement
and update methods. Owned map/resource inputs and renderer, display and sound
virtual boundaries are supplied; no original function output is replaced. The
combined traces do not execute the original GPU or SPU. Allocation failures,
natural scene/actor lifetimes and the complete native schedule remain open.

Development captures cover the first day at 32 supplied idle ticks and after
expiry at 4,000 ticks. This first-day scene has no active gradient/environment
boxes, so these captures confirm the running integration and cleanup path only.
They do not establish visible gradient output or an original framebuffer match.
The later box-rendering campaign also captures active layers on supplied
calendar days eight and seventeen and compares the Bevy compositor with its
integer reference. It still does not accept an original GPU framebuffer match.
The legacy linked-scene capture remains byte-identical to the prior checkpoint.

Whole-game acceptance remains **0/40 provisional feature groups**. The feature
inventory is not an official exhaustive total. Original NPC behavior and optional
AI tools/mods remain separate; this change makes no AI model calls. The matching
C checkout remains paused, with its HEAD and six pending source edits preserved.

Exact source, fixture, native-function and validation hashes are recorded in
[generation-layer-checks.json](generation-layer-checks.json). Original inputs,
tables and development captures are retained privately.
