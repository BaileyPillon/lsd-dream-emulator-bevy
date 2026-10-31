# Project status

Last reviewed: **2026-10-31**. Stage: **early development**.

The Rust/Bevy port has substantial recovered systems and an original-content
dream prototype. It does not yet reproduce the complete game. The full target
remains [PROJECT_OBJECTIVE.md](../PROJECT_OBJECTIVE.md).

## How progress is measured

| Measure | Current result | Meaning |
| --- | ---: | --- |
| Named original NPC callbacks implemented | 71 / 107 | About 66.4% of the named callback inventory; 36 remain |
| Complete original NPC encounters accepted | 0 | A callback comparison does not accept a full encounter |
| Provisional whole-game groups accepted | 0 / 40 | Every group still needs its complete acceptance campaign |
| Original audio scenes accepted | 0 | Controlled PCM and host output checks do not accept a natural scene |

The 40-group catalogue is provisional. It is not an official exhaustive count
of the game's features. A single overall percentage would conflate source
recovery, implemented components, and verified gameplay, so none is reported.

## What currently works

| Workstream | Implemented or demonstrated | Still required |
| --- | --- | --- |
| Assets | TIM/TIX textures, TMD geometry, LBD maps, model and animation libraries; 418 maps and 4,260 animation frames decoded | Reproducible complete content import, movies, persistence data, edition coverage |
| Dream runtime | Original-content prototype with movement, collision, streaming policy, links, generation, mood, day clock, and flashbacks | Natural construction/lifetime checks, complete RNG history, original loading and scheduling |
| NPCs | 71 distinct callbacks, shared actor animation, collision, interaction, notification, and clock services | 36 callbacks, complete encounters, special-event and lifetime comparisons |
| Rendering | Imported geometry/textures, integer animation and transforms, environment layers, palettes, and five-bit box composition | Full camera, rasterization, GTE, blending, draw-order, cache, and timing comparisons |
| Sound effects | Shared 24-voice driver/SPU, ADPCM, envelopes, mixing, reverb, continuous Bevy output, synchronous link/journal sounds | Original startup/bank allocation, hardware cadence, natural scene audio comparisons |
| Music | All 70 sequences imported; clock/channel/voice integration checked through full loops of 69 playable sequences; controlled music-to-WAV synthesis | Live dream playback integration, bank choice/lifetimes, pause/replay/fades, audible scene comparisons |
| Menus and persistence | Original scenarios enumerated in the feature inventory | Title/menu flows, chart UI, diary, save/load, special-day media, movies, and endings |
| Optional AI and mods | Explicit data packs, hashed profiles, dependencies/conflicts, streamed scene retention; original mode remains default | Code/plugin APIs, new NPC scripts, AI integrations, hot reload and distribution services |

See [music recovery](SEQUENCE_VOICES.md) for the latest channel/voice work and
its exact validation scope. Native component checks are stronger evidence than
source translation alone, but do not establish complete game parity.

## Evidence and history

- [Documentation index](README.md): implementation notes grouped by subsystem.
- [Frozen baseline and dated progress](parity.json): initial port measurements
  remain unchanged; later milestones appear under `progress`.
- [Reference identity](REFERENCE.md): canonical executable, provenance, and scope.
- [Roadmap](ROADMAP.md): remaining implementation and acceptance work.
- [CI](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/actions/workflows/rust.yml): Windows and Linux checks for the published revision.

Public fixtures use owned content or bounded numeric observations. Original
assets, full native traces, rendered audio, and analysis databases stay local.
Tests that require those local inputs are explicitly ignored in ordinary CI.

## Separate C project

The frozen transition checkpoint remains **194,212 / 199,912 bytes (97.15%)**
with nine assembly functions. Independently, the companion C repository at
[`171f74a`](https://github.com/BaileyPillon/lsd-dream-emulator-decompilation/commit/171f74a3b0b5b8c5e2b595c5a75fe155cdb11a38)
now reports **199,912 / 199,912 game-function bytes (100%)**, 306 current objects
and a matching canonical executable. This was rechecked on 2026-10-31. SDK and
startup assembly are outside that game-function count. The frozen port baseline
has not changed; C recovery does not measure Rust implementation or gameplay
acceptance.

See [data mods](MODDING.md) for the new optional content API. The original NPC
callback count and all whole-feature acceptance counts remain unchanged.
