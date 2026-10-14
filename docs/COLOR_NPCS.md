# Live dream color and color-dependent NPCs

Astronaut, Rainbow and Boat Bird now run in the shared Rust encounter scene.
The current count is 41 implemented callbacks among 107 distinct original
actions, with 66 pending. Whole natural encounters and game parity remain
unaccepted.

## Live services

The original color query averages the current chunk and instance mood
contributors, applies its signed averaging and boundary rules, then reads the
three-by-three color table. With no instance contributors, the chunk average
supplies both halves of the blend. The Rust actor journal dispatcher now
performs that live query for NPCs. It reads the current day state on every call;
the scene's cached display palette is separate.

Boat Bird also requires a distinct player-local-position service. Its facing
operation reads cached world position, while its later copy operation reads
the player's local coordinate. The running session supplies both services.
The comparisons deliberately use different local and cached world inputs.

## Original actions

Astronaut preserves its normal forward/back movement and tick-2400 loop.
Pink color selects its alternate route: upward translation, animation freeze
at tick 250, later drift and absolute orientation changes. Freezing animation
keeps the action callback running, as in the original.

Rainbow preserves its first-tick white-color scale and local-Y movement,
then consumes one RNG call to choose among five states. State zero continues
its original yaw motion.

Boat Bird preserves the blue-color route, target-facing movement and ascent
window from ticks 85 through 114. Tick 120 changes its state; the following
update copies player-local position and adds the original offset. Other colors
select its twofold scale and downward local movement.

## Evidence

| Original CPU comparison | Scenarios or queries | State observations |
| --- | ---: | ---: |
| Live color query with owned table and supplied contributors | 484 | 484 |
| Live color query with original imported table | 484 | 484 |
| Full actions with owned model/properties/math | 104 | 4,646 |
| Full actions with original imported model/properties/math | 104 | 4,646 |
| Production encounter adapter using live dispatcher services | 104 | 4,542 |

The query cases include signed-byte boundaries, empty contributors, last-mood
weighting, signed sums and negative counts. They check the resulting chart and
color and confirm that querying does not change the contributor state.
The production service is tested with a deliberately different cached palette.

Action comparisons cover all color-table cells, five Rainbow RNG outcomes,
full normal Astronaut movement through its loop, the alternate route through
its final orientation, Boat Bird's approach/copy phases, boundary ticks and
signed overflow. They compare pose, movement amount, cache state, action and
animation clocks, part state, sound requests, RNG, parent notifications and
color/local-position/facing service calls. All 104 owned action scenarios also
run through the production encounter adapter; it compares resulting state and
forwarded color/local-position calls. Direct replay compares facing-call counts.

```sh
cargo test -p lsd-assets --test native_color_npcs --locked
```

Private original-content comparisons additionally require `LSD_GAME_FILES` and
`LSD_NATIVE_COLOR_NPCS`. Original models, property rows, color table and detailed
original-model observations remain private. Public fixtures use owned inputs.
See [color NPC checks](color-npc-checks.json) for identities, hashes, repeated
recordings and wider validation.

The original allocator, model/entity/transform constructors, chart/color,
action, animation and sound-context code execute without replacing original
game-function outputs. Initial mood contributors, behavior start, tick/state,
seed and static local/world inputs are supplied. The semantic GTE bridge and
recorded sound services do not establish hardware timing, flags or GPU/SPU
output. Natural activation, reciprocal player response, full cache/residency
lifetimes, media and complete sessions remain unfinished. Whole NPC encounters
and all 40 provisional feature groups remain unaccepted. C stays paused;
original actions make no AI calls, with optional AI tools and mods separate.
