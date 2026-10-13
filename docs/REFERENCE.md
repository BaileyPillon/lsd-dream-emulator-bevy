# Reference and starting point

The original reference is `SLPS_015.56`, 505,856 bytes, with SHA-256
`282308ade239ce61b5603651dbbab0476a681b5d3618e82c0d3c3a95316d302f`.
The public numeric fixtures were generated from this executable through a
local Unicorn MIPS sandbox. Original instruction bytes remain private.

| Behavior | Original address | Native numeric observations |
| --- | --- | ---: |
| RNG output and internal state | `0x80049480` (`srand`: `0x800494A8`) | 128 |
| Animation lead-word fields | `0x8004416C` | 16 |
| Null/mixed-width signed cell bounds | `0x8004CD38` | 2,436 |

The checks also verify preserved registers and guarded scalar input/output
buffers. RNG observations cover eight seeds over sixteen consecutive calls.
Bounds observations cover 400 grid positions plus six signed/edge witnesses,
with null bounds and five bound shapes. These are constructed scalar checks.
The 2,580 observations contain 2,568 unique input cases and twelve repeated
edge witnesses, which are counted explicitly in the metadata.
PS1 timing, full streaming, asset construction, rendering, audio, encounters
and complete gameplay remain separate verification requirements.

[native-golden.json](native-golden.json) records function hashes, fixture hashes,
counts and scope. Rust integration tests compare the published numerical
observations with the new implementation.

The companion C project's last published commit at the transition was
`6ea437373d2fca48fe4d55bead34c820d73f9206`. Its local checkout also contains
six preserved, matching model-enumeration source corrections. C decompilation
work is paused, its repository is retained, and its 97.15% C-byte measure is
not imported as Rust port completion.

Engine/toolchain baseline: [Bevy v0.20.0](https://github.com/bevyengine/bevy/releases/tag/v0.20.0)
and the [tagged manifest](https://github.com/bevyengine/bevy/blob/v0.20.0/Cargo.toml),
which declares Rust 1.97.1 as its minimum supported version.
