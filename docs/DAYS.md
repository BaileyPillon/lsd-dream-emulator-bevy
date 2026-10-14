# Original days, moods, dream clock and flashbacks

The Rust core now implements original day preparation and completion, year/day
advancement, previous/average mood lookup, contribution logging, chart and color
selection, navigation/instance scores, initial spawn selection, special-day
selection, dream-clock expiry and flashback storage/load policy. Original
content is imported from the canonical local executable; tables are not embedded.

Normal startup clears the contributor counters, saves the current day and resets
the dream clock. It checks all 42 special-day table entries in original order.
A matching entry consumes one RNG call and selects a cinematic variant. Other
days use the previous mood to search stage/chunk data, then find a matching spawn
or the original fallback entry. The unmatched-mood path preserves the original
random-spawn call order and penalty change.

Clock ticks compare the previous unsigned timer with the signed limit viewed as
unsigned, then increment with 32-bit wrapping. Expired ticks skip the view/move
callbacks, optionally save/load a flashback and notify the map before resetting
the timer. Modeled callback notification order and state mutation match the native
CPU comparisons. The Bevy host still has a development global scheduling order.

Several original quirks are retained:

- Mood averages wrap an out-of-range axis to the opposite end, including the
  contribution from the last sample; they do not saturate.
- At day zero in a later year, the previous-mood pointer reads the high two bytes
  of the instance score immediately before the history array.
- At ten flashbacks, replacement uses unsigned timer modulo nine, preserving the
  tenth entry. Rotation packets, halfword truncation and untouched padding remain.
- Loading a flashback notifies before copying its stored day/stage/spawn. The
  modeled actor attachment step applies stored rotation, extends the recorded time limit
  and advances the flashback index.
- New-game initialization clears only the fields/byte ranges the original resets;
  existing mood history and old flashback record bytes remain.

## Runtime profile

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A --dream
```

This profile supplies a fresh day-one bootstrap and seed 1, uses the original
`DREAMER.TMD` model for its bounds, prepares the original spawn/time limit and
connects the day clock to loaded movement/link policy. It consumes chunk-mood
requests and the end-day result. The profile supplies the camera/host schedule
and now consumes the recovered scene texture/music choices and background color.
The generation actor channel queues original sound requests. Original generated
objects, the full generation RNG history, audio and final camera/render setup
require further work; see [generation recovery](GENERATION.md).

`--dream-capture /path/to/new.png 32` captures the prepared first dream after idle
host ticks. With 4000 ticks the original 3600-tick limit expires, the chart is
committed and the calendar advances to day two with normal result 1. The final
image remains the scene inspection view: original menu handoff, chart/diary UI,
special-day movies/ending media and GPU fade composition remain unfinished.

## Evidence and remaining acceptance

Generated-content fixtures compare 96 starts, 144 end-day cases, 384 clock ticks,
144 flashback-save cases, 24 loads, one new-game reset, 32 previous-mood cases and
16 chart/color cases: 841 native scenarios. They include unsigned timer wrapping,
year overflow, retained storage, both flashback modes and notification cancellation.
Original RNG, mood/spawn tables, scores, copies and cached actor key/rotation
getters execute directly. Parent notification and view/move callbacks are supplied
boundaries in this campaign.

A private original-content campaign compares every calendar day in two year
states and two RNG seeds: 1460 startups. It matches stage, spawn, time limit,
special selection, contributor state, RNG and penalty changes. Original table
bytes and observations stay private.

An imported first-day test connects startup, loaded placement, effect/session
state, timeout, chart/score/day commit and result delivery. It is a port integration
test, not a complete original-session replay. The no-day linked traversal capture
and all 17 supplied permanent-link replacements continue to pass.

Natural actor construction, all instance/NPC contribution and flashback callbacks,
original global/CD timing, generation-driven origin/objects, menus,
save/load, media, audio and original GPU/camera comparisons remain incomplete.
No whole original feature group is accepted by these checks. See
[day-recovery-checks.json](day-recovery-checks.json) for exact fixture, source and
validation hashes. The matching C checkout remains paused and unchanged.
