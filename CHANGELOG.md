# Changelog

This log summarizes development milestones. Entries describe merged work, not
stable releases or whole-game parity. See [project status](docs/STATUS.md) for
current capabilities and [the commit history](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/commits/main/) for individual changes.

## Unreleased

- Refine repository presentation, contributor guidance, support routing, and
  maintenance documentation.
- Add automated checks for local documentation links and public JSON evidence.
- Document Linux X11 runtime dependencies and dream-prototype controls.

## 2026-10-15 — Music voice and sequence recovery

[Reference commit: 19651c2](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/commit/19651c2966292684b8168be3d20a2f34763ea2ae)

- Recover original SDK sequence note allocation, controller volume/pan/expression,
  pitch bend, and pitch-table behavior.
- Check integrated clock, channel, and voice updates through full loops of 69
  playable local sequences. Retain the zero-tempo sequence's original fault.
- Add controlled music-to-WAV synthesis with independent software PCM comparison.
- Keep live dream music, original bank lifetimes, pause/replay/fades, and natural
  scene audio comparisons open.

## 2026-10-14 — Shared scene audio

- Connect live sound effects to a shared 24-voice driver and clocked SPU with
  sample memory, ADPCM, envelopes, interpolation, mixing, and reverb.
- Dispatch link and journal sounds at their original callback positions.
- Recover original sequence timing and full-loop event scheduling.
- Retain explicit limits around device cadence, original hardware, and complete
  audio-scene acceptance.

Subsystem notes and their input identities are indexed in
[the documentation](docs/README.md). Frozen baselines and dated observations
remain in [the parity tracker](docs/parity.json).
