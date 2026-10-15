# Original sequence clock and event timing

The port now implements the SDK's sequence timing calculation and callback
advance rules. An imported `SeqEventClock` visits channel events at the original
decoder boundaries and retains repeat/stop timing. It does not yet execute the
music channel handlers or synthesize music through the shared SPU.

The clock uses tenths of MIDI ticks. Header initialization rounds BPM to the
nearest integer with ties down; the embedded tempo handler truncates BPM.
Fast playback subtracts a signed halfword step from a wrapping word counter.
Slow playback has a separate countdown. Zero-delta events are visited during
the same callback, and a tempo event can change the step while dispatch runs.
The code retains signed widths and overflow behavior rather than substituting
a floating-point MIDI clock.

Loop limit zero repeats indefinitely. A repeat resumes after the first encoded
delta, so the initial wait is not repeated. Positive limits and signed loop
counter wrap retain the original comparison. Finite playback ends at the
original decoder cursor. These are event-clock rules; the SDK's subsequent
voice release, controller and transport effects remain separate work.

```sh
cargo test -p lsd-core --test sequence_clock --locked
cargo test -p lsd-assets --test sequence_clock --locked
cargo run -p lsd-assets --bin lsd-sequence --locked -- /path/BGA.SEQ 718
```

The diagnostic uses a supplied 60 Hz SDK callback rate and reports event counts,
repeat state, cursor and clock fields. It produces no audio. The optional final
argument is a signed 16-bit loop limit; zero is the default. The underlying API
also accepts other supplied callback rates.

## Comparisons against the original executable

| Check | Cases or observations |
| --- | ---: |
| Initialization timing, widths and division faults | 294 |
| Advance branches, return values and decoder-entry clock state | 350 |
| Embedded tempo timing and saved-step behavior | 216 |
| Owned running-status and repeat/stop scenarios | 45 |
| Owned repeat/stop callback states | 2,643 |
| Original SEQ header/rate combinations | 280 |
| Playable original sequences checked through their first repeat | 69 |
| Original sequence-clock callbacks | 37,859 |
| Original decoder-entry visits | 39,741 |

The native harness executes original initialization, delta readers, clock,
decoder and event handlers without replacing function outputs. Owned program
events provide the delta inputs for clock-boundary cases. Separate owned
sequences cover running status, program/controller/pitch/note events, a nonzero
initial wait, finite/infinite repeats and loop counter wrap. Tests compare clock
fields, event-entry positions and repeat/stop state at every supplied callback.
Recordings repeat in fresh processes.

All 70 local files retain their identified bytes. The zero-tempo
`CDI/STG09/BGC.SEQ` still faults in original initialization; it is not repaired
with a default tempo. Its four supplied rate cases remain failure observations.
The other 69 files match their initial clocks at 50/60/120/240 Hz and match all
event positions through a full loop at a supplied 60 Hz.

The full-loop native trace uses an assigned ready AMBIENT bank, volume 52,
owned zeroed MMIO and direct sequence-clock calls. Original SDK handlers do
execute, but SPU sample time and envelope evolution are not advanced, and this
trace does not run the complete `SsSeqCalledTbyT` orchestration. It establishes
event-clock agreement, not original instrument choice, final audio or physical
callback timing. Original files and detailed observations remain private.

## Remaining music work

The visitor currently marks decoder entry. The Rust SDK music handler still
needs channel/program state, volume/expression/pan and pitch bend, sequence
key-on/key-off, shared voice allocation, bank selection and loading, pause,
replay, stop and fades. Wire those handlers into the shared driver at their
original points, with SDK updates preceding sequence dispatch as in the
original callback. A host MIDI sampler would not verify those requirements.

The clock API exposes the recovered embedded tempo calculation, but the
imported event parser still rejects unsupported system/meta forms. The game
corpus uses note, control, program, pitch bend and end events. Broader forms
need their own recovery. An explicit event budget bounds malformed zero-delta
streams and retains state on failure.

No complete music scene or whole-game feature group is accepted. See
[sequence-clock-checks.json](sequence-clock-checks.json) for reference identity,
hashes, validation and limits. Original behavior and optional AI tools/mods
remain separate.
