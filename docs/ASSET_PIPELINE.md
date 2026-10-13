# Original map and texture pipeline

The Rust asset library now decodes TIM images, native TIX texture-bank lists,
unmapped TMD polygon objects and LBD map records. The public tests use synthetic
data; the local corpus check uses the Japanese-labelled reference extraction.

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

The decoder consumes 551 source files: 418 LBD maps, 56 TIX containers,
75 standalone TIMs and two standalone TMDs. This includes 520 TIM images,
13,037 TMD objects, 181,561 vertices, 125,304 normals and 111,177 polygon packets
in 20 observed variants. The maps contain 95,842 present records and 270
additional chain records.

An independent reader confirms source hashes, descriptor and packet counts,
tile counts, and all 56 reconstructed raw VRAM hashes. It follows native
image-then-palette transfer order. No original assets or raw polygons are
published in the repository.

## Format and source evidence

TMD/TIM structure was checked against
[Sony's File Formats reference](https://psx.arthus.net/sdk/Psy-Q/DOCS/Devrefs/Filefrmt.pdf)
and the actual files. LBD tile positions, headings and chains are grounded in
the companion project's `class_1C92C_unk29` and renderer consumers. TIX bank
and TIM-list ordering are grounded in their original loaders.

Readers reject truncated extents, invalid indices, descriptor overlaps, mapped
RAM pointers and cyclic chains. TIM image data follows the native rectangle
extent even when its ignored image length word is short; CLUT extents remain
bounded. Texture samples retain the original 16-bit color and STP flag.

## Remaining gates

The 98 streamed model-library regions remain bounded opaque data until MOM/MOS
and MML animation decoding is added. Audio, movies and save data are also open.
The viewer currently uses Bevy's standard material pipeline. Native lighting,
semi-transparency modes, palette gradients/fog, affine texture interpolation,
draw sorting and PS1 projection/timing remain rendering work. A successful
asset render does not establish original visual or gameplay parity.
