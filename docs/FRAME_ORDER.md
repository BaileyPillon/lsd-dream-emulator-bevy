# Original frame graph and pad phase

The original display timer advances its shared clock before polling and
dispatching the pad. Player callbacks therefore consume the control state from
the preceding poll. The linked Rust scene previously dispatched fresh input
before its view and movement callbacks. It now advances those callbacks first
and polls afterward.

`LinkedPlayer::tick_clock` exposes the player clock handler separately from
`poll_pad`. Waiting and finished signals leave player clocks and callbacks
unchanged. The surrounding display tick can still poll input. `begin_frame`
clears adapter reports before a future shared traversal, allowing earlier
subscribers' commands to survive the player phase. The combined `step` and
`step_with_clock` methods use the corrected order.

## Original emulator observation

A read-only BizHawk 2.11.1/Nymashock run loads a saved dream from the canonical
Japanese disc. Neutral emulator-local input advances 120 emulator frames and
records 121 clock graph samples. No original RAM is written or patched. All 15
launch input records are rehashed after execution. Eighteen relevant routines in
the captured initial and final RAM match canonical executable bytes.

The same ten-member list appears in every sample:

| Order | Role | Original clock handler |
| --- | --- | --- |
| 1–5 | Resident actors | `entity_update` |
| 6 | Map | `func_800533F0` |
| 7 | Stream context | `func_8004AB24` |
| 8 | Renderer | `func_8003EE40` |
| 9 | Dream session counter | `scene_update` |
| 10 | Player | `dream_sys_timer_tick` |

The actor ids are 91, 77, 77, 90 and 5. The map clock handler updates generation
through its active callback. The original display-timer routine dispatches pad
input after the entire clock traversal returns.

The clock advances from 386 to 406 and the player timer from 384 to 404 during
these 120 emulator frames. A nonzero cursor appears in 101 samples, pointing to
the session as the next subscriber; 20 samples have a completed cursor. An
emulator frame boundary can occur while the clock traversal is still in progress.
These are observed RAM graphs, not instruction traces or a portable frame-rate
acceptance test.

## Coupled native comparison

120 owned scenarios execute the original display-timer routine, clock, real TMD
and DreamSys constructors, player view callbacks, pad constructor, pad poll,
notification dispatch and SDK `PadRead`/`PAD_dr`. They compare 3,840 display
events before and after pad polling. Four movement directions, six pause/lock
states and five input patterns include supplied waiting/finished flags and
display signals that do not trigger a tick.

The external BIOS controller boundary receives an initialized SDK input buffer.
Original game function outputs are not replaced. Movement geometry is disabled
in this suite; full movement, body and link comparisons remain separate. A
linked-wall regression now checks that the first two forward-input clocks have
no contact and the third performs the original movement/link path.

```sh
cargo test -p lsd-core --test native_frame_pad_order --test linked_player --locked
LSD_GAME_FILES=/path/to/local/files cargo test -p lsd-assets \
  --test native_linked_player --locked -- --ignored
```

See [frame-order checks](frame-order-checks.json) for hashes, exact scopes and
validation. Five development captures are checked; the walking capture changes
with the corrected input phase. Original images, RAM and disc data stay private.

## Remaining work

The port still needs the entire observed graph connected to one shared clock.
Its loaded inspector currently places player work before actor/generation work,
binds new actors during the encounter tick, and supplies simplified asynchronous
stream completion. Recover original birth timing, session handoff, CD waiting,
renderer/cache refresh, reciprocal movement response and resource destruction.

Rendering/audio fidelity, the remaining 95 NPC callbacks, menus, persistence,
media and endings remain incomplete. This change accepts no whole feature group
or natural NPC encounter. Original gameplay continues to make no AI model calls,
and C decompilation work remains paused.
