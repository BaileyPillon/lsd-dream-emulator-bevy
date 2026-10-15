# Clocked SPU voice engine

The sound path now has 512 KiB sample memory and a clocked 24-voice engine.
`SpuVoiceEngine::clock_voices` advances one explicit sample clock and returns
the dry accumulator and saturated reverb input. Reverb and main output are
later stages; the returned values are not finished audio-device samples.

The engine connects incremental four-sample ADPCM fetches, the decoder ring,
Gaussian interpolation, ADSR, gain/sweep and pitch modulation. It retains
startup delay, accepted key-on/off ordering, loop overrides, one-shot release,
ENDX updates, sample/capture RAM, CD input, noise and IRQ latch state. Register
readback and data/DMA operations preserve their address increments and wrapping.
The [hardware reference](https://psx-spx.consoledev.net/soundprocessingunitspu/)
describes the register and memory layout; the current clock model is compared
with the identified software reference, not accepted as physical-bus timing.

`AudioNoteDriver::tick_engine` reads current engine envelope/noise values and
executes its ordered SDK writes. `dispatch_audio_commands` executes the sound
object's immediate writes/noise operations, retaining a failed command and
later commands in the queue. These APIs connect the existing `SoftwareSound`
adapter to the engine. The caller still supplies the callback schedule.

```sh
cargo test -p lsd-assets --test spu_engine --locked
```

## Comparisons

The private oracle compiles twenty unchanged functions and the unchanged
voice/capture/noise region from pinned
[Beetle PSX source](https://github.com/libretro/beetle-psx-libretro/blob/e57860d82faa953dc0fc10f201fd583739bc2f7d/mednafen/psx/spu.c).
The extracted clock region stops before reverb and main gain. Silent-voice
optimization is disabled so the comparison retains the full state walk.
The GPL reference source stays in the separate private test tool.

| Supplied software comparison | Coverage |
| --- | ---: |
| Owned voice/memory/register scenarios | 24 |
| Combined owned voice-engine clocks | 9,568 |
| Noise rates / explicit noise clocks | 64 / 2,097,152 |
| Original sample banks / SDK tone-register bindings | 9 / 351 |
| Combined original-input voice-engine clocks | 718,848 |
| State words checked at each observation | 1,716 |

Checks compare dry/reverb-input accumulator hashes, register reads, decoder
buffers/history/positions, envelope/gain/pitch state, keys, loop/end flags,
capture/IRQ/noise state and complete final sample RAM. Original note registers
come from the earlier original-executable SDK recordings, with identified
bank hashes and supplied SPU placement. Each tone uses 2,048 explicit clocks
and a supplied release at clock 1,024; this is not natural game note timing.
Portable fixtures contain owned data; original inputs/observations stay private.

The first comparison exposed a power-state detail: the reference's cached
sustain target is zero until an ADSR register write. The engine now preserves
that state and compares the cached target explicitly. Previous standalone
envelope and stored-sample comparisons remain regression checks.
See [spu-engine-checks.json](spu-engine-checks.json) for evidence identities.

## Remaining work

The [output stage](SPU_OUTPUT.md) now supplies reverb, main gain and final PCM.
Output-device integration remains open, and the Bevy scene still queues audio requests.
Real bank allocation, bus/FIFO/DMA duration, CPU IRQ delivery, original callback
timing and full music sequencing remain open. Reserved ADPCM encodings remain
explicit bounded errors rather than guessed behavior. The high-register pitch
source disagreements recorded in [SPU_ENVELOPES.md](SPU_ENVELOPES.md) also remain.

No complete audio group, audible original-scene comparison or whole-game parity
is accepted by these component checks. Original behavior has no AI dependency;
optional AI analysis/mods remain separate.
