# Roadmap

The target is the complete original game in Rust and Bevy. The
[project objective](../PROJECT_OBJECTIVE.md) defines completion;
[project status](STATUS.md) describes the current implementation.
Every workstream requires implementation and original/port comparisons.

| Workstream | Current foundation | Next acceptance gate |
| --- | --- | --- |
| Content | Geometry, texture, map, animation, and audio decoders | Reproducible import of all required content and media from identified local inputs |
| Dream simulation | Movement, collision, links, generation, mood, days, flashbacks, and shared frame services | Matched natural sessions with complete RNG, loading, clock, and lifetime history |
| Original NPCs | 71 / 107 callbacks and shared actor services | Remaining 36 callbacks, then complete encounters with original content and timing |
| Rendering | Imported content, integer transforms/animation, environment layers, palette and box composition | Original camera, GTE, rasterization, draw order, transparency, fades, and full-frame comparisons |
| Audio | Shared 24-voice SPU and live sound effects; sequence clock and music voice recovery | Original music/bank lifecycle and shared playback, then original scene PCM and timing comparisons |
| UI, media, persistence | Scenario inventory | Menus, chart, diary, save/load, errors, movies, special days, and endings |
| Optional tools and mods | Explicit, hashed data packs with load order and dependency/conflict checks ([guide](MODDING.md)) | Expanded authoring APIs, analysis integrations and distribution workflows |

## Tracked work

Milestones group work by outcome, without promised release dates. The linked
issues carry scope, reference anchors, and acceptance criteria. Open tasks are
not completion claims.

| Milestone | Implementation and acceptance tasks |
| --- | --- |
| [Playable dream preview](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/milestone/1) | [Reproducible content import](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/1), [live dream music](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/2), [remaining NPC callbacks](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/3), [natural dream and encounter comparisons](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/4) |
| [Complete game flow](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/milestone/2) | [Menus, diary, and persistence](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/5), [movies, special days, and endings](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/6) |
| [Reference parity acceptance](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/milestone/3) | [Original rendering and presentation](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/7), [complete session and feature acceptance](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/8) |

Optional AI-assisted analysis and modding follow the separate
[tooling plan](../tools/ai-assist/README.md); they do not alter these original-game
acceptance gates.

## Immediate priorities

1. Connect music event dispatch to shared SDK/SPU playback and recover original
   bank loading, allocation, pause/replay, stop, and fade behavior.
2. Complete the named NPC callback inventory while checking interaction,
   collision, clock, sound, and lifetime behavior in natural encounters.
3. Implement the original UI, media, persistence, and special-day routes.
4. Expand reproducible original/port session, image, audio, and timing comparisons.

## Acceptance rules

- Identify the executable/disc edition, asset hashes, starting state, seed,
  input sequence, timing assumptions, expected outputs, and known exclusions.
- Preserve original integer widths, wrapping, RNG call order, and notification
  order. Record intentional differences as optional modifications.
- Separate component comparisons, controlled integrations, and natural complete
  scenarios. Passing one level does not accept the next.
- Keep the frozen C checkpoint and initial Rust baseline unchanged. Add dated
  evidence under [parity.json](parity.json)'s `progress` section.
- Review all 40 provisional groups and resolve inventory gaps before declaring
  complete parity. The provisional catalogue is not an exhaustive official total.
- Track original NPC scripts, AI analysis tools, and optional AI gameplay mods
  separately. Tool installation and model calls do not complete original features.

## Find the underlying work

The [documentation index](README.md) links the recovery notes, implementation
boundaries, and check manifests for each subsystem. Historical milestone notes
describe the evidence available at that checkpoint; use [STATUS.md](STATUS.md)
for the current summary.
