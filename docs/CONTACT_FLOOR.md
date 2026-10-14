# Player contact, floor correction and two original actions

Soldier and Giant Turtle now run in the shared Rust encounter scene. There are
38 implemented callbacks among 107 distinct original actions, with 69 pending.
These counts measure action implementations; whole natural encounters and
whole-game parity remain unaccepted. See [branching floor actions](BRANCHING_FLOOR.md)
for the subsequent Giant Maiko, Fox and penguin comparisons.

## Contact and the floor gate

The original movement wrapper clears its contact, moves locally and optionally
notifies the actor's parent. The player mesh query and its synchronous response
can acknowledge the actor. Only an assigned actor contact bypasses the floor
search. A player geometry hit alone does not guarantee that acknowledgement:
the body-message policy may leave the actor contact empty, allowing the floor
query and correction to run in the same call.

The combined comparison executes the original actor/model/player construction,
cached-world geometry query, body acknowledgement, journal policy and floor
wrapper. The Rust replay uses the production journal dispatcher and scene floor
service. It compares local pose, cache state, contact identity, prepared clouds,
query counts, local floor signals, journal order, cinematic requests and timer
changes. Geometry uses the supplied pre-movement world cache; the floor query
uses the resulting local position.

| Original-code comparison | Cases | Actor contact bypasses |
| --- | ---: | ---: |
| Owned actor/player models, supplied floor graphs | 1,280 | 80 |
| Privately imported original actor/player models | 48 | 4 |

The owned cases also include 80 player hits without actor acknowledgement.
Other cases cover a player miss, notifications disabled, two movement axes,
signed movement, body-policy gates, filtering and video/end requests. These are
controlled inputs, rather than observations of a complete natural encounter.

## Soldier and Giant Turtle

Soldier preserves its seven-context-tick note request, volume parameters and
turns at behavior ticks 200, 400, 600 and 800. The last turn resets the behavior
counter through minus one. Giant Turtle preserves its untriggered periodic
note and triggered quarter-scale operation. Both move forward by minus 30
through the original floor wrapper without a movement notification. If that
operation leaves a contact, they then move locally on Y by minus 200 while
retaining the contact. This final movement also replaces the movement amount
stored by the wrapper.

| Original-code comparison | Scenarios | State observations |
| --- | ---: | ---: |
| Owned models/properties/math in eight floor graphs | 144 | 3,172 |
| Privately imported original models/properties/math | 144 | 3,172 |
| Production encounter adapter, supplied live floor scene | 4 | 986 |

The scenarios include full trajectories, turn and sound boundaries, triggered
state, negative counters and signed overflow. The original behavior-start
method is explicitly invoked. Imported model/property rows and their recorded
observations remain private; public fixtures contain owned inputs. Direct
replays compare local virtual-call instrumentation; the encounter adapter
compares resulting actor, action, animation and sound-request state.

```sh
cargo test -p lsd-assets --test native_floor_bypass --test native_actor_floor --locked
```

Private original-model comparisons additionally require `LSD_GAME_FILES`,
`LSD_NATIVE_FLOOR_BYPASS`, `LSD_NATIVE_FLOOR_NPCS` and `LSD_NATIVE_HORSE_FLOOR`.
See [contact and floor checks](contact-floor-checks.json) for input identities,
hashes, repeated recordings and wider validation.

The original routines have no function-output replacements. Static cache and
math inputs are supplied, and the semantic GTE bridge does not establish
hardware flags or timing. Natural activation, reciprocal player movement,
complete map/cache/residency lifetimes, native GPU/SPU output and full sessions
still require comparison. All 40 provisional whole-feature groups remain
unaccepted. The C reference stays paused; these original actions make no AI
model calls. Optional AI tools and gameplay mods remain separate.
