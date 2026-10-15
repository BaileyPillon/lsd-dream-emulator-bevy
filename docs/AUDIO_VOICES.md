# Original software voice selection, release and pitch

The Rust core now implements the original SDK's voice-pool selection, single
voice release and note-to-pitch conversion. The asset library imports the
original 192-entry pitch table and binds it to each imported VAB tone's center
note and correction. These are components of the pending audio driver; the
live scene still uses the queued backend that returns -1.

```sh
cargo test -p lsd-core --test native_voice_pool --locked
cargo test -p lsd-assets --test spu_pitch --locked
```

## Recovered behavior

The pool models 24 physical voice slots and the original 52-byte software
records. Construction matches original software startup from zero-initialized
driver data, including the byte-width requested count and its 24-voice cap.
It does not replace hardware initialization or reinitialization of a running
driver.

Selection preserves first-free order, stored signed priorities, level and age
tie rules, and the different signed/unsigned age reads. A successful selection
increments all enabled-slot ages with halfword wrapping and resets the chosen
age. Mode-two reclamation disables noise mode for all 24 voices. The
allocator exposes that request to its caller. Selection alone does not mark a
note playing or establish playback success.

The raw SDK result is retained. Normal failure equals the pool capacity;
some supplied unusual priority/level/age combinations retain sentinel 99.
Callers must check that the result is below capacity before accessing a slot.

Release preserves the global busy-state gate and halfword handle conversion.
It clears the selected record fields, accumulates its key-off bit and removes
all already-pending key-off bits from key-on state. It retains the old level
until the device/driver update changes it. A released voice is not simply
declared free before that update.

Pitch preserves tone-center/correction arithmetic, signed halfword truncation,
integer octave/fraction lookup and MIPS variable-shift widths. The original
table remains local. Out-of-table inputs return a bounded error; that error
does not claim to reproduce the original's out-of-range memory reads.

## Evidence

| Original SDK execution comparison | Cases |
| --- | ---: |
| Software startup counts | 10 |
| Voice allocations, including an ordered supplied-state sequence | 640 |
| Release handles, masks and busy states | 48 |
| Pitch conversion with an owned table | 3,159 |
| Pitch conversion with the original table | 3,159 |
| Imported original tone bindings and pitch | 5,265 |

The pool checks compare every byte of all 24 software records, current voice,
lock and key masks. They also compare all 84 noise-mode disable calls.
Original startup executes unchanged instructions with zero-initialized owned
register storage. Selection/release receive identified owned software states;
the ordered sequence supplies a playing-state change between allocations.

Pitch checks use the original function and either owned table data or the
unchanged original table. The coupled import check executes original VAB
binding and pitch conversion for all 351 tones from all nine sound banks,
then compares Rust through the imported tone and table APIs. Original tables,
tone properties, register observations and SDK assembly remain private.
Repeated original recordings must be identical before publication.

A recording error in the first ordered-allocation fixture mutated an expected
post-allocation state while preparing the next supplied playing state. That
fixture and failed comparison are retained privately; the producer now copies
the next input before modifying it. The corrected recordings are the accepted
comparison inputs.

## Remaining audio work

[The software clock and sound-object adapter](AUDIO_CLOCK.md) now implement
auto-volume, stop-all and register flushing. The same checkpoint corrects the
earlier reclaim-command label to noise-mode disable; historical check labels
should be read with that correction.

[Normal sound-effect note setup](AUDIO_NOTES.md) now binds imported tones and
prepares original volume/pan, sample/envelope registers and key/reverb masks.
It remains separate from the live queued backend and SPU synthesis.

The live backend still needs device envelope advancement, command execution
and clock integration. Sequence dispatch/controllers and timing,
ADPCM decoding, SPU interpolation, reverb, mixing and output scheduling also
remain required. Original complete software startup followed by key-on returns
voice zero in the reference harness; live device/output integration remains open.

Owned register storage does not emulate SPU reads, device timing, DMA or sound
output. No complete audio feature group or whole-game parity is accepted.
Original NPC behavior remains independent of optional AI tooling and mods.
