# Linked scene integration

The streamed Bevy inspector now has a `--walk-links` mode. Movement processing
first returns the original action, then the link dispatcher checks stairs,
teleports and tunnels before applying normal motion. The inspector uses the
original map-scene speed defaults and 16-tick dynamic-link interval.

Body traversal exposes each cell stamp and successful contact in native order.
Callbacks can change the live query before the next tile. Wall handling copies
the contact key, tries the permanent link, optionally selects a random
destination, sends the transition/sound requests and restores the saved pose.
Floor handling retains original surface masking, tunnel surface adjustment and
the ready-halfword gate for fall links or restoration.

The original map-notification policy now accepts or cancels the transition. It
preserves the original fade requests, color steps, input locks, pause modes, movement
callback selection and source rotation. The inspector now consumes effect
requests with the original scalar lifecycle and map completion callbacks.
The original session policy now receives completion notifications, and the
inspector reloads destination maps/textures with the original motion reset.
GPU fade composition, sound playback and full original scene construction remain
open; see [effect recovery](EFFECTS.md) and [session recovery](SESSION.md).

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 3 136 A --walk-links
```

For a finite supplied-input capture, replace the final option with
`--walk-links-capture /path/to/new.png 120`. The capture profile supplies forward
input, a first-found floor spawn, seed 1, palette indices 0 and a 15 Hz development
schedule. It bypasses original startup/day preparation. Its clock increments and
cache refresh are host integration choices; original timeout/flashback behavior,
global callback order and timing still need complete session comparisons.

The full native coordinate snapshot is 80 bytes plus a 40-byte tail. Its numeric
save/restore is implemented and checked separately. The current Bevy player owns
position/angles and restores those fields; natural GTE cache, parent and matrix
ownership remain open.

## Evidence

Native CPU fixtures check 240 wall cases, 360 floor cases, eight complete
coordinate snapshots and 864 map-notification/start cases. Notification, sound,
color/effect services and the floor owner query have explicit supplied boundaries.
Another fixture executes the original body dispatcher through transforms,
bounds and geometry. A supplied actor contact callback moves the cached world
position; the next tile reads the changed value. Rust matches both hit sequences.

A loaded-scene test connects input, a wall contact, permanent destination,
notification, input lock, fade request, sound request and pose restoration.
A private original-input campaign additionally runs all 17 instant-teleport
triggers through locally imported maps, player bounds, math and link tables.
It supplies trigger poses and enabled teleports, and checks the resulting
destination, immediate position and parent notification. It does not accept
natural startup, complete encounters or visual parity.

The linked inspector renders supplied cottage and natural-world captures; those
two captures exercise traversal without reaching a link. The legacy streamed
capture remains byte-identical. Original images and inputs stay private.
See [linked-scene-checks.json](linked-scene-checks.json) for exact scope and hashes.

Natural actor lifetimes, every floor/entity callback, GPU effect composition, full timer and
flashback logic, scene rebinding, palette/day preparation, camera scheduling,
audio, NPCs and full-game comparisons remain open. Whole original acceptance is
still **0/40 provisional feature groups**. The C checkout remains paused.

The effect request parameter at native offset `0xD0` sets the per-tick color
step, rather than a duration. Native `effect_activate` computes its initial life
from `256 / color_step` (with additional linked-effect adjustment), and
`effect_tick` checks the previous life before completion. These requests must
reach the original effect lifecycle before full fade timing is accepted. The older
numeric map fixture labels this captured setter `duration`; the observation
values remain valid and that historical label does not describe its semantics.

The effect start call takes a variant and a mode; its second numeric argument
is a mode, not a target color. `MapLinkCommand` now names those fields explicitly.
The scalar effect lifecycle is now compared with original CPU execution, while
the inspector's effect-tick position within its host loop remains a development
schedule requiring complete original-session comparison.
