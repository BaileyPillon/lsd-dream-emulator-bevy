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
The special Gray Man model now has a dormant per-scene instance and original
activation placement/facing/restart order. The loaded original day gate is
connected, followed by its original forward action, triggered color flash and
flashback-count reset. Natural encounter/render/audio parity remains open.
See [special encounters](SPECIAL_ENCOUNTERS.md) for the comparison scope.
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

Generation style, texture/music choice, placement seeds, trigger selection and
all 14 proximity sound callbacks now have original CPU comparisons. The day
profile consumes selected textures/background colors and queues audio requests.
Generated model/sprite construction and updates, gradient layers, full generation
RNG history, original audio output and complete native scheduling remain open;
see [generation recovery](GENERATION.md).

Generated model/sprite construction, child motion, unsigned ticks, the alternate
color alias and constructor/update RNG are now implemented and compared with
original CPU execution. The day profile loads the original generation model
and texture and presents these objects. Gradient/border composition, complete
allocation/lifetime cases, original GPU projection/ordering and global/native
session comparisons remain open; see [generated objects](GENERATION_OBJECTS.md).

Gradient/environment layer allocation state, parent relationships, offsets,
color shifts and teardown now have original CPU comparisons. Combined generation
frames preserve layer/object/palette/sound order and RNG history in supplied
graphs. The day profile composes development background/foreground layers and
rotates palette strips in VRAM. Original framebuffer/raster, masking, complete
global scheduling and audio output remain open; see
[generation layers](GENERATION_LAYERS.md).

Original recursive box sort, SDK packet construction and ordering-table draw
order now have native comparisons. The day profile composes gradients in RGB555
and blends foreground boxes in an integer shader. Owned pixel plates and three
captured Bevy scenes match the integer model. Original GPU framebuffers, model
projection/lighting, textured rasterization, mask/interlace behavior and complete
draw ordering remain open; see [box rendering](BOX_RENDERING.md).

Original chunk encounter selection and actor construction now feed the day
profile. Canonical model libraries and 130 entity-property records load locally;
the imported actor rigs track part parents, mesh replacement and animation.
Original CPU checks cover signed encounter gates, recursive cousin spawning,
placement, rotation, RNG and constructor/animation state. Loaded slot release,
scene rebuild and day expiry also have integration checks. Named NPC scripts,
Gray Man activation, body/interaction/mood dispatch, audio and full native scene
cadence remain unfinished; see [encounter recovery](ENCOUNTERS.md).

The shared entity start/stop/link state, three clocks, sound context, interaction
policy and target query math now have original CPU comparisons. Futon, Television
and Drumstick callbacks are implemented, including canonical model/property
comparisons for the first two. The scene adapter connects the query policy and
queues sound requests. Recover the remaining callbacks and complete natural
activation, inherited body response, full scene scheduling and original GPU/SPU
output. Whole NPC encounters remain unaccepted;
see [entity behavior](ENTITY_BEHAVIOR.md).

Actor journal type/state gates, mood/score, recall capture, direct dynamic links
and video/end-clock effects now have 336 owned original CPU comparisons. The day
profile delivers these notifications immediately with the shared RNG, and a
loaded Futon scenario verifies a single mood/score contribution. Inherited body
acknowledgement, cinematic playback and full original scene order
remain open; see [actor journal recovery](ACTOR_JOURNAL.md).
Original continuous facing now applies during interaction, and Bear, Rabbit
and Peacock actions preserve their original movement, sound and RNG rules.
See [animal actions and facing](PURSUIT.md); full natural encounters remain open.

Local-axis movement and pose setters now have original CPU comparisons, and nine
additional callbacks are connected. Eight have controlled comparisons using
canonical encounter/model/property bindings. The day-eight Yokai capture uses
the original proximity policy to start its script, change clip and move its
render instance. Inherited movement/body consequences, the remaining callbacks,
natural scene scheduling and original GPU/SPU output remain open; see
[actor motion](ACTOR_MOTION.md).

Inherited entity body-message gates, contact/disable policy and synchronous
journal acknowledgement consequences now have original CPU comparisons. The
journal base stores contact before its sender/link-state filters, correcting an
omission in the earlier adapter. Moving-actor/player geometry queries now execute;
movement collision response remains pending, alongside complete native scene ancestry and
scheduling; see [entity body routing](ENTITY_BODY.md).

Imported actor collision bindings and model bounds now have 900 original CPU
observations across 100 canonical models. Cuboid construction, movement extension
and cached-world matrix application have 504 owned comparisons. The encounter
adapter retains imported collision geometry; complete world cache/ancestry,
reciprocal queries and movement correction remain open. See
[actor collision geometry](ACTOR_GEOMETRY.md).

Moving actors now deliver prepared clouds to the imported player geometry and
apply contact/link/video/end consequences before returning. The player uses
the original `DREAME5.TMD` body, correcting the earlier flat `DREAMER.TMD` bounds.
300 owned supplied pair scenarios include 40 contacts; 20 original Yokai/player
mesh scenarios include four contacts. Natural cache/ancestry timing, movement
response and complete encounters remain open. See
[actor/player queries](ACTOR_PLAYER_QUERIES.md).

Moving-actor queries now carry cached world position independently of the new
local pose. 270 full original movement-wrapper scenarios compare cached matrices,
positions, margin arithmetic and cloud notifications. Full movement/player-pair
comparisons include 32 owned scenarios and 16 original Yokai/player-mesh scenarios.
Using the new local position instead of the cached position disagrees with the
original contacts in 20 owned and eight original-mesh scenarios. Natural hierarchy/
frame scheduling, reciprocal queries and movement response remain open; see
[cached movement](CACHED_MOVEMENT.md).

The encounter channel now tracks clock attachment order separately from actor
storage. Original start/stop/restart routines establish newest-first delivery;
32 controlled three-actor clock scenarios compare 416 states, shared RNG,
sound requests, waiting signals and finish cleanup. The phase counter/flags have
36 original CPU comparisons. A live cursor now replaces snapshot dispatch and
finished actors retire during iteration. 88 original-clock scenarios with owned
mutation callbacks compare 176 notifications and 698 callback observations,
including next-member removal, restart, duplicates, self destruction and flag
changes. Global player/map/generation/render/CD ordering and NPC-requested
births/removals during callbacks remain open. See
[actor clock recovery](ACTOR_CLOCK.md) and [live traversal](FRAME_CURSOR.md).

The original emulator now supplies 121 read-only observations of the actual
cottage clock graph: five actors, map, stream context, renderer, session counter
and player. Original display-timer/player/pad comparisons cover 120 sequences
and 3,840 display events. The linked-player step now advances clock callbacks
before polling new input; its separate clock and pad APIs allow later scene
integration. Connect the observed actor/map/generation/stream/render/player order,
session handoff and pad phase to one shared clock, including delayed actor births,
waiting and transform-cache refresh. See [frame and pad order](FRAME_ORDER.md).

The loaded scene now connects actor/effect/map/stream/render/session/player
phases to one clock, followed by pad dispatch and session handoff. 8 original
owned-clock scenarios compare 192 notifications and 24 real actor births during
stream callbacks. A local original-asset integration checks delayed first updates,
shared waiting, journal delivery and render-before-player state. Recover actual
CD completion/wait transitions, original renderer/cache and cross-map residency,
NPC-requested lifetime changes and the remaining fidelity requirements. See
[shared scene frame](SHARED_FRAME.md).

Dog, Gargoyle and Demon King now run with the original player action getter
and movement/view callback clearing. Direct DreamSys link requests have
original CPU comparisons through destination selection, RNG and sound.
Player clock comparisons cover callback clearing, retained camera updates and
input locking. See [player-interacting NPCs](PLAYER_NPCS.md). That checkpoint
implemented 19 actions. The newer
[sound, motion and event work](ACTION_PHASES.md) raises the current total to 31
among 107 distinct callbacks, leaving 76 pending. Natural
encounters and all 40 provisional whole-feature groups still need acceptance.

Ferris Wheel now retains the action window, camera-preserving ride and
video/end request branches, including player local-axis movement. Eleven
further actions add original sound loops, scaling, signed remainder and
animation pause/resume behavior. The Ferris Wheel scene adapter also compares
280 states against original-code observations. This connects existing event
requests; movie playback and full natural encounters remain unfinished.
