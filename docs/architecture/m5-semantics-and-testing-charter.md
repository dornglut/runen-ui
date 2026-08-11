# M5 Semantics and Deterministic Testing Charter

> **Category: Target architecture**
>
> **Status:** Accepted
>
> **Accepted by repository owner:** 2026-08-10
>
> **Milestone:** M5

This charter owns the durable M5 implementation boundaries. It does not claim
that blocked M5 behavior exists. The
[M5 conformance matrix](m5-conformance-matrix.md) owns exact observable
acceptance, while the accepted
[M4 conformance matrix](m4-conformance-matrix.md) continues to own inherited
`ACCESS-01` and `ACCESS-02` until M5C accepts them.

The M4 command, event, scheduler, trace, export, replay, focus, and mounted-tree
authorities remain inputs. M5 extends them; it does not replace them.

## Delivery sequence

M5 implementation remains deliberately sequential:

```text
M5A0  architecture/conformance authority + matrix-audit tooling
  -> M5A  semantic contribution and independent semantic identity
    -> M5B  semantic tree publication and incremental updates
      -> M5C  semantic action ingress and accessibility resolution
        -> M5D  public deterministic headless testing harness
          -> M5E  integrated conformance, migration, and M5 closure
```

The public implementation issues are #46 through #51 under umbrella issue #45.
Every implementation branch begins from accepted `main` produced by the
preceding accepted slice. No implementation branch may stack on an unmerged
predecessor or a pending required authority reconciliation/readiness amendment.

The post-M5A readiness gate #55 is not an additional implementation slice. It
freezes the successor contract before M5B/M5C source work and performs one
bounded pre-1.0 vocabulary correction: route-bound semantic LogicalScroll is
removed from semantic authoring while accepted routed M4 scrolling remains.

## Accepted M5A baseline

M5A is accepted and reconciled. The accepted baseline now provides:

- platform-neutral `SemanticContribution` with zero or more owner-local semantic
  nodes per mounted owner;
- stable owner-local `SemanticKey`, including reserved `SemanticKey::PRIMARY`;
- strict duplicate-key, relationship, and mounted-children-marker validation;
- canonical core-owned `LogicalSize` and `LogicalRect` plus validated owner-local
  semantic bounds;
- an independent checked generational semantic arena and owner/key binding
  store;
- stable identity retention across compatible updates/reordering and exact
  stale/revoked/foreign behavior;
- fail-closed semantic withdrawal on invalid contribution, identity exhaustion,
  or semantic-index integrity failure.

M5A does **not** provide the independent M5B semantic publication product,
semantic-node action ingress, native accessibility integration, or the public
M5D testing crate.

## Authority principles

M5 follows these hard boundaries:

1. **Semantic truth is not renderer truth.** Semantic tree, layout, hit-test,
   paint, style, and diagnostics are distinct products.
2. **Runtime IDs are runtime authority.** Widgets author stable local semantic
   keys, never live `SemanticNodeId` values.
3. **Mounted ownership is not semantic identity.** One mounted lifetime may own
   zero, one, or many semantic lifetimes.
4. **Mounted focus is the focus authority.** Semantic focus is a projection of
   current mounted focus, never a competing focus model.
5. **Accessibility actions are command ingress.** They resolve exact current
   semantic authority and converge on the accepted M4 queue/routed/default path.
6. **Testing is downstream ergonomics.** Public testing composes public runtime
   APIs and receives no private mutation authority.
7. **Platform adapters are adapters.** AccessKit/native accessibility types do
   not become core/runtime semantic vocabulary.
8. **Publication is atomic.** A new renderer/input/semantic product is exposed
   only after its required capacities and staged state can commit coherently.

## Core semantic vocabulary

`runenui_core` owns platform-neutral semantic description vocabulary:

- `SemanticKey`;
- `SemanticContribution` and `SemanticNodeContribution`;
- `SemanticRole`;
- names, descriptions, values, state, relationships, bounds, and plain text;
- `SemanticAction` only for actions with real M5 RunenUI semantics.

The M5 semantic action vocabulary is exactly:

```text
Activate
RequestFocus
OpenMenu
OpenContextMenu
```

Route/device/session-specific scrolling is not semantic-authoring authority.
`SemanticCommand::LogicalScroll`, `LogicalScrollCommand`, pointer-derived scroll,
focus logical-scroll derivation, routed callbacks, and accepted M4 scrolling
remain unchanged. M5 defines no compatibility semantic-scroll alias and invents
no fake pointer identity. M7 may introduce device-independent semantic scrolling
once production scrolling owns an appropriate contract.

Semantic vocabulary is `#[non_exhaustive]` where later real controls/text may add
behavior. Placeholder actions that silently do nothing are not added.

## Owner-local contribution and identity

A compatible mounted widget contributes **0..N** semantic nodes. Contribution is
read-only with respect to application/runtime behavior and independent of the
widget's `Action` type.

Every contributed node owns one `SemanticKey` unique within its exact mounted
owner. The complete owner-local contribution is validated before acceptance.
One explicit mounted-children marker can splice all direct mounted-child semantic
roots at one deterministic position. Exact rules are:

- zero local semantic nodes make an owner transparent; no marker is allowed;
- one or more local nodes plus direct mounted children require exactly one
  marker;
- no direct mounted children permits no marker;
- missing, duplicate, or unnecessary markers reject rather than receive an
  inferred placement;
- duplicate keys or invalid local references reject deterministically;
- invalid owner-local authoring contributes no local semantic nodes or
  fabricated marker, while independently valid direct child-owner roots remain
  transparent through that owner;
- recursive component action mapping leaves semantic contribution unchanged.

Runtime owns the private binding:

```text
SemanticNodeId
  -> exact mounted owner lifetime
  -> SemanticKey
```

Retaining owner lifetime + key preserves identity across compatible updates and
reordering. Removing the key or owner revokes that lifetime. Later arena reuse
uses a later checked generation; stale and foreign IDs never retarget.

The public semantic product does not expose `MountedNodeId` as a routing
shortcut. Widgets cannot author runtime namespace, live semantic identity,
absolute surface coordinates, mounted identity, or adapter objects.

## Geometry

Canonical `LogicalPoint`, `LogicalLength`, `LogicalSize`, and `LogicalRect` are
core-owned host-neutral geometry. A semantic node uses exact arranged owner
bounds or a validated owner-local logical rectangle translated by runtime.
Widgets do not author absolute surface coordinates.

Layout changes may change semantic bounds/product without rerunning an unchanged
semantic contribution callback. `WidgetInvalidation::SEMANTICS` continues to
mean contribution content/structure may have changed.

M6 may refine transforms/clips/scene geometry without merging semantic and
hit/paint authority.

## Composition, state, support, and focus

`SemanticNodeContribution` is authoring input. M5B publishes composed semantic
state plus supported actions. M5C evaluates current availability separately.

For every semantic node owned by one mounted widget:

```text
published_disabled = authored SemanticState.disabled
                   || !current WidgetActivation.enabled
```

The accepted activation distinctions remain meaningful:

- `WidgetActivation::NONE` means enabled but not directly owner-actionable;
  virtual semantic children may still support actions;
- `WidgetActivation::disabled()` disables interaction owner-wide;
- a named semantic node may additionally author its own disabled state;
- hidden is semantic-subtree scoped;
- inert is authored semantic state;
- disabled/inert nodes may retain supported-action identity while execution is
  unavailable.

Owner `actionable` is **not** a universal semantic-node gate:

- PRIMARY `Activate` support = authored Activate + owner `actionable`;
- named/virtual `Activate` support = exact authored Activate without an owner
  `actionable` requirement;
- `RequestFocus` support exists only on a visible PRIMARY node; `Focusable`
  support is independent of `actionable`, while `Automatic` requires
  `actionable`; current owner enabled/live focus eligibility remains admission
  and default authority;
- `OpenMenu` / `OpenContextMenu` support follows exact authored support without
  fabricating an activation/default requirement.

If required capability authority is unresolved, publication/admission is stale
rather than guessed. Structural contradictions omit support and produce a typed,
deterministic diagnostic.

Mounted `FocusState` is the sole runtime focus authority. A focused mounted owner
projects to its **currently published visible `SemanticKey::PRIMARY`** only.
There is no first/only/root/named fallback. If there is no visible PRIMARY,
semantic focus is `None` and publication records a deterministic typed projection
diagnostic. A hidden PRIMARY is not semantic focus. Active-descendant/composite
focus semantics are later work; non-primary `RequestFocus` is unsupported in M5.

## Relationships and hidden composition

A relationship target is either:

- another owner-local `SemanticKey`; or
- a unique authored `ElementId` plus an optional semantic key in that target
  owner.

`SemanticReference::Authored { semantic_key: None }` means the exact target
owner's `SemanticKey::PRIMARY`. Missing PRIMARY is missing; no first/root/named
fallback is permitted. Ambiguous authored `ElementId` remains ambiguous.
Runtime resolves relationships after current semantic identities are known using
deterministic publication-local indexes.

Hidden nodes/subtrees are absent from the published tree/action surface but may
retain semantic identity for reappearance. Diagnostics never become fallback
routing authority.

## Independent semantic publication

M5B removes production semantic authority from renderer-facing `SurfaceNode`,
`SurfaceFrame`, and debug rendering. The semantic snapshot/update becomes an
independently typed sibling product.

Semantic composition consumes canonical layout facts rather than renderer output.
A renderer can consume its product without semantic vocabulary; an accessibility
or test consumer can consume semantics without interpreting paint proof kinds.
The public semantic product exposes semantic identity/tree/content, not mounted
routing authority.

Any aggregate equality or consuming API must not silently ignore the semantic
sibling. Where both concepts are useful, APIs distinguish renderer-only from
all-product comparison/extraction rather than retaining a compatibility alias.

The snapshot contains deterministic roots/tree order, exact-ID lookup, composed
state/support, relationships, bounds, and semantic focus required by adapters
and tests. RunenUI does not fabricate a semantic wrapper solely for a platform
that wants one root; such a root is adapter-local.

## Semantic revision and updates

Semantic snapshot/update authority is scoped by exact opaque `SurfaceId`, not by
`SurfaceInputContext`, hit-test generation, or coordinate revision.

For each surface:

- the first committed snapshot is revision `1` with no synthetic `0 -> 1` delta;
- an unchanged adapter-visible semantic product keeps the same revision and
  produces no update;
- a changed product advances one checked, non-wrapping revision;
- removals follow previous semantic order;
- additions/changes follow new semantic order;
- roots and semantic focus are explicit update facts when changed;
- wrong surface or wrong prior revision requires full resynchronization;
- diagnostics/readiness-only changes do not bump semantic revision.

Updates are deltas derived from runtime-owned current product, not an independent
mutable semantic store.

## Surface publication transaction

M5B must replace the current imperative append-style publication flow with one
transaction:

```text
admit -> plan -> candidate-dependent final preflight -> commit
```

### Admission

Before downstream capability callbacks, preflight every failure knowable at that
point, including runtime status, required hit-test/coordinate counter capacity,
stationary-rehit queue/work capacity when needed, trace reservation capacity, and
redraw/control validity.

Ordinary queue `Full` while a stationary rehit is required is **recoverable
publication backpressure**:

- return a typed recoverable refusal;
- commit no new publication/cache/semantic/snapshot/trace/redraw/rehit state;
- leave redraw pending/unacknowledged;
- caller may pump work and retry.

Queue fullness is not generic terminal `Poisoned` once publication is atomic.
Work-sequence exhaustion, trace-sequence exhaustion, publication-counter
exhaustion, and genuine integrity failure remain terminal under their owning
authority.

### Plan

After admission succeeds, contractually read-only widget capability callbacks may
run. Stage RunenUI-owned capability-cache results, semantic identity
reconciliation, semantic candidate, and renderer/layout/hit/diagnostic products.

Widget state is not cloned and RunenUI does not promise rollback of arbitrary
interior mutation that violates the read-only callback contract. Semantic-store
reconciliation uses a non-mutating plan/reservation so currently published IDs
are not revoked/allocated during planning.

M5A fail-closed semantics remain exact: invalid contribution, state/bridge
mismatch, semantic identity exhaustion, or semantic-index integrity failure
stages the required owner withdrawal/error state. Identity exhaustion is not
converted into “keep old semantics and continue.”

### Candidate-dependent final preflight

After the candidate product exists, preflight requirements that depend on it,
including semantic revision advancement only if the adapter-visible semantic
product changed. A trace reservation acquired during admission must be releasable
without consuming `TraceSequence` when commit is refused.

If semantic refresh staged a fail-closed owner withdrawal and final preflight
succeeds, withdrawal/revocation, semantic-removal update/diagnostic, capability
cache, and publication commit atomically.

If a terminal final-preflight failure prevents commit, none of the staged
RunenUI-owned semantic mutations commit and no partial new publication is
exposed; runtime becomes terminal rather than remaining live against an
unrepresented transition.

### Commit

Only after all preflights succeed commit semantic store/bindings, capability and
derived caches, dirty completion, semantic revision/update, retained input
snapshot, publication trace, stationary rehit, and redraw acknowledgement.

Recoverable refusal preserves the prior coherent semantic IDs/product/revision
and leaves required dirty/redraw work pending. Terminal failure exposes no
partial new product.

### Failure taxonomy

The public publication boundary becomes fallible or equivalent and distinguishes
at least:

1. recoverable backpressure/refusal, including required stationary-rehit queue
   full;
2. terminal surface-publication exhaustion with exact redraw-revision,
   hit-test-generation, coordinate-revision, or semantic-revision subreason;
3. `WorkSequenceExhausted`;
4. `TraceSequenceExhausted`;
5. terminal integrity failure.

Ordinary monotonic exhaustion must not use `unreachable!`, wrap, or saturate.
No refusal may lose a reservation, clear dirty/redraw authority, or expose a
partial commit.

## Semantic action ingress and M4 convergence

M5C public submission uses an exact surface-scoped request equivalent to:

```text
SemanticActionRequest {
    surface: SurfaceId,
    target: SemanticNodeId,
    action: SemanticAction,
}
```

No semantic revision is carried in the request; admission evaluates current
truth. Node-only public submission is not the long-term contract. M5 still owns
one logical surface; M10 owns multi-surface lifecycle.

Submission performs **no widget callback**. It validates runtime namespace,
exact current surface, semantic lifetime, membership in that surface product,
current support/composed state, and clean/current action readiness. Wrong/foreign
surface, absent target, unresolved semantic/structural authority, or relevant
readiness dirtiness rejects before callback, mutation, wake, `WorkSequence`, or
`TraceSequence`. Layout-only bounds dirtiness need not block action admission.

Accepted semantic-origin work privately retains at least:

```text
SurfaceId
SemanticNodeId
SemanticKey
exact mounted owner lifetime
original SemanticAction
mapped SemanticCommand
```

That metadata travels through the **existing** command envelope/routed
transaction/trace. No second queue, callback engine, activation engine, or trace
is introduced. Semantic-origin callbacks may observe exact read-only target
metadata through non-forgeable vocabulary; ordinary commands carry none and
delegated commands do not inherit it implicitly.

Before the first routed callback, processing revalidates exact surface, semantic
lifetime, owner/key binding, mounted owner, surface membership, support, composed
state, owner enabled state, and action-specific readiness. There is no
replacement, cross-surface, owner-only, or first/last retarget. Failure after
acceptance consumes the accepted `WorkSequence` and performs no callback/default.

For semantic-origin defaults (`Activate`, `RequestFocus`), callbacks may invalidate
semantic/action authority after queue-front validation. Before default execution,
if required authority became dirty/unresolved, runtime does **not** synchronously
refresh semantics: delivered callbacks and work sequence remain, default is
suppressed fail-closed, a deterministic target-invalidated suppression fact is
traced, and no retarget occurs. Explicit `prevent_default` remains a distinct
suppression reason.

PRIMARY Activate uses the existing enabled/actionable probe. Named Activate does
not require owner `actionable`, but does require owner-wide enabled plus exact
named node non-disabled/non-inert state. RequestFocus uses current M4 focus
eligibility.

### Target-aware canonical activation

Existing mounted-owner `Widget::activate` remains the sole activation default
execution path. Semantic-targeted Activate passes immutable exact semantic target
metadata into that existing activation context so a custom owner can distinguish
virtual keys. M5 adds no `activate_semantic`, per-node closure registry, or second
default engine. Non-semantic activation remains unchanged.

## Semantic diagnostics

Semantic contribution/tree/relationship/state/publication diagnostics are typed,
deterministic, and independent from paint/debug strings. They distinguish the
relevant duplicate/missing/ambiguous/stale/integrity/projection/publication
failure classes and never become fallback authority.

## Trace and replay

There remains one canonical bounded/redacted trace. M5C extends its lineage to
prove semantic request -> exact surface/node/key/private owner -> mapped canonical
command -> existing routed/default/update/reconciliation behavior.

M4D3 replay remains inert offline causal observation. It never submits live
semantic actions or reconstructs live runtime authority.

## AccessKit-neutral adapter foundation

RunenUI does not adopt AccessKit/native accessibility types as core vocabulary in
M5. The semantic tree/update/action model is designed to map to stable node
identity, roots/focus, atomic partial updates, properties/bounds/actions, and
targeted action requests without making those platform types authoritative.

M5E performs a fresh source-grounded mapping review against the then-current
adapter API. No AccessKit dependency/native bridge is required for M5A-M5D; M10
owns native host/accessibility integration.

## Public deterministic testing crate

M5D adds genuine downstream workspace crate `runenui_testing` only after M5C
stabilizes public semantic query/action APIs. It depends on public core/runtime
behavior and must not enable private test seams, call hidden mutation bridges,
seed runtime sequences/generations, replace snapshots, invoke callbacks directly,
or maintain parallel expected runtime state.

The harness delegates deterministic mounting, bounded pumping/time, explicit
publication, semantic query/action, existing public interaction sources, and
read-only inspection to `AppRuntime`.

Semantic query results are snapshot-scoped. Action helpers carry exact
`SurfaceId` + `SemanticNodeId` from that product, or act on a query proven unique;
they never reconstruct `MountedNodeId`, bypass semantic ingress, or invent a
LogicalScroll semantic helper.

Any settle convenience takes an explicit finite budget and returns structured
quiescent/exhausted/closed/terminal outcome. Assertions prefer typed data and
ordinary Rust assertions over a macro-heavy DSL or snapshot-golden framework.

## Deferred architecture

Issue #10 (Element/Widget concentration) is not an M5 prerequisite. Focused
semantic ownership should remain cohesive rather than trigger unrelated broad
reorganization.

Issue #12 (widget event-output capacity) is resolved: current capacity is
sufficient because semantic contribution is observational and semantic actions
are ingress into existing command authority, not new widget outputs.

M7 owns possible device-independent semantic scrolling. M8/M9 own production
text/editing and later control semantics. M10 owns multi-surface lifecycle and
native host/accessibility integration.

## Slice boundaries

### M5A — contribution and independent identity

Owns core semantic vocabulary, logical size/rect core cutover, contribution,
semantic arena/index, identity lifecycle, and proof-semantic migration. Accepted.

### M5B — semantic product and incremental updates

Owns tree composition, relationships, composed state/support/focus/bounds,
private owner resolution, independent surface-scoped snapshot/update, semantic
diagnostics, revisions/diffs, staged atomic publication/failure taxonomy, and
clean removal of production semantic authority from renderer-facing products.
It does not accept semantic action requests.

### M5C — semantic actions and accessibility resolution

Owns surface-scoped semantic request/result/error API, exact private target
lineage, support/availability admission, queue-front and post-callback
revalidation, target-aware convergence on existing activation/default authority,
semantic action trace, and inherited `ACCESS-01`/`ACCESS-02` proofs.

### M5D — public deterministic testing

Owns `runenui_testing`, bounded harness ergonomics, snapshot-scoped semantic
queries, exact surface/node action helpers, deterministic settle, assertions, and
replay integration.

### M5E — integrated closure

Owns cross-source conformance, fresh adapter mapping review, final migration
audit, current-document truth, complete then-current M5 matrix proof, and M5
closure. It does not implement M6.

## Migration policy

RunenUI is pre-1.0. M5 uses a clean cutover:

- no compatibility alias for retired semantic proof authority;
- no second old/new semantic callback;
- no retained production renderer semantic authority after M5B cutover;
- no semantic identity coupling to mounted arena allocation;
- no public semantic -> `MountedNodeId` routing shortcut;
- no route-bound LogicalScroll semantic action or compatibility alias;
- no direct semantic activation path, second semantic queue/trace/default engine;
- no public testing API that preserves private/internal test seams.

Repository authority audit must enforce final retired-surface rules by M5E.

## Stop conditions

Stop a slice rather than widening it when:

- accepted `main` drifts with overlapping semantic/testing work;
- semantic truth would merge into renderer/hit/layout authority;
- semantic actions would bypass canonical M4 admission/routing/default authority;
- publication cannot fail without exposing partial state;
- public testing needs private mutation authority;
- implementation needs native accessibility, production text editing, M6 scene
  behavior, or broad M7+ behavior;
- a new production crate lacks independent ownership/consumer pressure;
- an accepted row cannot be proven without first amending charter/matrix.

## M5 exit gate

M5 closes only after:

- every then-current required M5 matrix row is `owner-accepted`;
- inherited M4 `ACCESS-01` and `ACCESS-02` are `owner-accepted`;
- stable and Rust 1.93.0 validation pass at the exact reviewed closure head;
- exact-head CI proves checkout of that head;
- current public API/status/support/roadmap truth is reconciled;
- retired semantic authority/private testing bypasses are absent;
- complete critical review finds no unresolved defect;
- guarded merge and reviewed-head/squash content identity are verified;
- accepted-main validation succeeds;
- any required non-circular post-merge acceptance reconciliation is accepted.

Only the exact accepted M5 closure base may activate M6.
