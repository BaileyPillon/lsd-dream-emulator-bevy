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

Current checks cover original scalar primitives, asset decoding, integer
animation, controls and collision, plus scene lookup and query selection.
Body geometry also has fixed-point parent rotations compared with the original
hit/notification path in supplied graphs; natural actor hierarchies remain open.
The streamed viewer now uses recovered neighbor, slot-reuse and completion
rules. Original CD timing, complete binding/actor callbacks and session
scheduling remain open. Link lookup, random destinations, facing gates and
transition policy now have original CPU checks; natural link callbacks and
visible transitions still need integration. All 40 provisional whole-game groups require acceptance; use
[parity.json](parity.json) for the frozen initial baseline and dated progress.
Dream link handlers, cached actor keys and all four stair scripts now have
original CPU comparisons. Their scene/notification interfaces still need
connection to the running Bevy session and its full transition lifecycle.
The `--walk-links` streamed inspector now connects movement, contacts and original
link/map-notification policies. The original scalar effect lifecycle and map
completion callbacks now consume those requests. Session parent policy and
player-motion reset also have original CPU comparisons; the inspector rebuilds
destination geometry/textures after completion. Original day generation,
asynchronous scene construction, GPU composition, natural parent lifetimes and
scheduling remain open; see [session recovery](SESSION.md).
Day preparation, previous/history mood, chart/score rules, signed year wrapping,
flashback records and timer expiry now have original CPU comparisons. The new
`--dream` profile connects an imported first day through timeout and day commit;
original generation, UI/media and full native-session comparisons remain open.
See [day recovery](DAYS.md).
