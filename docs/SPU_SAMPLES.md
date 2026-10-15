# SPU sample decoding, interpolation and pitch arithmetic

`lsd-core` now decodes the five documented SPU ADPCM predictors, preserving
signed samples, separate arithmetic rounding of the predictor terms, saturation
and history across blocks. Block flags remain available to the future device
layer. This is decoding of stored blocks, not an implementation of voice-loop
timing, envelopes or the complete sound device.

The core also implements Gaussian interpolation and pitch-register processing,
including modulation from the preceding voice and the signed high-register
behavior. Interpolation accumulates the fixed-point products before shifting.
The numeric hardware table comes from the MIT-licensed psxe source; attribution
and its license are retained in [NOTICE.md](../NOTICE.md).

```sh
cargo test -p lsd-assets --test spu_samples --locked
cargo run -p lsd-assets --bin lsd-audio-check -- /path/to/extracted/files
```

The inventory command now decodes each stored sample and reports block counts
and PCM hashes alongside the existing import metadata. It does not write or
bundle original audio. The original-sample comparison test additionally needs
the local game files and private reference hashes; its environment variables
are `LSD_GAME_FILES` and `LSD_SPU_SAMPLE_REFERENCE`.

## Comparison evidence

The private comparison tool compiles the unchanged decoder body from pinned
[Beetle PSX source](https://github.com/libretro/beetle-psx-libretro/blob/e57860d82faa953dc0fc10f201fd583739bc2f7d/mednafen/psx/spu.c).
Its software FIR and pitch arithmetic are also comparison references. That
GPL test source remains separate from the port implementation. No emulator
decoder source is included in the MIT Rust code.

The pitch comparison was corrected after this checkpoint; the current fixture
uses the unchanged playback calculation. See
[the correction and envelope work](SPU_ENVELOPES.md#correction-to-the-earlier-pitch-comparison)
for the changed high-register behavior and the superseded pitch evidence.

| Comparison | Cases |
| --- | ---: |
| Owned decoder inputs, including predictor boundaries and block continuity | 1,292 |
| Gaussian inputs across all 256 fractions | 16,384 |
| Every pitch register, unmodulated and with nine preceding-voice outputs | 655,360 |
| Original sound banks / stored samples | 9 / 255 |
| Original stored blocks / decoded sample values | 126,210 / 3,533,880 |

All original stored sample PCM hashes and final predictor histories match the
pinned software reference. Original inputs and detailed per-sample observations
stay private; portable tests use owned data. See
[spu-sample-checks.json](spu-sample-checks.json) for the identified evidence.

## Limits and next work

The [hardware reference](https://psx-spx.consoledev.net/soundprocessingunitspu/)
describes the SPU's block layout, flags, interpolation and pitch controls.
References disagree on reserved shift/filter behavior; those encodings return
explicit bounded errors pending stronger evidence. None occurs in the 255
supplied original stored samples. Invalid lengths are also rejected.

The original executable delegates synthesis to SPU hardware. Comparing a
separate emulator decoder therefore does not prove original runtime or hardware
parity. The [clocked voice engine](SPU_ENGINE.md) now connects decoder prefetch,
loop/end handling, noise, envelopes and gain to explicit sample clocks. Actual
SDK callback/transfer timing and live output integration remain required. The
[PCM output stage](SPU_OUTPUT.md) now processes reverb and main output. The running
Bevy scene still queues audio requests. No complete audio group, audible scene
comparison or whole-game parity is accepted by these checks.
