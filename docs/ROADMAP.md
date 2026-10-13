# Roadmap

The target is a faithful Rust/Bevy port of the original game. Each milestone
needs an implementation and original/port evidence. Completing a milestone
does not imply every provisional feature group is accepted.

1. **Local content pipeline.** Decode TIM/TMD, MOM/MOS and LBD/MML with bounded
   offsets, format tests and content hashes. Record disc edition and provide
   a reproducible import command for locally supplied assets.
2. **First original stage.** Render imported geometry/textures and implement
   original coordinate widths, movement, camera, collision and stream selection.
   Replace the procedural preview with a reference stage and compare paths.
3. **Dream and NPC systems.** Port linking, generation, day/seed transitions,
   animation interpretation and original entity scripts. Preserve RNG call order
   and distinguish partial tests from complete encounter acceptance.
4. **Complete presentation and persistence.** Recreate menus, graphs, diary,
   original save formats, music, sound, movies, special events and endings.
5. **Parity campaign.** Replay enumerated scenarios with identical assets,
   seeds and inputs. Review state/event, visual and audio differences, including
   fixed-point/GTE effects, affine texture mapping and draw-order behavior.
6. **Optional development tools and mods.** Add AI analysis/import assistance,
   authoring workflows and clearly selectable gameplay changes. These have a
   separate scorecard and do not increase original-feature acceptance.

Current original scalar observations support three small Rust primitives.
All 40 provisional whole-game feature groups still require acceptance in this
port. Use [parity.json](parity.json) for the explicit initial baseline.
