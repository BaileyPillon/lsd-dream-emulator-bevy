# Complete LSD: Dream Emulator in Rust and Bevy

Recreate the original game's content and behavior in the Bevy port. The matching
C project is a preserved, read-only reference while its decompilation is paused.
The port has its own implementation, evidence and acceptance tracker.

Completion requires all of the following:

1. Original content can be imported reproducibly from local reference inputs:
   textures, geometry, animation libraries, maps, sounds, music, movies and
   persistence data. Record executable identity and disc edition.
2. Original movement, camera, collision, linking, dream generation, RNG call
   order, day progression, events and NPC scripts run in Rust with their
   original integer widths and state transitions.
3. Menus, graphs, diary, save/load, error handling, special days and endings
   implement the enumerated original scenarios.
4. Rendering, animation and audio reproduce the reference behavior, including
   relevant fixed-point, GTE, texture, transparency, palette, draw-order and
   timing effects. Bevy defaults and asset previews require further review.
5. Original/port comparisons use identified inputs, assets, initial states and
   scenarios. Narrow scalar tests, static asset counts and a rendered frame do
   not accept a complete feature group. Review every provisional group and
   resolve inventory gaps before reporting end-to-end parity.
6. Original NPC scripts, AI-assisted development tools and optional AI-driven
   gameplay mods remain clearly separated. The default original experience
   has no dependency on AI accounts or generated behavior.

The 40-group catalogue is provisional, not an official exhaustive feature
total. Keep the initial port snapshot and C checkpoint frozen. Add dated
progress evidence; do not transfer C coverage into Rust completion or revise
the goal around what currently passes tests.
