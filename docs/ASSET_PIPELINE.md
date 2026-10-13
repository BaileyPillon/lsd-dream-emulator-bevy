# Original map, texture and animation pipeline

The Rust asset library now decodes TIM images, native TIX texture-bank lists,
unmapped TMD polygon objects, LBD map records and MOM/MOS/MML model libraries.
The public tests use synthetic data; the local corpus check uses the
Japanese-labelled reference extraction.

```sh
cargo run -p lsd-assets --bin lsd-asset-inventory --locked -- /path/to/extracted/files
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A
```

`lsd-stage` loads original geometry, tile centers, signed heights, quarter-turn
headings, vertex colors and texture-page/CLUT bindings. The free camera uses
WASD, left/right arrows and Q/E. These controls serve asset inspection; original
movement, collision, NPCs and gameplay state still require implementation.

For an input check without opening a window, append `--inspect`. For an
offscreen 320×240 rendering check, append `--capture /path/to/new.png`. Existing
capture files are preserved. The rendered output is original-derived content
and should stay in private storage.

## Verified corpus

The decoder consumes 553 source files: 418 LBD maps, 56 TIX containers,
75 standalone TIMs, two standalone TMDs and two standalone MOMs. Including
embedded animated geometry, this contains 520 TIM images, 13,627 TMD objects,
191,582 vertices, 134,340 normals and 119,458 polygon packets in 24 observed
variants. The maps contain 95,842 present records and 270 additional chain
records.

An independent reader confirms source hashes, descriptor and packet counts,
tile counts, and all 56 reconstructed raw VRAM hashes. It follows native
image-then-palette transfer order. No original assets or raw polygons are
published in the repository. The earlier texture/map checkpoint remains in
[asset-pipeline-checks.json](asset-pipeline-checks.json).

The 98 embedded MML regions contain 109 model entries; the two standalone MOMs
bring the total to 111 entries with 100 distinct model hashes. Their 122
animation entries contain 4,260 frames and 26,797 commands. The decoder retains
animation selection order, aliased offsets, duplicate part declarations,
frame headers and payload words. It checks each command's extent and the
minimum payload length read by the original interpreter.

An independent source-byte reader confirms model/frame/command counts and all
24 geometry packet variants. These checks establish asset decoding; runtime
animation and complete NPC behavior remain open. See
[animation-import-checks.json](animation-import-checks.json).

## Format and source evidence

TMD/TIM structure was checked against
[Sony's File Formats reference](https://psx.arthus.net/sdk/Psy-Q/DOCS/Devrefs/Filefrmt.pdf)
and the actual files. LBD tile positions, headings and chains are grounded in
the companion project's `class_1C92C_unk29` and renderer consumers. TIX bank
and TIM-list ordering are grounded in their original loaders.

MOM/MOS offsets and model-part enumeration follow `func_80044858`,
`func_800452FC` and `func_800453DC` in the preserved C reference. Animation
headers and payload widths follow `func_80066214`, `func_800662BC` and
`func_80066340`. Unknown child targets follow the native skip behavior;
commands for existing children validate their primitive and parent references.

Readers reject truncated extents, invalid indices, descriptor overlaps, mapped
RAM pointers and cyclic chains. TIM image data follows the native rectangle
extent even when its ignored image length word is short; CLUT extents remain
bounded. Texture samples retain the original 16-bit color and STP flag.

## Remaining gates

MOM/MOS/MML streams are decoded; their integer transform updates, primitive
attachments, parent changes and tick order still require Rust execution and
native comparisons. Audio, movies and save data are also open.
The viewer currently uses Bevy's standard material pipeline. Native lighting,
semi-transparency modes, palette gradients/fog, affine texture interpolation,
draw sorting and PS1 projection/timing remain rendering work. A successful
asset render does not establish original visual or gameplay parity.
