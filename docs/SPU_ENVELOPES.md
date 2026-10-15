# SPU envelopes and volume controls

The Rust audio core now advances per-voice attack, decay, sustain and release,
plus direct and swept volume for voice and main-output gains. These components
retain the current level, divider and phase. Register changes preserve state
unless an accepted restart/release action explicitly resets it.

Direct volume writes become effective on a sample clock. Sweep controls retain
signed phase, exponential increase/decrease, saturation and stopped rates. The
envelope preserves literal counter wrapping, manual level writes and phase
transition boundaries. The
[hardware reference](https://psx-spx.consoledev.net/soundprocessingunitspu/#spu-volume-and-adsr-generator)
provides the register layout and rate behavior.

```sh
cargo test -p lsd-assets --test spu_envelopes --locked
cargo test -p lsd-assets --test spu_samples --locked
```

## Comparisons

A private test tool compiles eight unchanged envelope/sweep functions from the
pinned [Beetle PSX source](https://github.com/libretro/beetle-psx-libretro/blob/e57860d82faa953dc0fc10f201fd583739bc2f7d/mednafen/psx/spu.c).
It supplies initial states, explicit clocks and register/key actions. That GPL
comparison source remains separate from the MIT Rust implementation.

| Supplied software comparison | Coverage |
| --- | ---: |
| Envelope control/rate configurations | 1,088 |
| Initial envelope states / clock updates | 32,640 / 130,560 |
| Every volume-control word with supplied levels/counters | 65,536 |
| Initial volume states / clock updates | 983,040 / 1,966,080 |
| Envelope action traces / volume action traces, 256 clocks each | 15 / 22 |
| Original tone bindings / distinct ADSR pairs | 351 / 75 |

Each original parameter pair is also compared for 66,150 explicit clocks, with
a supplied release after 44,100. These are controlled component lifecycles,
not observed natural game note lengths. The original parameter bindings and
detailed observations remain private. Portable tests use owned inputs.
See [spu-envelope-checks.json](spu-envelope-checks.json) for identified evidence.

## Correction to the earlier pitch comparison

The earlier sample checkpoint's private pitch wrapper altered the pinned
emulator's playback calculation: it inserted a 16-bit intermediate mask and
saturated to `0x4000`. The actual source playback path retains a wider result
and saturates to `0x3fff`. The Rust function and comparison now follow that
unchanged source block. All 655,360 pitch cases were regenerated; 453,328
previous outputs change. The current fixture replaces the earlier pitch hash.

The stored ADPCM and Gaussian cases are unchanged, including all 255 original
stored sample PCM comparisons. The earlier dated
[spu-sample-checks.json](spu-sample-checks.json) is superseded for its pitch
comparison claim by the new check file. The old fixture remains recoverable in
the [prior commit](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/tree/48aac5e818946ec473c6bdf3ab0d5d06f762b0cd).

The pinned FPGA source also limits the step to `0x3fff`, but its high-register
modulation expression differs from the software reference. Hardware agreement
for those edge cases remains open; software agreement is not hardware proof.

## Remaining device integration

The [clocked voice engine](SPU_ENGINE.md) now connects these components with
prefetch, key-on/off delay and ordering, looping, capture, noise and mix inputs.
The caller still supplies sample clocks; actual transfer/callback timing,
complete mixing/reverb, sequence dispatch and output remain open. The default Bevy scene still
queues audio requests, and no complete audio group or audible scene comparison
is accepted by these checks.
