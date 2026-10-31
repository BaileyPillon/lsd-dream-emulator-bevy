![LSD: Dream Emulator — an independent Rust and Bevy reconstruction](assets/project-banner.svg)

# Project handbook

Build, explore, and contribute to the independent Rust/Bevy reconstruction of
**LSD: Dream Emulator**. This handbook connects practical setup guides with the
implementation notes and comparisons behind the project.

**Development preview.** The desktop dream prototype is explorable; the full
game's menus, movies, persistence, and complete parity remain in progress.

Start with [project status](STATUS.md), [getting started](GETTING_STARTED.md),
and [the roadmap](ROADMAP.md). The [project objective](../PROJECT_OBJECTIVE.md)
defines the full target; [contributing](../CONTRIBUTING.md) explains how to help.

## Find your next step

| Task | Read this |
| --- | --- |
| Install and explore the prototype | [Getting started](GETTING_STARTED.md) |
| Install or author a data mod | [Modding](MODDING.md) |
| Choose a binary, feature, or test target | [Developer reference](DEVELOPMENT.md) |
| Browse Rust types, examples, and source | [API reference](API_REFERENCE.md) |
| Understand crate boundaries and original behavior | [Architecture](ARCHITECTURE.md) |
| Check capabilities and remaining work | [Status](STATUS.md) and [tracked roadmap](ROADMAP.md#tracked-work) |
| Report an original-game difference | [Compatibility issue form](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/new?template=compatibility.yml) |
| Document a behavior comparison | [Compatibility report template](templates/COMPATIBILITY_REPORT.md) |

Recovery notes describe bounded subsystem evidence at individual checkpoints.
Later work may supersede a checkpoint. Use the status page for the current
summary and the linked check manifests for input identity and comparison scope.

## Project and setup

- [Project status](STATUS.md)
- [Getting started](GETTING_STARTED.md)
- [Developer reference and tool catalogue](DEVELOPMENT.md)
- [Published Rust API reference](API_REFERENCE.md)
- [Roadmap](ROADMAP.md)
- [Architecture](ARCHITECTURE.md)
- [Reference and starting point](REFERENCE.md)
- [Development changelog](../CHANGELOG.md)
- [Maintenance and releases](MAINTENANCE.md)
- [Documentation preview and publishing](DOCUMENTATION.md)
- [Compatibility report template](templates/COMPATIBILITY_REPORT.md)
- [Support and reporting](../SUPPORT.md)

## Content and animation

- [Original map, texture and animation pipeline](ASSET_PIPELINE.md)
- [Original animation playback](ANIMATION_RUNTIME.md)
- [Original sound-bank and sequence import](AUDIO_IMPORT.md)

## World, movement, and dream sessions

- [Movement and collision recovery](MOVEMENT_COLLISION.md)
- [Scene lookup and layered movement](SCENE_RECOVERY.md)
- [Spatial stream recovery](STREAMING_RECOVERY.md)
- [Body dispatch and tile attributes](BODY_DISPATCH_RECOVERY.md)
- [Body transforms and native hit checks](BODY_TRANSFORMS.md)
- [Original link rules](LINKS.md)
- [Dream link handlers and actor keys](LINK_RUNTIME.md)
- [Linked scene integration](SCENE_LINKS.md)
- [Original session policy and destination scenes](SESSION.md)
- [Original days, moods, dream clock and flashbacks](DAYS.md)
- [Original generation choices and proximity sound](GENERATION.md)
- [Original generated models, sprites and update RNG](GENERATION_OBJECTS.md)
- [Dream layers, palette rotation and combined frame order](GENERATION_LAYERS.md)

## Actors, NPCs, and interaction

- [Entity lifecycle, perception and original callbacks](ENTITY_BEHAVIOR.md)
- [Chunk encounters and multipart actors](ENCOUNTERS.md)
- [Actor motion and nine additional callbacks](ACTOR_MOTION.md)
- [Actor journal consequences](ACTOR_JOURNAL.md)
- [Entity body routing and synchronous acknowledgement](ENTITY_BODY.md)
- [Actor collision bindings and prepared clouds](ACTOR_GEOMETRY.md)
- [Moving actors and player geometry](ACTOR_PLAYER_QUERIES.md)
- [Cached world state during movement](CACHED_MOVEMENT.md)
- [Actor clock subscriptions and frame signals](ACTOR_CLOCK.md)
- [Live clock traversal](FRAME_CURSOR.md)
- [Original frame graph and pad phase](FRAME_ORDER.md)
- [Shared scene clock](SHARED_FRAME.md)
- [Original floor-following actor movement](ACTOR_FLOOR.md)
- [Player contact, floor correction and two original actions](CONTACT_FLOOR.md)
- [Branching movement actions](BRANCHING_FLOOR.md)
- [Live dream color and color-dependent NPCs](COLOR_NPCS.md)
- [Movement actions and complete entity updates](MOTION_NPCS.md)
- [Original scene scale and requesting NPC actions](SCENE_SCALE.md)
- [Triangle Head and player callback restoration](TRIANGLE_HEAD.md)
- [Twelve further movement, sound and animation actions](ADDITIONAL_ACTIONS.md)
- [Day, proximity and animation-frame actions](DAY_FRAME_NPCS.md)
- [Special encounter activation](SPECIAL_ENCOUNTERS.md)
- [Gray Man action and retained effect](GRAY_MAN.md)
- [Animal actions and player-facing policy](PURSUIT.md)
- [NPC actions that read or disable player controls](PLAYER_NPCS.md)
- [Original NPC sound, movement and event phases](ACTION_PHASES.md)

## Rendering and effects

- [Original effect lifecycle](EFFECTS.md)
- [Original box packets and integer composition](BOX_RENDERING.md)

## Sound and music

- [Original software voice selection, release and pitch](AUDIO_VOICES.md)
- [Original sound-effect note setup](AUDIO_NOTES.md)
- [Original sound objects, volume updates and register flushing](AUDIO_CLOCK.md)
- [SPU sample decoding, interpolation and pitch arithmetic](SPU_SAMPLES.md)
- [SPU envelopes and volume controls](SPU_ENVELOPES.md)
- [Clocked SPU voice engine](SPU_ENGINE.md)
- [SPU reverb, final PCM and a sound diagnostic](SPU_OUTPUT.md)
- [Continuous scene audio in Bevy](LIVE_AUDIO.md)
- [Link and journal sound callbacks](LINK_AUDIO.md)
- [Original sequence clock and event timing](SEQUENCE_CLOCK.md)
- [Music voices and sequence integration](SEQUENCE_VOICES.md)

## Evidence and optional tooling

- [Frozen baseline and dated progress](parity.json)
- [Original NPC scripts, AI analysis tools, and optional mods](../tools/ai-assist/README.md)
- [Attribution and reference inputs](../NOTICE.md)
- [Security policy](../SECURITY.md)

The JSON check manifests linked from recovery notes record counts, input and
fixture hashes, and limitations. Original assets and detailed private traces
are not distributed with the repository.
