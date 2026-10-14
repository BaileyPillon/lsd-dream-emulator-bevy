# Original generation choices and proximity sound

The Rust core implements the original day/location style selector, fixed-stage
entries, palette and height lookup, random/daily placement seeds, generation
entry search, two sound-trigger slots and all 14 generation sound callbacks.
Generation tables and paths are loaded from the canonical local executable.
Original table bytes and game assets are not distributed.

The implementation preserves several details that affect later behavior:

- Music selection consumes a random result before texture selection, even when
  a track override is active. Stage nine retains its original track remapping.
- Texture availability advances through the original ten-day divisions of a
  repeating forty-day window.
- Daily placement discards its first random result. The field historically
  named `SpawnRoll` is position Z, rather than the rotation packet.
- A five-way offset lookup on a four-word table reads the neighboring table's
  first word. The loader preserves that alias, as well as the style lookup's
  original address relative to the effect callback table.
- Entry search advances its cursor before testing an entry, skips nonpositive
  effect IDs and uses strict X/Z Manhattan range. A missing position preserves
  output storage. Active entries negate their effect ID until deactivation.
- Sound context storage survives deactivation. The original scheduler resets
  requested IDs and envelope fields before invoking the callback, retains its
  handle behavior and wraps the callback tick after a reset to -1.

## Bevy integration

The `--dream` profile selects music and textures when starting each scene and
loads the chosen original texture variant. It uses the recovered scene color
for the development view's background. Texture changes invalidate the resident
mesh/material cache even when the stage number stays the same.

The actor channel runs after the inspector refreshes the cached player key.
It queues original sound requests for a future audio backend; requests currently
receive the original failed-voice handle (-1). Selecting a music path and queuing
sound requests do not establish audible playback. Height and environment-effect
colors are retained as original state; their full scene/render consumers remain
unfinished. The host schedule is a development integration, with original global
ordering and timing still requiring comparison.

## Evidence

Owned-content CPU fixtures cover 308 style starts, 45 random and 45 daily seed
cases, 1,218 callback states, 360 music-then-texture choices, 64 entry searches
and 42 actor sequences containing 504 update ticks. They execute the original
RNG, placement, selection, callbacks and sound scheduler. The external sound
virtual methods are supplied sinks. Pointer identities in sound context are
represented by typed slot/callback identities in Rust and normalized in comparisons.

A private original-content campaign covers all 365 calendar days and 14 stages:
5,110 style starts and 10,220 music-then-texture choices using two seeds. It
compares the imported tables, path records, choices and resulting RNG state;
selected files also exist in the local disc extraction. The 56 original
generation trigger records and their style/range tables load locally.

The loaded first-dream integration, end-day result and 17 supplied permanent-link
destinations remain checked. Development captures exercise 32 and 4,000 supplied
idle host ticks. Their images establish the Bevy integration output only.

## Remaining work

Generated model/sprite construction, child updates, gradient effects, border
composition and cleanup still need their complete native graph and rendering
implementation. In particular, structure construction and updates consume
additional random results. The current actor/choice integration does not yet
reproduce the full generation subsystem's RNG history. An earlier unverified
structure-planning draft was preserved privately and is not part of the runtime.

Natural original NPC encounters, audio/SPU output, PS1 GPU/camera precision,
CD scheduling and full session replay remain unaccepted. These checks do not
accept a complete generation feature or raise whole-game feature acceptance
above 0/40 provisional groups. AI-assisted tools and gameplay mods remain separate.

See [generation-recovery-checks.json](generation-recovery-checks.json) for
fixture, function, source and validation hashes. The matching C checkout remains
paused and preserved.
