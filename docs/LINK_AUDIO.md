# Link and journal sound callbacks

Accepted link effects now call the shared scene sound object during their
original callback. The player collision/link adapter and actor journal adapter
retain their request traces for inspection; those traces are not replay queues.
Generation, NPC and link requests use the same bank, voice pool, pitch offset,
volume context and SPU.

`execute_link` first sends its notification. If that notification cancels the
link, the link's sound is skipped. Otherwise it commits the destination, resets
the recall timer when required, then requests packed note `0x90` with volume
arguments `110, 110`. A note started by notification handling already owns its
voice before the link's note allocates the next one. Cancelling the link does
not undo a note that the notification already started.

`LinkedPlayer::tick_clock_with_sound` accepts borrowed clock inputs and a
`GenerationSound` backend. Its movement, wall and floor handlers dispatch sound
where the effect occurs. Deterministic player state owns no audio thread or
device. `StageLinkSession` supplies the connected shared backend to player and
journal callbacks. Legacy silent calls retain their existing reports.

## Sound-driver initialization correction

The original `func_8002EDD4` invokes the SDK update once during initialization.
The live backend previously started with the raw empty driver, omitting that
update. Its history index and free-voice history therefore differed before any
note was requested. `SceneAudio` now performs this update during construction.
The diagnostic `startup_sdk_ticks` counter records it separately from callbacks
after sample production begins.

Fresh original-code comparisons match all software voice records, cached
registers, dirty flags, automation/reclaim history, pending key/reverb masks,
shared volume context, and history index before and after the supplied calls.
The previously checked two-second PCM diagnostic remains a regression gate.
Recovering this initialization update does not establish periodic interrupt
timing: the live prototype still supplies one callback per 735 samples.

## Evidence and limits

| Comparison | Cases |
| --- | ---: |
| Owned bank, original link/journal/sound/SDK chain | 32 |
| Local original SE bank through the same original chain | 32 |
| Bound actor journal adapter, accepted/cancelled acknowledgement and direct events | 4 |
| Player collision/wall adapter with immediate sound dispatch | 1 |

The native scenarios vary direct link versus journal entry, cancellation, a
note played during notification handling, recall mode and pitch offset. The
original journal, link, sound-object, key-on and auto-volume routines execute
from the identified local executable. Each SDK initialization is checked to
invoke exactly one original update. Full recordings repeat in a fresh process.

Notification handling is an explicitly supplied owned service that accepts or
cancels and can play a note through the original sound object. It is not a
recording of a natural map/NPC encounter. No original sound-object or SDK output
is stubbed. Tests compare notification/sound/return order and the complete
software driver state. They do not accept original CD loading, SPU bus timing,
host buffering, a full sound scene or whole-game parity.

The owned fixture contains synthetic bank metadata and bounded numeric
observations. Original bank data and detailed original-bank observations remain
private. See [link-audio-checks.json](link-audio-checks.json) for identities,
hashes, checks and remaining requirements.

```sh
cargo test -p lsd-assets --test link_audio --locked
cargo test -p lsd-assets --test actor_journal_integration --locked
cargo test -p lsd-core --test linked_player --locked
```

Music sequencing, real bank allocation/loading and mute lifecycle, original
interrupt cadence, natural encounter comparisons and the remaining game
systems still need implementation and verification. Original behavior remains
separate from optional AI analysis and gameplay mods.
