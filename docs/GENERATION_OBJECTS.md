# Original generated models, sprites and update RNG

`DreamGeneration` now constructs and updates the original four structure modes.
The Rust representation retains model indices, persistent spawn fields, local
parent/child poses, five sprite children, unsigned object ticks and the original
setter widths. The first generation tick builds objects and updates them before
visiting the two proximity sound slots.

The implementation includes the constructor's parity random result, the four
sprite scale/rotation pairs on each mode-three update, random/daily placements,
both extra-structure paths and the original count-byte/offset aliases. Mode-zero
children begin moving at unsigned tick 501, increment their local positions and
rotation, and reset their placement at the original division-derived cadence.
Year/session bootstrap and original global scheduling remain separate work.

The alternate sprite color preserves another alias: `apply_entry` writes the
scene-color pointer over `EntityParamsAlt[0]`. The alternate color therefore
uses the current style color unless the day is divisible by twenty.

## Local content and Bevy output

The asset loader reads the canonical model/texture bindings and imports
`DREAMER.TMD` and `ETC.TIM` from the user's extraction. Original CPU execution
confirms that the generation helper changes the first primitive's **CLUT** in
the two selected models. Texture pages, vertices and UVs stay intact. Sprite
rectangles and palette coordinates also come from the local executable.

The `--dream` profile now consumes the object-channel RNG and renders the imported
models and sprites at their recovered logical poses. Sprite texture lookup uses
the original byte UVs, dimensions and palette coordinates. Color modulation is
performed on the five-bit source channels, with transparent/STP texels retained
for the presentation backend. Object destruction/replacement removes their
development visuals when the scene changes.

The presentation adapter still uses Bevy transforms, camera projection and
materials. Its billboard projection, transparency/draw order, fixed-point/GTE
precision and raster output require original framebuffer comparisons. The
development viewer does not yet reproduce the complete PS1 renderer.

## Verification

Owned-content comparisons execute the original allocator, scene-node/model/sprite
constructors, parent/child setup, update methods and teardown. The model accessor
is a supplied resource boundary pointing to an empty owned TMD. No original
function's output is stubbed. These successful-allocation graphs do not cover
all allocation failures or original model-render lifetimes.

- 108 constructor cases, 864 update states and 108 native teardown cases.
- 432 generation builds and 1,296 subsequent object-update passes.
- 168 private original-content builds and 504 update passes across 14 stages,
  six sampled days and two seeds.
- Original TMD mapping/class construction and palette mutation on the local
  three-object model; the two generation bindings change only their CLUT bytes.

The imported first-day start/expiry/next-day handoff, all 365-day scene-choice
comparisons and 17 supplied permanent-link destinations remain regression checks.
Desktop captures exercise the prepared object channel at 32 and 4,000 supplied
idle host ticks. Captures establish development output, not whole-session parity.

Gradient and environmental effects, border composition, original renderer/global
ordering, natural NPC encounters, audio/SPU output, menus, media and persistence
remain required. No complete original feature group is accepted by this change;
the whole-game tracker remains 0/40 provisional groups. Optional AI tools/mods
remain separate, and the matching C checkout stays paused and unchanged.

See [generation-object-checks.json](generation-object-checks.json) for exact
fixture, function, source and validation hashes.
