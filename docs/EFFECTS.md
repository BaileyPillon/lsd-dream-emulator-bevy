# Original effect lifecycle

Rust now implements original effect activation, both color-cycle starts,
notification-2 ticks, deactivation and completion-color selection. The controller
retains signed 32-bit life and step values, wrapping byte colors, channel masks,
inherited render flags, linked-effect life adjustment and mode-9 color suppression.

The start computes life as `256 / color_step`, followed by the linked adjustment
when enabled. A tick checks the previous life, so an unlinked increasing cycle
with step 10 completes on tick 26; step 30 completes on tick 9. The reducing
cycle reverses the color step. Completion restores its sign and sends event 5
or 6 after detaching from its render parent.

The original map callback detaches the effect, restores pause state for event 5,
or selects the stage color and notifies its parent for event 6. State 10 also
restores the view and requests an entity sound stop before notifying state 4.
Other map states preserve their original completion notification.

`MapEffect` consumes the inspector's fade requests and supplies its two parent
relationships. It processes effect ticks before the inspector's movement step;
this host order is a development choice pending full original-session scheduling
comparisons. Parent notifications are emitted for the future session consumer.
The controller does not invent scene reconstruction or clear input locks at fade
completion. Original stage replacement and sound playback remain unfinished.

The palette loader requires the canonical local `SLPS_015.56`. Original palettes,
images, private traces and game assets are excluded from this repository. Public
fixtures contain generated test colors and numeric observations from supplied
graphs. Unsupported division and palette accesses return errors rather than
substituting timing or colors; native fault handling is not accepted.

## Comparisons

- 128 original CPU sequences produce 3,600 compared effect states with generated
  colors, both start directions, four step values, normal/mode-9 ticks, variant
  reuse and linked/unlinked cycles.
- 24 additional tick cases check ignored notifications, inactive controllers,
  negative counters and signed wrapping. Twelve deactivation and three reset
  cases check flags, colors and callback order.
- 84 original map-completion cases check ignored codes, state preservation,
  color selection, pause/view restoration and parent notification order.
- Sixteen private sequences compare 536 states using colors imported from the
  original executable. Those palette bytes and state colors remain private.
- The inspector's supplied traversal captures remain identical to the previous
  checkpoint. Those paths reach no link and do not verify visible fades.

Native flag/color setters and the map pause/view routines execute directly.
Effect parent attachment/detachment/notification, stage color application and
entity sound stopping are supplied boundaries. Natural actor lists, GPU blend
mode, framebuffer precision, draw order, visible scene replacement, original
session cadence and end-to-end gameplay parity remain open.

See [effect-lifecycle-checks.json](effect-lifecycle-checks.json) for input identity,
function hashes, fixtures and validation. Whole original acceptance remains
**0/40 provisional feature groups**; the matching C decompilation is paused.
