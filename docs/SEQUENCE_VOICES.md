# Music voices and sequence integration

The Rust music path now connects imported SEQ events to the original SDK's
music note and controller behavior. It uses the shared 24-voice driver and
SPU implementation. Sound effects retain their separate original entry point.

## Recovered behavior

- A note selects every tone whose inclusive key range matches, allocating each
  through the shared priority/age/envelope voice pool. The literal packed SDK
  result is preserved; zero is not a reliable note handle.
- Music note-on and note-off execute while the global callback lock is held.
  They do not use the sound-effect entry's lock gate.
- Note-off matches score/track, bank, program, and key. Released records retain
  the original repeated-release and later controller behavior.
- Program IDs retain all 128 sparse MIDI entries. This differs from the sound
  object's logical program-count table.
- Channel volume, pan, and expression use the actual B0 handlers. Expression
  changes shared bank program volume. The SDK volume update's original tone
  and packed-program indexing quirks are preserved.
- Pitch bend uses the second data byte, asymmetric divisors 63/64, and the
  original fine-pitch carry. Note-on clamps its correction fraction differently.
- Low notes can read before the named pitch table. A local lookup window
  preserves signed remainder behavior and nearby reference data. These bytes
  are imported locally rather than distributed. Later mutations of neighboring
  SDK data still require lifecycle analysis.
- Channel defaults retain program IDs 0–15, pan 64, volume 127, and last
  velocity 127. A zero left sequence volume suppresses note handling as in the SDK.

`SeqMusicTrack` combines the verified clock and channel handlers for the
infinite-repeat profile used by the original BGM play request. Its caller owns
bank placement, driver flush, callback locking, SPU clock, and event budget.
Note events read the following delta before dispatch; controller, program,
and bend events read it afterwards. Handler faults retain partial state.

## Validation

| Comparison | Scope |
| --- | --- |
| 138 owned note scenarios | Sparse programs, sixteen overlapping tones, priority exhaustion/reclaim, lock, mono, ADSR offset, and repeated release |
| 932 original-bank note scenarios | All nine imported banks, key-range/center/boundary requests, complete driver state, and release state |
| 90 owned controller operations | Sparse/overlapping tones, released records, another score/track, shared expression, volume, pan, and bend |
| 1,836 original-bank controller operations | All 102 active programs across nine banks |
| 9,632 owned pitch cases | Both original pitch routines, signed remainders, correction clamp, fine carry, and narrow/wrapping arithmetic |
| 201 owned integrated callbacks | Nine sequence/bank/rate scenarios at supplied 50, 60, and 120 Hz |
| 37,859 original integrated callbacks | All 69 positive-tempo sequences through their first repeat, 39,741 decoder visits |

Integrated comparisons execute the real `SsSeqCalledTbyT`, original decoder,
handlers, and software driver. Each callback checks clock fields, full driver
state, channel state, shared program volumes, and ordered SPU register writes.
All original sequences receive the same supplied AMBIENT bank and zero device
envelopes in this campaign. Those inputs do not reproduce natural bank choice
or a running original SPU.

The seventieth imported sequence has a zero tempo that faults in original SDK
initialization. It remains preserved with that explicit fault rather than
receiving a fabricated tempo.

Original audio function outputs are not stubbed. Public fixtures use owned
banks, sequences, and numeric state. Detailed original observations and assets
remain local. [sequence-voice-checks.json](sequence-voice-checks.json) records
identity, fixture hashes, counts, and limits. The corresponding integration
tests are `sequence_notes`, `sequence_controls`, `music_track`, and `sequence_pitch`.

## Controlled music render

```sh
cargo run -p lsd-assets --bin lsd-music --locked -- "/path/to/SLPS_015.56" "/path/to/AMBIENT.VH" "/path/to/BGA.SEQ" 12 "/path/to/new-music.wav" "/path/to/new-trace.json"
```

The diagnostic renders raw 44.1 kHz stereo through the shared SDK/SPU path.
It supplies one bank at byte 4096, one infinite-repeat track at volume 52,
735-sample callbacks, and no reverb. Driver flushing precedes sequence dispatch.
The optional trace records its register schedule and input/output hashes.
Existing output files are preserved.

A controlled twelve-second STG00/BGA render with AMBIENT produced 529,200
stereo frames, including 528,292 nonzero frames. Independently replaying its
register schedule through the pinned native software DSP produced identical
raw PCM. That validates the supplied schedule and synthesis path, not a natural
game scene or original bank selection. The audio and detailed trace stay local.

This is controlled synthesis. Live dream music, original bank allocation and
loading, pause/replay, stopping, fades, multiple-score orchestration, hardware
cadence, neighboring SDK data lifetimes, and natural original audio scene
comparisons remain open. No whole audio or gameplay group is accepted here.
