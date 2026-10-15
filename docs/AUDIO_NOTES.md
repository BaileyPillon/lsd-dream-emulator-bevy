# Original sound-effect note setup

The Rust audio driver now binds imported tones to assigned sound banks and SPU
sample addresses, selects a physical voice, and prepares the original note's
software record, register cache, dirty flags and key/reverb masks. This covers
the normal sound-effect key-on path used by all nine supplied game banks.
The live scene still queues requests; audible playback is not connected.

```sh
cargo test -p lsd-assets --test note_setup --locked
```

## Retained behavior

`Vab::prepare_voice_note` keeps logical program IDs separate from packed
tone-block indices. It derives the sample start from the assigned SPU base and
the original body region, including any prefix. It checks the complete bank's
RAM extent, bank/tone indices and sparse bindings before constructing the note.

`AudioNoteDriver::key_on` retains the SDK busy gate and allocation failure,
including mode-two immediate key-off commands. A selected record receives its
bank, logical and packed program, tone, sample, note, pitch, level and channel
fields at their original widths. Other fields are preserved. Existing voice
ages follow the recovered allocator.

Bank, request, program and tone volumes use the original integer scaling.
Tone pan, program pan and requested stereo balance apply in order, followed by
the original mono rule and nonlinear volume curve. No floating-point volume
substitute is used. Pitch uses the imported SDK table and tone corrections.

The cache receives left/right volume, pitch, sample start and ADSR words.
Release-offset addition wraps as a halfword. The driver clears the selected
voice from all sixteen automation masks and retains unrelated register words,
dirty bits and voices. It updates reverb routing and the accumulated key masks
in original order. A reverb routing bit is not a reverb audio implementation.

## Evidence

| Original complete key-on comparison | Cases |
| --- | ---: |
| Two owned sparse banks | 144 |
| Nine original banks, covering all 351 tones | 1,404 |

Comparisons execute the unchanged original `func_80030E90` and its helpers.
The Rust replay goes through imported VAB data and the prepared-note API, then
compares every byte of all 24 voice records, every cached register word, dirty
flags, sixteen automation masks, current voice, lock and key/reverb masks.
Immediate all-voice key-off calls are compared separately.

Supplied cases cover stereo endpoints and unequal levels, mono mode, fine
pitch, bank ID seven, sparse programs, full-pool failure, busy state and
mode-two reclamation. Nonzero previous cache words check retention, and owned
ADSR offsets check halfword wrap. Native recordings are repeated independently
and must match byte for byte before publication.

Original software startup runs against owned register storage. Bank loading
uses the original fake-header setup with an assigned base and supplied ready
state. Sample transfer, hardware timing, device reads and synthesis are not
executed. Original models, sample bytes, pitch values and detailed original
tone observations remain private; public cases use owned banks and pitch data.

## Remaining work

The driver's success means that original software note setup selected a voice;
it is not confirmation of an audible sound. The default queued backend still
returns -1. Original sound-object pitch offsets, auto-volume, driver ticks,
register flushing, sample transfer and a verified SPU/output path are needed
before connecting this driver to live requests. Sequence playback remains
separate work.

The special SDK sample-255 path is explicit unsupported input; none of the
nine imported banks uses it. Invalid binding, stereo division and pitch-table
bounds produce explicit port errors rather than accepting original invalid
memory access or traps. No complete audio group or whole-game parity is
accepted. Original game behavior remains independent of optional AI tools.
