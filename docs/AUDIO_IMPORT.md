# Original sound-bank and sequence import

The asset library now imports the Japanese game's nine version-seven VH/VB
sound banks and 70 version-one SEQ files. It retains 102 active programs,
351 tones, 255 sample regions and 38,976 sequence events including end markers.
This is input recovery for the audio runtime; the live game still queues sound
requests and does not yet reproduce original SPU playback.

```sh
cargo run -p lsd-assets --bin lsd-audio-check --locked -- /path/to/extracted/files
cargo test -p lsd-assets --test audio_import --locked
```

The command emits source hashes, counts and timing issues as JSON. It does not
export original samples or sequence events. The public fixture contains two
owned sparse sound banks and two owned sequences. Original-content tests need
`LSD_GAME_FILES` and `LSD_NATIVE_AUDIO_IMPORT`, the local SDK observation file.

## Retained format details

Program IDs stay sparse: the original ambient bank has an inactive program
between active programs. Tone blocks follow active-program order, with sixteen
reserved tone slots per active program. The importer preserves tone bytes,
key limits, center note, pitch correction, envelope words and sample references.
Waveform numbers start at one; length-table entry zero describes the body
prefix. Sizes use eight-byte units. Owned examples exercise a nonzero prefix
and active program 127, as well as a gap between program IDs.

Sequences preserve big-endian header fields, original event order, running
channel status, one/two-byte message widths and one-to-four-byte delta values.
Zero-velocity note-on remains a note-on event in the imported data. The SDK
delta reader multiplies by ten and accumulates with 32-bit wrapping; the
importer exposes that value alongside an unscaled 64-bit tick position.
Unsupported system/meta events produce a decoding error rather than silently
discarding data. All events observed in the local game corpus are supported.

The source format reference is Sony's [Run-Time Library Overview, chapter 15,
pages 15-3 and 15-8–15-9](https://psx.arthus.net/sdk/Psy-Q/DOCS/Devrefs/Libovr.pdf).
The game's own SDK instructions provide the local execution evidence below.

## Original-code comparisons

Original `SsVabFakeHead`, `SsUtGetVabHdr`, `SsUtGetProgAtr` and `SsUtGetVagAtr`
execute against each owned and original bank. The tests compare every returned
defined attribute byte. Sample regions have separate source-byte lengths and
hashes. The fake-header call receives a supplied SPU base address; the bank's
post-transfer ready state is also supplied. No DMA or sample playback is
executed by these checks.

Original `_SsInitSoundSeq` runs with a supplied sequence slot and 60 Hz SDK
clock. It successfully initializes 69 original sequences. `CDI/STG09/BGC.SEQ`
contains zero tempo, and original initialization raises an exception while
handling division by zero. The importer retains this record and returns no
initial BPM. The command reports the issue; it does not substitute a tempo.
Whether the original game reaches this selection still needs runtime evidence.

Original `_SsReadDeltaValue` also reads every identified delta in all 70
sequences. Rust matches the returned values, consumed byte extents and wrapping
accumulators. Channel-event classification and tick accumulation are checked
against a separate reader of the source bytes; original event dispatch is
outside this comparison. Repeated recordings must reproduce all observations
before publication. Raw original attributes, sample bytes and sequence-event
observations remain private.

## Work still required

[Original voice selection, release and pitch](AUDIO_VOICES.md) now have Rust
implementations and original SDK comparisons, including all imported tone
bindings. They are components for the pending playback backend.

The imported data is not connected to a physical-voice audio implementation.
Original voice allocation/priority, key-on pitch, envelopes, auto-volume,
controller behavior, sequence scheduling, loops, interpolation, reverb,
mixing and output cadence need recovery and original register/waveform checks.
Original CD audio and movie sound remain separate content/runtime work.

The port's existing queued sound backend returns -1; the native NPC tests use
supplied external handles. Neither accepts original physical voice behavior.
Audio import does not accept a complete audio feature group or whole-game
parity. Original gameplay stays independent of optional AI tools and mods.
