# Entity lifecycle, perception and original callbacks

The day profile now runs the original interaction policy against imported actors
and the player's integer pose/view. The shared lifecycle and three-slot sound
context run in Rust. Futon, Television and Drumstick callbacks are implemented;
the other 104 distinct callbacks retain an explicit pending state.

This is partial original-game recovery. Trigger, link, video and end notifications
now reach the original journal policy immediately; see
[actor journal recovery](ACTOR_JOURNAL.md). Aim and body acknowledgement requests
remain queued. Inherited body dispatch, complete NPC scripts, parent/camera
construction, global cadence and GPU/SPU output remain unfinished. Optional AI tooling is
separate and the default simulation makes no AI model calls.

## Implementation

`EntityBehavior` retains original enable/disable, start/stop, trigger and active
state. Enabling a link clears the part hide bit and resets the interaction timer.
Disabling hides the part meshes, stops behavior and clears the trigger. Positive
unlock initialization sets bits three through five to one; the previous adapter
incorrectly treated that vtable method as a bit-six setter. Real native entity
constructors establish the corrected flags.

The original has three clocks: the interaction timer at actor offset `0x24`,
the sound-context tick at `0xA0`, and the behavior tick at `0xFC`. Action runs
after the interaction-timer increment and before animation. Behavior callbacks
see the old context/behavior ticks, then those counters advance independently.
Start/stop and overflow preserve that order. `AnimationPlayer::advance_frame`
allows action code to change the clip or playback flags before animation.

`BehaviorContext` implements original `sound_entity_init`, `sound_entity_stop`
and `sound_entity_update` semantics. It resets three command slots, invokes the
original callback, then emits stop/volume/play commands in order. The actor queue
is separate from generated proximity sounds and currently returns the missing
voice handle `-1`. Owned CPU comparisons use positive supplied handles to check
replacement and cleanup; queued commands prove no SPU playback.

Interaction policy retains signed proximity inputs, negative unlock height
offsets, random gates, timed interaction-range expiry, delayed link activation,
out-of-range stop, trigger notifications and negative-angle aim requests.
`EntityTarget` preserves PsyQ `ApplyMatrixLV` signed split-vector arithmetic,
view-height interpolation and inclusive cube bounds. Its facing-distance query
uses the original height band and strict Manhattan-XZ threshold. The scene
currently supplies the player's position/view and rotation matrix; complete
native transform ancestry and scheduling require further comparison.

The three callbacks preserve their original context writes and RNG use. Futon
chooses its sound on context tick zero; Television chooses its first sound with
the shared RNG; Drumstick uses behavior tick zero. Imported properties control
activation, so the idle calendar-day captures can leave these actors dormant.
Unimplemented callbacks keep frozen action/animation state while the shared
interaction timer and query policy run; that state is explicitly incomplete.

## Evidence

| Comparison | Scope/count |
| --- | ---: |
| Real owned entity lifecycle scenarios | 32 |
| Their constructor/start/stop/tick/overflow state observations | 544 |
| Interaction policy and service argument/order scenarios | 288 |
| Original target projection/height/cube cases | 720 |
| Canonical Futon/Television model and property bindings | 2 |
| Their controlled constructor/behavior/animation observations | 40 |
| Original distinct callback catalog | 107 |
| Entity property records / nonzero callback records | 130 / 111 |
| Named callbacks implemented and compared in supplied graphs | 3 |
| Whole NPC encounters accepted | 0 |

Native checks execute identified original functions with the real allocator,
MOM/MOS/TMD construction, SDK model binding and original callbacks. Owned roots,
already-initialized CD, external audio services, target coordinate state and,
where stated, a target-resolution or matrix service are supplied inputs. The
matrix cases use the explicit semantic COP2 bridge and do not accept hardware
flags/timing. Original function outputs are not replaced. Interaction comparisons
accept aim/notification requests and policy state; they do not accept the complete
consequence chain or aim rotation. See [behavior checks](entity-behavior-checks.json)
for identities, hashes, detailed counts and scope.

The older owned spawn model had offsets four bytes beyond its actual MOS/TMD
layout. Its coupled spawn observations covered selection, pose, order and counts,
and did not expose its malformed animation header. The input has been corrected
and regenerated through the original routines. All 161 spawn observations remain
identical. A new decoder regression requires the owned model to parse with two
animation frames, two part types and two empty primitives. Original imported
game files were unchanged; the earlier native proof remains identified in history.

```sh
cargo test -p lsd-assets --test native_entity_runtime --locked
cargo test -p lsd-core --test native_entity_target --locked
```

The additional original-model comparison requires `LSD_GAME_FILES` and private
CPU observations in `LSD_NATIVE_ENTITY_RUNTIME`, then runs with `--ignored`.
Original asset bytes, property rows, detailed original-model observations and
captures stay private. CI runs owned data independently of game files.

For a development capture that supplies player placement at the Futon query:

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- /path/to/extracted/files 0 0 A --npc-behavior-capture /path/to/new.png 32
```

This supplies a nearby player position after the first tick and exercises the
connected original query/script path. It does not establish a naturally played
encounter, original global order, GPU or audio parity.
