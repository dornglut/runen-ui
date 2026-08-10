# M5 Semantics and Deterministic Testing Charter

> **Category: Target architecture**
>
> **Status:** Review candidate for owner acceptance
>
> **Milestone:** M5
>
> **Accepted implementation base:** `a63a249de9d4d53eeef4104ae3384e7898aacad1`

This charter turns the M5 roadmap goal into decision-complete implementation
boundaries. It does not claim that M5 behavior exists. The
[M5 conformance matrix](m5-conformance-matrix.md) owns M5-specific observable
acceptance. The accepted
[M4 conformance matrix](m4-conformance-matrix.md) continues to own inherited
`ACCESS-01` and `ACCESS-02` until M5C completes and accepts them.

The M4 command, event, scheduler, trace, export, and replay authorities remain
inputs. M5 must not create replacements for them.

## Delivery sequence

M5 is deliberately sequential:

```text
M5A0  architecture/conformance authority + matrix-audit tooling
  -> M5A  semantic contribution and independent semantic identity
    -> M5B  semantic tree publication and incremental updates
      -> M5C  semantic action ingress and accessibility resolution
        -> M5D  public deterministic headless testing harness
          -> M5E  integrated conformance, migration, and M5 closure
```

The public execution issues are #46 through #51 under umbrella issue #45.
Every implementation branch begins from the accepted `main` produced by the
preceding accepted slice. No M5 implementation branch may be stacked on an
unmerged predecessor.

M5A0 changes architecture/conformance authority and the minimum repository-audit
tooling needed to validate multiple milestone matrices. It changes no
`runenui_core` or `runenui_runtime` behavior.

## Inherited implementation facts

M4 leaves useful semantic infrastructure but not a production semantic system:

- `WidgetSemanticProof` is a flat M2 proof capability on `Widget`;
- mounted nodes have a cached semantic capability and a semantic dirty phase;
- `SemanticNodeId` is currently generated from the mounted arena's slot and
  generation;
- semantic proof data is embedded in renderer-facing `SurfaceNode`/
  `SurfaceFrame` and debug rendering;
- public `AppRuntime` already owns deterministic mounting, canonical ingress,
  bounded pumping, deterministic time, inspection, surface publication, trace,
  export, and offline replay;
- the only inherited blocked M4 matrix rows are M5-owned `ACCESS-01` and
  `ACCESS-02`.

M5 preserves the useful scheduling/invalidating authority and replaces the
proof-only semantic payload, identity coupling, publication ownership, and test
surface.

## Authority principles

M5 follows six hard boundaries:

1. **Semantic truth is not renderer truth.** Semantic tree, layout, hit-test,
   paint, style, and diagnostics are distinct authoritative products.
2. **Runtime IDs are runtime authority.** Widgets author stable local semantic
   keys, never live `SemanticNodeId` values.
3. **Mounted ownership is not semantic identity.** One mounted lifetime may own
   zero, one, or many semantic lifetimes.
4. **Accessibility actions are command ingress.** They resolve to the exact
   mounted owner and enter the accepted M4 command queue; they never activate a
   widget directly.
5. **Testing is downstream ergonomics.** The public testing crate may compose
   public runtime APIs but receives no private mutation authority.
6. **Platform adapters are adapters.** AccessKit or native accessibility types
   never become core/runtime semantic vocabulary.

## Core semantic vocabulary

`runenui_core` owns the platform-neutral semantic description vocabulary. The
exact public names may be refined during M5A only if they preserve these
responsibilities:

- `SemanticKey`: stable owner-local authored identity for one semantic node;
- `SemanticContribution`: one widget's action-type-independent semantic forest;
- `SemanticNodeContribution`: one owner-local node description;
- `SemanticRole`: non-platform semantic role vocabulary;
- names and descriptions as owned Unicode text;
- values and typed states needed by actual M5 semantics;
- `SemanticRelationship` and `SemanticReference`;
- `SemanticAction`: only actions with real RunenUI behavior in the accepted
  milestone;
- semantic bounds policy using canonical logical geometry;
- plain-text semantic content with an extension boundary that can grow into the
  M8 text-range model without giving M5 fake editing behavior.

The vocabulary is `#[non_exhaustive]` where later controls/text legitimately add
variants. M5 does not add placeholder actions that silently do nothing.

### Roles and later controls

M5 implements the roles required by built-ins, Counter, and genuine downstream
custom-widget proofs. Later M8/M9 controls may add roles without changing
semantic identity, tree ownership, action ingress, or adapter boundaries.

### Values and text

M5 may publish real read-only semantic values and plain text. Production text
selection/ranges/editing remain M8. The M5 node model must allow a later text
extension without replacing `SemanticNodeId` or the semantic tree/update
protocol.

## Owner-local semantic contribution

A compatible mounted widget contributes **0..N** semantic nodes. Contribution is
read-only with respect to application/runtime behavior and independent of the
widget's `Action` type.

Each contributed node has one `SemanticKey` unique within its exact mounted
owner. Runtime validates the complete contribution before accepting it as the
owner's semantic description.

A contribution is an owner-local forest. It may designate one explicit splice
point at which the semantic forests of mounted child elements are inserted.
This prevents renderer/mounted topology from being copied blindly while still
allowing transparent wrappers and virtual semantic descendants.

Required composition behavior:

- an owner with no semantic roots is transparent; mounted child semantic roots
  splice into the nearest semantic ancestor;
- an explicit mounted-child splice point fixes parent and ordering; no implicit
  first/last semantic root is selected;
- duplicate `SemanticKey` values or structurally invalid splice references are
  rejected deterministically and diagnosed;
- invalid owner contribution never causes first/last-match recovery;
- recursive component action mapping leaves semantic contribution unchanged.

The contribution contract does not expose mounted arena access, runtime focus,
absolute surface coordinates, semantic arena allocation, or platform adapter
objects.

## Semantic identity model

`SemanticNodeId` remains an opaque core-owned runtime-local ID sharing the M4
runtime namespace, but M5 removes its mounted-arena allocation coupling.

Runtime owns a separate checked generational semantic arena/index. Conceptually
one live record binds:

```text
SemanticNodeId
  -> exact mounted owner lifetime
  -> owner-local SemanticKey
  -> accepted semantic contribution node
```

Identity rules are exact:

- runtime issues IDs; downstream code cannot construct, decompose, serialize
  into live authority, or extract the runtime namespace;
- owner lifetime + `SemanticKey` retention preserves the exact semantic ID across
  compatible updates and contribution reorder;
- removing a local key revokes that semantic lifetime;
- removing/replacing a mounted owner revokes every semantic lifetime it owns;
- later reuse of an arena slot uses a later non-wrapping generation;
- an old ID is stale and never retargets a replacement;
- a foreign runtime ID is foreign even if its opaque internal slot/generation
  values would otherwise coincide;
- public-slot conversion and generation advancement are checked, never truncated
  or wrapped.

Canonical runtime-issued IDs are unique. `ACCESS-02`'s ambiguous-identity branch
therefore represents semantic-index integrity failure, not an ordinary authored
state. An integrity failure rejects without selecting first or last.

## Logical geometry ownership

M5 semantics need bounds independent of renderer output and virtual semantic
nodes may need sub-bounds inside one mounted widget.

`LogicalPoint` and `LogicalLength` are already core-owned. M5A cleanly moves
canonical `LogicalSize` and `LogicalRect` ownership to `runenui_core`; runtime may
re-export those same canonical types where its public APIs require them.

A semantic node's bounds policy is one of:

- the exact arranged owner bounds; or
- a validated owner-local logical rectangle translated into publication
  coordinates by runtime.

A widget does not author absolute surface coordinates.

Layout changes mark semantic publication/bounds dirty even if the cached widget
semantic contribution remains valid. Runtime recomposes absolute bounds without
calling an unchanged semantic contribution again. `WidgetInvalidation::SEMANTICS`
continues to mean that the contribution itself changed.

M6 may later add transforms/clips/scene geometry. That work must refine mapping
without merging semantic and hit/paint authority.

## Semantic states and runtime-derived facts

The semantic contribution owns semantic facts derived from widget state; runtime
owns runtime facts.

Runtime-derived facts include:

- live `SemanticNodeId`;
- exact mounted owner;
- absolute logical bounds;
- exact runtime focus;
- publication/update revision;
- semantic-action target resolution.

Required state policy:

- **disabled:** remains observable with disabled state; an unavailable action
  cannot execute;
- **hidden:** absent from the published semantic tree and action surface; a live
  owner/local-key record may be retained so reappearance can preserve identity;
- **inert:** may remain observable but exposes no executable action and action
  ingress rejects it;
- **focused:** derived from exact mounted runtime focus, never authored as a
  contradictory widget fact.

Later states may extend this model without changing identity/lifetime authority.

## Relationships

M5 supports real semantic relationships without exposing live runtime IDs to
widget authoring.

A relationship target is either:

- another `SemanticKey` within the same mounted owner; or
- a unique authored `ElementId` plus an optional owner-local `SemanticKey` in
  that target owner.

Runtime resolves relationships only after semantic identities for the current
product are known.

Missing or ambiguous authored element references, missing local semantic keys,
hidden targets, and stale owner transitions produce deterministic semantic
diagnostics. They never pick a first/last candidate or fabricate a replacement
relationship.

## Independent semantic publication

M5B removes production semantic authority from `SurfaceNode`, `SurfaceFrame`, and
`DebugSurfaceRenderer`.

The public surface publication may carry semantics as a **sibling product** so
one publication can align logical bounds and displayed state, but the semantic
snapshot is independently typed and consumable. A renderer can consume the
frame without semantic vocabulary; an accessibility/test consumer can consume
semantics without interpreting paint proof kinds.

The semantic snapshot exposes deterministic tree order and read-only lookup by
exact `SemanticNodeId`. It contains the complete current semantic product needed
by an adapter or test query.

## Incremental semantic updates

M5B owns one non-wrapping semantic publication revision authority.

Each accepted semantic change produces a deterministic update from the exact
previous revision to the next revision. Updates can represent:

- added nodes;
- changed node content/states/actions;
- removed semantic IDs;
- parent/child/root changes;
- relationship changes;
- runtime focus changes;
- logical bounds changes.

An unchanged publication retains its semantic revision and reports no fabricated
update.

An update is not an independent mutable semantic store. It is a delta derived
from the runtime-owned current semantic product. Consumers that apply a delta to
the wrong prior revision have invalid state and must resynchronize from a full
snapshot.

This model is intentionally compatible with adapters that accept atomic partial
tree updates while remaining RunenUI-owned.

## Semantic diagnostics

Semantic contribution/tree/relationship integrity diagnostics are typed,
deterministic, and independent from paint/debug strings.

At minimum diagnostics distinguish:

- duplicate owner-local semantic key;
- invalid mounted-child splice reference;
- missing local relationship target;
- missing cross-owner authored target;
- ambiguous cross-owner authored target;
- missing cross-owner semantic key;
- semantic index integrity ambiguity.

Diagnostics never become fallback routing authority.

## Semantic actions and M4 convergence

M5C introduces a public semantic-action request targeted by exact
`SemanticNodeId`.

Submission performs no callback. It:

1. validates runtime namespace and semantic-index integrity;
2. classifies the target live/stale/missing/foreign;
3. checks current published hidden/inert/disabled/action-support policy;
4. resolves the semantic record's exact live mounted owner;
5. maps the accepted `SemanticAction` to an existing `SemanticCommand`;
6. uses `CommandOrigin::accessibility()`;
7. submits through the accepted canonical command preflight/FIFO/wake/routed
   transaction authority.

M5 semantic action vocabulary contains only commands with real accepted
behavior. Expected initial mappings include activation, focus request,
menu/context-menu, and logical scrolling where semantically valid. M8/M9 own
value mutation and text editing actions.

Rejection returns the exact owned request and consumes no work/trace sequence,
invokes no callback, mutates no state/focus, and requests no wake.

Required rejection classes include semantic identity status, integrity
ambiguity, hidden/inert/disabled/action-support failures, exact mounted-owner
status/integrity, canonical queue/status, and sequence exhaustion.

## Trace and replay integration

There remains one canonical trace.

M5C extends it with semantic action resolution/admission/rejection facts needed
to prove:

```text
semantic request
  -> exact SemanticNodeId
  -> exact mounted owner
  -> canonical SemanticCommand
  -> existing routed/default/update/reconciliation lineage
```

M4D2 bounded retention, redaction, and external-sink guarantees remain intact.
M4D3 replay remains inert offline causal proof. M5 testing may consume replay;
replay never submits live semantic actions or reconstructs runtime authority.

## AccessKit-neutral adapter foundation

RunenUI does not adopt AccessKit as core vocabulary in M5.

The M5 tree/update/action model is designed so a later adapter can map to the
current AccessKit concepts of stable node identity, tree/root/focus state,
partial atomic tree updates, node properties/bounds/actions, and targeted action
requests. M5E records an explicit source-grounded mapping review against the
then-current AccessKit API.

Primary references used during the readiness audit include:

- <https://docs.rs/accesskit/latest/accesskit/struct.TreeUpdate.html>
- <https://docs.rs/accesskit/latest/accesskit/>
- <https://developer.android.com/reference/androidx/customview/widget/ExploreByTouchHelper>

These sources support stable accessibility node identity, incremental updates,
targeted actions, and virtual logical hierarchies. They do not define RunenUI's
public types.

No AccessKit dependency or native platform bridge is required for M5A-M5D.
M10 owns native host/accessibility adapter integration.

## Public deterministic testing crate

M5D adds a genuine downstream workspace crate, `runenui_testing`, after M5C
stabilizes semantic query/action APIs.

The crate depends only on public `runenui_core` and `runenui_runtime` behavior.
It must not:

- enable `internal-test-seams`;
- call doc-hidden runtime construction bridges;
- seed runtime sequences/generations;
- corrupt mounted state;
- replace surface snapshots;
- invoke widget callbacks directly;
- maintain a parallel expected runtime/semantic state.

### Harness responsibilities

A typed harness owns a public `AppRuntime<App>` and deterministic publication
configuration. It provides ergonomic delegation for:

- mounting/configuration;
- bounded pump;
- deterministic time advancement;
- explicit publication;
- semantic queries;
- public semantic actions;
- pointer, keyboard, committed text, composition, automation, and direct action
  submission;
- normalized controller commands through existing `SemanticCommand` plus
  `CommandOrigin::controller()`;
- read-only state/focus/reconciliation/semantic/layout/hit/current-paint/trace/
  replay inspection.

### Query policy

Semantic queries return deterministic match sets or structured unique-match
results. A unique lookup distinguishes missing from ambiguous. Mutation helpers
never choose first/last from an ambiguous result.

### Pump/settle policy

`pump` is explicit and takes public `PumpBudget`.

A convenience settle operation is allowed only if it takes an explicit bounded
settle budget and returns a structured quiescent/exhausted/closed/terminal
outcome. No unbounded hidden loop is permitted.

### Assertion style

Prefer typed data/query APIs and ordinary Rust assertions. M5 does not add a
macro-heavy expectation DSL or a snapshot/golden framework.

M6 will later replace current proof-level hit/paint products; the testing crate
may adapt cleanly because it does not own those products.

## Deferred architecture issues

### Issue #10 — Element/Widget concentration

Not an M5 prerequisite. M5A creates a focused core semantic module and removes
semantic type ownership from `element.rs` rather than broadly reorganizing the
whole public protocol during a semantic-contract cutover. #10 remains a later
complete coupling/responsibility audit.

### Issue #12 — widget event-output capacity

Resolved after M4 with outcome 1: current capacity is sufficient for accepted M5
requirements. Semantic contribution is observational and semantic actions are
new ingress into the existing command authority, not new widget outputs.

## M5 conformance ownership

The [M5 conformance matrix](m5-conformance-matrix.md) uses the repository's
accepted status vocabulary:

- `blocked`;
- `implementation-complete`;
- `proof-complete`;
- `owner-accepted`.

M5A0 owns no behavioral rows; the initial M5 matrix therefore begins with every
M5 behavior row `blocked`. This avoids inventing feature acceptance for the
documentation/tooling gate.

The inherited M4 `ACCESS-01` and `ACCESS-02` rows remain in the M4 matrix and are
completed by M5C. No duplicate IDs are created in the M5 matrix.

## Slice boundaries

### M5A — contribution and independent identity

Owns core semantic vocabulary, logical size/rect core cutover, contribution,
semantic arena/index, identity lifecycle, and proof-semantic callback migration.
It does not publish the complete production semantic tree or accept semantic
actions.

### M5B — semantic product and incremental updates

Owns semantic tree composition, relationships, runtime-derived state/bounds,
separate publication, semantic diagnostics, revisions/diffs, and removal of
semantic authority from renderer-facing frame/debug products.

### M5C — semantic actions and accessibility resolution

Owns semantic-action request/result/error API, exact semantic->mounted resolution,
canonical command convergence, semantic action trace, and inherited
`ACCESS-01`/`ACCESS-02` behavior/proofs.

### M5D — public deterministic testing

Owns `runenui_testing`, bounded harness ergonomics, semantic queries, public
interaction helpers, deterministic settle, assertions, and replay integration.

### M5E — integrated closure

Owns cross-source conformance, AccessKit-neutral mapping review, final migration
audit, current-document truth, complete M5 matrix proof, and M5 closure. It does
not implement M6.

## Migration policy

RunenUI is pre-1.0. M5 uses a clean cutover:

- no compatibility alias for `WidgetSemanticProof`;
- no second old/new semantic callback;
- no retained production `SurfaceNode::semantics()` after its assigned removal;
- no semantic ID compatibility coupling to mounted arena allocation;
- no direct semantic activation path;
- no parallel accessibility queue or trace;
- no public testing API that preserves private/internal test seams.

Repository authority audit must enforce the final retired-surface rules by M5E.

## Stop conditions

Stop a slice instead of widening it when:

- accepted `main` drifts with overlapping semantic/testing work;
- semantic truth would be merged into renderer/hit/layout authority;
- semantic actions would bypass canonical M4 command admission;
- a public testing requirement needs private mutation authority;
- implementation needs a native accessibility bridge, production text editing,
  M6 scenes, or broad M7+ behavior;
- a new production crate has no independent ownership/consumer pressure;
- an accepted row cannot be proven without changing the charter/matrix first.

## M5 exit gate

M5 closes only after:

- all M5-specific required matrix rows are `owner-accepted`;
- inherited M4 `ACCESS-01` and `ACCESS-02` are `owner-accepted`;
- stable and Rust 1.93.0 validation pass at the exact reviewed closure head;
- exact-head CI proves checkout of that head;
- current public API/status/support/roadmap truth is reconciled;
- retired M2 semantic authority and private testing bypasses are absent;
- complete critical review finds no unresolved defect;
- guarded merge and accepted-main content identity are verified;
- any required non-circular post-merge acceptance reconciliation is accepted.

Only the exact accepted M5 closure base may activate M6.