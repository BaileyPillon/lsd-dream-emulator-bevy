# Original sound objects, volume updates and register flushing

The software audio path now includes the original sound-object pitch selector,
packed-note lookup, key-on followed by auto-volume, per-voice volume updates,
stop-all and the SDK driver's envelope-read/history/register-flush loop.
`SoftwareSound` connects those calls to the existing `GenerationSound` trait.
It produces software state and device commands. The interactive dream profile
now uses the [shared live backend](LIVE_AUDIO.md); silent checks retain the
queued backend. Original scene audio acceptance remains unfinished.

```sh
cargo test -p lsd-assets --test audio_clock --locked
```

## Correction to earlier command labels

The mode-two reclaim call at `0x800375E8` is `SpuSetNoiseVoice(0, 0xffffff)`.
It disables noise mode for the 24 voices. Earlier voice/note check snapshots
and their `immediate_key_off` API name described it incorrectly as key-off.
The recorded arguments and software states were valid; their interpretation
was wrong. The current API is `disable_noise`, and current public fixtures use
`noise_calls`. Historical check files remain as dated snapshots and are
superseded on this point by [audio-clock-checks.json](audio-clock-checks.json).

New original-routine tests capture the actual writes to noise registers
`0x1F801D94/0x1F801D96` and prove that key-off registers remain unchanged.
The separate key-off registers are `0x1F801D8C/0x1F801D8E`.
These addresses also match the [SPU register reference](https://psx-spx.consoledev.net/soundprocessingunitspu/#spu-noise-generator).
The correction was made before connecting these commands to a live SPU.

## Recovered behavior

Auto-volume retains signed halfword arguments, equal-value no-ops, countdown
wrapping, partial state before a division fault, the literal step direction
and clamping. For example, the supplied original call `(voice, 100, 0, 2)`
reaches zero on its first volume update. It is not replaced with a two-update
interpolation. Volume-update panning/scaling also differs from initial note
setup, and the Rust code preserves both paths.

The full SDK update consumes supplied device envelope levels, advances its
sixteen-entry history ring, applies its original fifteen-entry reclaim scan,
updates active volumes and flushes dirty voice registers in original order.
It then emits key-off, key-on and reverb-routing writes, clearing the pending
key masks. The shared volume context is retained across voices, matching the
original globals. Tests include simultaneous voices with different tones.

The sound-object adapter preserves the `12 * selector - 24` pitch offset,
including the default selector zero. It supplies the tone correction to key-on
and invokes original-style auto-volume after a successful allocation.
Single-voice stop and stop-all retain their distinct record/mask/device effects.

The sound object's program table uses logical slots up to `program_count`.
An inactive program can alias the next copied tone block; the ambient bank's
program-seven case is covered. Inactive attributes are preserved in the VAB
import. Requests beyond known/copied tone data return an explicit bounded
error, without claiming original out-of-range or uninitialized-memory parity.

## Evidence

| Original-code comparison | Cases |
| --- | ---: |
| Noise-register operations, with unrelated key-off registers retained | 24 |
| Owned auto-volume boundary cases and subsequent updates | 10 |
| Complete SDK update traces, 18 calls each | 12 |
| Stop-all counts and ordered direct register writes | 5 |
| Owned sound-object calls through the compatibility adapter | 6 |
| Original-tone auto-volume cases and subsequent updates | 702 |
| Original sound-object calls, including inactive-program aliases | 696 |

Comparisons retain every voice-record byte, cached register, dirty flag,
automation mask, key/reverb mask and the shared volume context. Complete update
tests compare all emitted register writes in order. Native recordings are
repeated independently before publication. Previous note, pitch, allocation
and import comparisons remain regression checks.

## Remaining work and scope

The original routines execute with owned register storage, supplied device
envelope reads, assigned bank addresses and ready flags. Sound-object parameter
tables are prepared inputs matching the copied logical slots. These checks do
not execute sample DMA, SPU synthesis, real device clocks or natural startup.

The current update API explicitly rejects active SDK pan automation, whose
separate implementation remains open. The [clocked voice engine](SPU_ENGINE.md)
now executes ADPCM/envelope/interpolation, noise and mix inputs. The SDK update
can use `tick_engine`, and `dispatch_audio_commands` connects `SoftwareSound`'s
immediate commands. Actual bank allocation/transfer timing, interrupt/callback
cadence, sequence dispatch and natural original-scene audio still require
implementation and original runtime evidence. The [PCM output stage](SPU_OUTPUT.md)
now processes reverb and main output with controlled waveform comparisons.

The [shared live backend](LIVE_AUDIO.md) now handles host output. Fresh
[link/journal sound comparisons](LINK_AUDIO.md) also recover the SDK's single
initialization update and connect those callbacks to the same driver. Periodic
callback timing and natural original audio-scene acceptance remain open.

No complete audio group or whole-game parity is accepted. Original behavior
has no AI model dependency; optional analysis/modding tools remain separate.
