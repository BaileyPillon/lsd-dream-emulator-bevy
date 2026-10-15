# Continuous scene audio in Bevy

The interactive `lsd-stage --dream` profile now connects generation and NPC
sound calls to one shared sound object, SDK driver and 24-voice SPU. Calls return
actual voice handles immediately, so stored handles can stop or change the
corresponding voice. Both channels share pitch offset and volume context.

The canonical executable supplies the dream session's sound-bank path through
`get_se_path`'s pointer. The original constructor requests that bank (`SE`);
the separate BGM object selects a music bank. Music sequencing is still open.
Original sound data stays in the user's local extraction. Link and journal
sound commands still use a separate queue; dispatching them at their original
callback position remains required.

One continuous, nonspatial Bevy audio source consumes finished SPU stereo PCM
at 44,100 Hz. The source converts signed 16-bit values to float by dividing by
32,768. Bevy handles host-device conversion and output. Individual notes are
never replaced with host sampler players, and host spatial audio does not alter
the original stereo mix. Only one decoder can claim a source at a time.

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A --dream
```

Silent captures retain the queued backend used by existing deterministic
checks. A trailing `--audio` on a dream capture enables audio and advances the
scene during the application rather than fast-forwarding before output starts:

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A --dream-capture /path/to/new.png 60 --audio
```

`--dream-scenario-capture` also accepts this flag. The other supplied NPC/effect
captures retain their existing silent scenario setup and reject `--audio`.

A short host-output diagnostic needs no GPU. It starts the supplied packed
note in the session's local sound bank and exits after the requested number of
samples. It fails if no sink starts, no nonzero PCM is generated, or the device
faults. The last number is a duration from one to thirty seconds.

```sh
cargo run -p lsd-bevy --bin lsd-audio-live --features audio --locked -- /path/to/extracted/files 0 2
```

Linux audio builds need the ALSA development package (`libasound2-dev` on
Debian/Ubuntu). Playback also needs a working default output device. An attached
Bevy sink and consumed nonzero source samples establish host-output operation;
they do not prove that a person heard a physical speaker.

## Current timing and initialization limits

The prototype still supplies bank ID zero, bank placement at 4,096 bytes,
disabled reverb, master-volume registers 15,480, and one SDK callback per
735 samples (60 Hz), including a callback before the first sample. Sample
production is driven by host audio consumption. Scene updates retain the
inspector's existing 15 Hz schedule. These values make a working integration;
they do not establish original boot, CD/DMA allocation, IRQ, VBlank or bus timing.
Host buffering and resampling are not part of original-game parity acceptance.

Owned checks cover shared handles and pitch state, stop calls, different host
buffer boundaries, stereo conversion, duplicate consumers and retained faults.
A local original-content check renders the same two-second AMBIENT scenario as
the independently checked [SPU diagnostic](SPU_OUTPUT.md); its PCM hash must
remain identical through the new continuous runtime. That controlled scenario
is separate from the scene's SE bank and from a natural original encounter.

Faults remain inspectable in `SceneAudioSnapshot`; playback ends on a retained
sound/SDK/device fault instead of resetting synthesis. Whole audio scenes,
music, natural NPC timing and the complete game remain unfinished. No whole
feature group is accepted by these integration checks. Original behavior and
optional AI analysis/modding remain separate.
