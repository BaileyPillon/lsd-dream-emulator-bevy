# Entity body routing and synchronous acknowledgement

The port now retains the original entity body-notification gates and contact
state. Bound journal acknowledgements can deliver their link, video or dream-end
consequence before the call returns. Inherited geometry queries remain explicit
pending requests; this does not accept complete NPC collision or encounters.

## Recovered routing

`BodyPeer` replaces a native pointer with a typed identity. `EntityBodyPlan`
preserves the original transform/scene/entity wrapper order for body senders.

| Sender/event | Original consequence |
| --- | --- |
| Low byte `0x34`, codes 2–8 | Apply inherited body handling only when the signed link-stage property is positive |
| Low byte `0x34`, code 4 with positive stage | Store contact, then emit link 10 or stage-127 video/end 11/12 |
| Low byte `0x24`, code 4 | Store contact, stop behavior/hide parts and disable the link |
| Accepted codes 2/3 | Enter inherited geometry query |
| Accepted `0x34` codes 5–8 | Enter inherited geometry query |
| Other body type bytes | Leave entity body state unchanged |

The stage-127 branch selects video when the signed video property is nonzero;
otherwise it selects dream end. Other positive stages select linking. Existing
animation/context clocks and sound stop order remain intact. The body API is
scoped to these senders; controller, destruction and complete scene/lifetime
messages remain separate requirements.

The inherited DreamSys journal handler executes before sender filtering and the
link-state guard. Code four stores the sender even when those guards reject the
entity-specific effect. The earlier journal adapter treated that inherited
handler as a no-op. Its snapshots excluded the DreamSys contact field and accepted
acknowledgement request routing only. New coupled snapshots measure both contacts
and establish the corrected mutation and recursive consequence order.

`EncounterEventHost::dispatch_actor` supplies the emitting actor subject.
The dispatcher acknowledges that actor through the DreamSys `0x1F34` route. A
permitted contact can return notification 10/11/12, which the core journal handles
immediately with the same day/link/RNG state. Calls without an actor subject keep
an explicit acknowledgement request. The map adapter checks include bound code
four, its fade request and cancellation after contact storage.

## Evidence

There are 2,016 original actor/transform/scene/entity body-notification scenarios,
using 4,032 real owned entity constructions and valid MOM decoding. They vary
sender types, signed stage/video properties, event widths, active state, prior
contacts and live sound handles. Native geometry-query entry occurs in 288 cases.
The owned primitives are empty: these checks accept query routing and policy,
not geometry hits or collision output.

An additional 336 cases use a real original linked-list parent subscription to
deliver actor notifications back to the original DreamSys journal. Original
construction, getters, contact handlers, spawn/link selection, mood/recall and
video/end dependencies execute without replacing original function outputs.
Typed roots, initialized dream state and external key/notification/audio services
are supplied. The Rust comparison now includes actor acknowledgement state and
DreamSys contact storage alongside state, RNG, spawn, score, recall and request
order. The older unbound journal comparison remains identified separately.

```sh
cargo test -p lsd-assets --test native_entity_body --test native_actor_journal_body --test actor_journal_integration --locked
```

See [body checks](entity-body-checks.json) for fixture/function/source identities
and validation limits. CI uses owned inputs. Original asset bytes, table rows,
canonical observations and captures stay private. Whole-game feature acceptance
remains 0/40 provisional groups; original whole NPC encounters remain unaccepted.

## Remaining requirements

Execute real inherited body queries with imported model/root geometry, prepared
collision clouds, transform ancestry and live contact feedback. Implement original
movement correction, ground reconciliation, aim and natural scene ownership/
lifetime/cadence. The Yokai development capture now exposes 48 pending inherited
query requests over its 63 active ticks; its rendered motion is unchanged from
the previous checkpoint. Request counts do not establish physics completion.
Compare original GPU/SPU and complete natural encounters. The remaining 95
callbacks and the other full-game requirements remain open. The matching C
checkout stays paused and preserved; default gameplay makes no AI model calls.
