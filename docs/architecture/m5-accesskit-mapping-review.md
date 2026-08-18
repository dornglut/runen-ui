# M5 AccessKit mapping review

Status: M5E conformance evidence, not RunenUI semantic authority.

## Review baseline

This review was refreshed for M5E on 2026-08-18 against the public AccessKit repository at exact upstream `main` commit `2dfdd7b92e68edd4276841a5061f31ffc77e718b` (2026-08-16). At that commit `accesskit/Cargo.toml` declares `accesskit` version `0.24.1`.

Source evidence:

- `https://github.com/AccessKit/accesskit/blob/2dfdd7b92e68edd4276841a5061f31ffc77e718b/accesskit/Cargo.toml`
- `https://github.com/AccessKit/accesskit/blob/2dfdd7b92e68edd4276841a5061f31ffc77e718b/accesskit/src/lib.rs`

The reviewed public model still consists of stable tree/node identities, `Node` role/properties/actions/relationships/bounds, `TreeUpdate` replacement-style partial node updates plus tree/focus facts, and `ActionRequest` carrying an action with explicit target tree and target node. `TreeUpdate` documents that an updated node replaces the previous node with the same ID, so unchanged fields of a changed node must be populated again. This matters to an adapter because RunenUI's semantic delta is not itself an AccessKit patch format.

This file records compatibility of concepts only. M5 adds no AccessKit/native dependency, platform bridge, AccessKit-named RunenUI public type, second semantic tree, or second action path.

## Mapping boundary

| RunenUI accepted authority | Adapter-local AccessKit projection | Boundary rule |
| --- | --- | --- |
| exact `SurfaceId` | one adapter-owned `TreeId` | The adapter owns the association. RunenUI does not expose or derive a platform tree identifier. |
| exact `SemanticNodeId` within a surface snapshot | stable adapter-owned `NodeId` | Never cast, serialize, or otherwise assume representation equivalence. Mapping is scoped by the exact RunenUI semantic lifetime and surface. |
| semantic roots and deterministic child ordering | AccessKit tree root and node children | RunenUI snapshot/tree order remains the source fact; the adapter only projects it. |
| `SemanticSnapshot` | initial/full `TreeUpdate` | Initial adapter publication materializes complete AccessKit nodes from one exact RunenUI snapshot. |
| `SemanticUpdate::Delta` | replacement entries in `TreeUpdate.nodes` plus affected parent/tree/focus facts | A changed AccessKit node is rebuilt completely because AccessKit replacement semantics do not preserve omitted fields. Removals are represented by rebuilding surviving parent/child facts; the adapter does not invent a RunenUI node-removal API. |
| full-resync result | fresh complete AccessKit tree update | Wrong surface/revision and any lost adapter mapping use RunenUI's accepted full-resync authority rather than guessing a patch. |
| semantic PRIMARY focus projection | AccessKit update focus target | RunenUI `FocusState`/semantic publication remains the sole focus authority. No AccessKit focus cache may retarget RunenUI state. |
| `SemanticRole` | best matching AccessKit `Role` | Mapping is adapter policy. Missing future role fidelity must not change RunenUI role vocabulary merely to mirror AccessKit. |
| semantic name/description/value | corresponding AccessKit textual node properties | RunenUI published content is copied; the adapter must not synthesize authoring truth back into RunenUI. |
| composed semantic state | corresponding AccessKit state properties where representable | Disabled/hidden/inert/focus and other accepted RunenUI facts remain authoritative even if a platform has different granularity. |
| supported `SemanticAction` set | AccessKit supported action flags where an accepted M5 action mapping exists | Advertise only actions that can round-trip into an accepted RunenUI semantic action. Unsupported platform actions are not advertised as working. |
| semantic relationships | AccessKit node-ID relationships | Relationship targets are mapped through the same exact semantic-ID table. Missing/stale mappings fail closed; no mounted-owner fallback. |
| published semantic bounds | AccessKit bounds | Use the exact published RunenUI geometry for that surface revision. Platform coordinate conversion belongs to the future adapter, not semantic authoring. |
| AccessKit `ActionRequest { action, target_tree, target_node, data }` | exact RunenUI `SemanticActionRequest { surface, target, action }` when M5 behavior exists | Resolve `TreeId` and `NodeId` through adapter-owned tables, then submit the ordinary public RunenUI surface-scoped semantic request. Never invoke a widget callback, mounted node, private owner map, or separate queue directly. |

## M5 action mapping

Only mappings backed by the accepted M5 vocabulary are valid at this boundary:

- AccessKit click/activation intent can map to RunenUI `SemanticAction::Activate`.
- AccessKit focus intent can map to RunenUI `SemanticAction::RequestFocus` when the current RunenUI node advertises/supports it.
- A platform context-menu intent can map to `SemanticAction::OpenContextMenu` where the target advertises it.
- A platform menu/open intent may map to `SemanticAction::OpenMenu` only where the adapter can make that meaning exact and the target advertises it.

AccessKit actions that require production text replacement/value editing, selection mutation, scrolling, numeric increment/decrement, or other semantics not present in the accepted M5 `SemanticAction` vocabulary are rejected/not advertised by an M5 adapter. They are not silent no-ops and do not justify compatibility wrappers. In particular, AccessKit scroll actions do not resurrect retired `SemanticAction::LogicalScroll`; semantic scrolling remains later-milestone work while routed M4 `SemanticCommand::LogicalScroll` remains ordinary non-semantic command behavior.

`ActionRequest.data` is consumed only when an accepted RunenUI semantic action has corresponding request data. M5's accepted actions carry no platform-specific action payload, so payload-dependent platform actions remain unsupported rather than being truncated.

## Identity, lifetime, and failure rules

The adapter must key reverse lookup by the exact platform tree/node identity it published and recover an exact current RunenUI `SurfaceId + SemanticNodeId`. A missing, retired, stale, foreign, or ambiguous mapping rejects. It must never fall back to a first node, mounted owner, authored ID, another surface, or a newly published semantic node.

RunenUI remains responsible for submission-time and processing-time semantic authority validation. The adapter does not cache enough widget/runtime state to reproduce those checks. Accepted platform action work therefore enters `AppRuntime::submit_semantic_action`/the corresponding public harness delegate and then follows the one canonical M4 queue, routed event/default, application update/reconciliation, and trace lineage.

## Dependency and authority conclusion

The current AccessKit public model remains compatible with an adapter over RunenUI's accepted M5 semantic snapshot/update/action boundary. Nothing in the reviewed model requires AccessKit to become a RunenUI dependency or semantic authority. A native accessibility adapter is therefore still a separate consumer/host milestone with its own platform lifecycle, threading, coordinate, and integration requirements.

For M5E the correct closure is documentation plus public-only conformance proof: retain one RunenUI semantic/action/runtime/trace authority, keep AccessKit at the edge, and reject unsupported later actions explicitly.
