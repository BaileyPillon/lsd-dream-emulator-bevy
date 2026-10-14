# Live clock traversal

The original frame clock holds a cursor into its subscriber list. It advances
that cursor before invoking each subscriber. If a callback removes the next
subscriber, `frame_phase_remove_parent` moves the cursor past it before freeing
the list node. Removing a later subscriber changes the remaining chain. New
subscriptions prepend to the list and wait until the following notification.
A restart removes the old subscription and prepends its replacement.

The Rust `FrameSubscribers` implements this behavior. It retains stable node
positions during a traversal, allows the original base list's duplicate entries,
and handles removals of current, next, later, visited and absent subscribers.
`EncounterScene` now uses the live cursor instead of cloning its member list.
Finished actors disable their behavior, detach and retire individually during
notification. Host errors clear the cursor before returning.

The notification code is selected before traversal. Setting waiting or finished
inside a callback changes the next clock advance, while every remaining callback
in the current notification receives the already selected code. Reset cancels
the cursor as well as clearing flags. The clock and subscriber list are separate
Rust types; callers resetting during dispatch must cancel that cursor too.

## Original-code comparisons

88 scenarios execute the unmodified original allocator, base constructors,
attach/detach/destructor, frame clock and list traversal routines. Six owned base
objects provide callback programs that call those original mutation routines.
The programs are supplied MIPS code, not original NPC scripts. They are observed
at entry without replacing original function outputs.

Each scenario compares two notifications, giving 176 notifications and 698
callback observations. Comparisons include each callback's subscriber, code and
next cursor value, the remaining membership order, counter, flags and final
cursor clearance. Actions cover removal, restart, duplicate subscription,
self destruction, reset, waiting and finish. The unchanged three-actor native
suite also compares 32 scenarios and 416 states through real original entity
callbacks and animation updates.

```sh
cargo test -p lsd-core --test native_frame_cursor --locked
cargo test -p lsd-assets --test native_actor_clock --locked
```

See [cursor checks](frame-cursor-checks.json) for hashes and validation results.
Public fixtures contain owned input and scalar observations. Original executable
bytes, assets and captures remain private.

## Remaining work

This recovers clock list mechanics and connects them to the actor channel. The
shared player/map/generation/renderer/CD graph, its attachment times, waiting
synchronization, natural transform caches and full resource lifetimes still need
recovery. Original NPC callbacks do not yet request other actors' destruction,
restart or birth through this adapter. Clear-all during a native callback, native
allocation failure and nested notifications are outside these comparisons.

The port still has 95 pending distinct NPC callbacks. Full rendering, audio,
menus, persistence, media and endings remain incomplete. No whole feature group
or natural NPC encounter is accepted by these tests. The original experience
uses no AI model calls, and the C decompilation remains paused and preserved.
