# Changelog

This log summarizes development milestones. Entries describe merged work, not
stable releases or whole-game parity. See [project status](docs/STATUS.md) for
current capabilities and [the commit history](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/commits/main/) for individual changes.

## Unreleased

- Add opt-in data mods with validated manifests, hashed immutable snapshots,
  explicit load order, dependencies/conflicts, streamed-destination retention,
  original-only isolation, a headless checker and an authoring guide.
- Refresh the separate C reference status to 100% game-function recovery while
  preserving the frozen Rust baseline and all gameplay acceptance counts.

- Publish revision-identified Rust API documentation alongside the handbook,
  with generated source, search, and checked links between all three libraries.
- Shorten the repository overview and separate platform build coverage from
  playable-preview and original-game acceptance status.
- Publish a searchable project handbook with mobile navigation, light/dark
  themes, source links, and checked deployment from protected `main`.
- Validate generated documentation links and assets in the required repository
  check; pin documentation dependencies and track their updates.
- Add crate API overviews and checked examples, a source map, linked recovery
  evidence, platform coverage, and warning-free API documentation builds in CI.
- Add a developer tool catalogue, fork contribution instructions, and a
  structured original-game compatibility issue form; clarify probe help text.
- Track implementation and acceptance work in three undated GitHub milestones.
- Require pull requests and the Windows, Linux, and repository checks on `main`.
- Add monthly dependency update proposals and a reusable compatibility report.
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
