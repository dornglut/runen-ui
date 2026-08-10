# M5 Conformance Matrix

> **Category: Target architecture / normative acceptance inventory**
>
> **Status:** Review candidate for owner acceptance
>
> **Milestone:** M5
>
> **Accepted implementation base:** `a63a249de9d4d53eeef4104ae3384e7898aacad1`

This matrix is the single M5-specific observable behavior and proof inventory.
The [M5 semantics and testing charter](m5-semantics-and-testing-charter.md) owns
implementation boundaries and slice order. The accepted
[M4 conformance matrix](m4-conformance-matrix.md) continues to own inherited
`ACCESS-01` and `ACCESS-02`; those IDs are deliberately not duplicated here.

M5A0 owns the documentation/conformance authority and repository-audit tooling
gate. It owns no framework behavior row, so every M5 behavior row begins
`blocked` until its implementation slice reaches the accepted status transition.

Allowed statuses remain:

- `blocked`;
- `implementation-complete`;
- `proof-complete`;
- `owner-accepted`.

Initial audited summary:

```text
50 total unique rows
0 owner-accepted
0 implementation-complete
0 proof-complete
50 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

## M5A — semantic contribution and independent identity

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SEM-ID-01 | Every live semantic node has one opaque runtime-issued `SemanticNodeId` independent of mounted arena slot/generation allocation. | Runtime semantic-arena identity tests | Compile/API construction and mounted-coupling exclusion | Semantic identity diagnostics | M5A | blocked | Required |
| SEM-ID-02 | Retaining the same mounted owner lifetime and owner-local `SemanticKey` preserves the exact semantic identity across compatible updates. | Runtime + downstream compatible-update proof | Replacement/removal contrast proof | Identity retention diagnostics | M5A | blocked | Required |
| SEM-ID-03 | Reordering owner-local semantic contributions preserves identities by `SemanticKey`, not contribution position. | Downstream reorder proof | Position-derived identity rejection | Identity/reconciliation diagnostics | M5A | blocked | Required |
| SEM-ID-04 | Removing a local semantic key or removing/replacing its mounted owner revokes the exact semantic lifetime; later reuse never retargets the stale ID. | Runtime removal/replacement/generation-reuse proof | Stale-to-replacement no-retarget proof | Semantic lifetime diagnostics | M5A | blocked | Required |
| SEM-ID-05 | Foreign IDs, semantic-slot overflow, and generation exhaustion are rejected without truncation, wrapping, or live-namespace forgery. | Runtime boundary tests | Compile/API namespace extraction and overflow proof | Semantic identity failure diagnostics | M5A | blocked | Required |
| SEM-CONTRIB-01 | A widget contributes an action-type-independent semantic forest containing zero or more owner-local semantic nodes. | Core + downstream custom-widget proof | Action-mapping semantic-coupling proof | Contribution diagnostics | M5A | blocked | Required |
| SEM-CONTRIB-02 | Every contributed semantic node has one owner-local `SemanticKey` unique within its exact mounted owner. | Core/runtime contribution validation | Duplicate-key first/last-match rejection | Duplicate semantic-key diagnostic | M5A | blocked | Required |
| SEM-CONTRIB-03 | One explicit mounted-child splice point composes child semantic forests; transparent owners introduce no artificial semantic node. | Core/runtime composition fixture | Missing/duplicate/implicit splice rejection | Contribution-structure diagnostic | M5A | blocked | Required |
| SEM-CONTRIB-04 | Semantic contribution carries platform-neutral roles, names/descriptions, real values/states/actions, relationships, and plain-text extension facts without AccessKit/native types. | Built-in + downstream semantic vocabulary proof | Platform-vocabulary/dependency exclusion audit | Contribution diagnostics | M5A | blocked | Required |
| SEM-CONTRIB-05 | Recursive component action mapping leaves semantic contribution identity and content unchanged. | Downstream mapped-component proof | Mapped semantic mutation/callback duplication proof | Contribution comparison diagnostics | M5A | blocked | Required |
| SEM-GEOM-01 | Canonical `LogicalSize` and `LogicalRect` are core-owned host-neutral geometry types and runtime deliberately re-exports the same authority where needed. | Core/runtime API conformance | Duplicate runtime geometry type/compatibility alias audit | Repository authority audit | M5A | blocked | Required |
| SEM-GEOM-02 | A semantic node uses exact owner bounds or a validated owner-local logical rectangle; widgets cannot author absolute surface coordinates. | Runtime virtual-semantic-bounds proof | Non-finite/negative/absolute-coordinate rejection | Semantic geometry diagnostics | M5A | blocked | Required |

## M5B — semantic tree publication and incremental updates

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SEM-TREE-01 | Runtime composes accepted owner contributions into one deterministic renderer-independent semantic tree/forest with exact semantic identities. | Counter + downstream tree snapshot proof | Renderer/mounted-topology copy rejection | Semantic tree diagnostics | M5B | blocked | Required |
| SEM-TREE-02 | Transparent mounted owners splice child semantic roots into the nearest semantic ancestor without fabricated wrapper nodes. | Transparent-owner composition proof | Artificial-wrapper/order mismatch proof | Tree-composition diagnostics | M5B | blocked | Required |
| SEM-TREE-03 | Explicit mounted-child splice order determines child semantic placement deterministically, including virtual siblings before/after mounted children. | Virtual-node ordering proof | Implicit first/last splice rejection | Tree-composition diagnostics | M5B | blocked | Required |
| SEM-TREE-04 | Runtime-derived `SemanticNodeId`, exact mounted owner, absolute logical bounds, and focus cannot be contradicted or forged by widget contribution. | Runtime-derived-facts proof | Widget-forged runtime-fact exclusion | Semantic integrity diagnostics | M5B | blocked | Required |
| SEM-REL-01 | Owner-local relationships resolve by `SemanticKey` to the exact live semantic target. | Local relationship conformance | Missing/stale local-key rejection | Relationship diagnostics | M5B | blocked | Required |
| SEM-REL-02 | Cross-owner relationships resolve through unique authored `ElementId` plus optional semantic key; missing or ambiguous authored targets never select first/last. | Downstream cross-owner relationship proof | Missing/ambiguous/replacement no-retarget proof | Relationship diagnostics | M5B | blocked | Required |
| SEM-STATE-01 | Disabled semantic nodes remain observable with disabled state while unavailable actions cannot execute. | Counter/downstream disabled snapshot proof | Disabled-action exposure/execution proof | Semantic state diagnostics | M5B | blocked | Required |
| SEM-STATE-02 | Hidden nodes are absent from published semantics/action resolution, inert nodes expose no executable action, and exact runtime focus is derived rather than authored. | Hidden/inert/focus publication proof | Hidden-action/inert-action/authored-focus rejection | Semantic state diagnostics | M5B | blocked | Required |
| SEM-PUB-01 | Public semantic snapshot is an independently typed sibling product; renderer-facing `SurfaceFrame` can be consumed without semantic vocabulary. | Independent semantic/frame consumer proofs | Mixed `SurfaceNode::semantics` authority audit | Publication authority audit | M5B | blocked | Required |
| SEM-PUB-02 | Semantic snapshot exposes deterministic tree order and exact-ID lookup without mutable runtime authority. | Public snapshot inspection proof | Public constructor/mutation/first-match bypass proof | Snapshot diagnostics | M5B | blocked | Required |
| SEM-PUB-03 | Layout-only movement refreshes absolute semantic bounds/publication without re-running an unchanged cached widget semantic contribution. | Phase-count + bounds-update proof | Missed-bounds and redundant-callback proof | Surface/semantic phase report | M5B | blocked | Required |
| SEM-UPD-01 | Semantic publication owns one deterministic non-wrapping revision that advances only when the semantic product changes. | Revision progression proof | Unchanged/fabricated-revision and exhaustion proof | Semantic update diagnostics | M5B | blocked | Required |
| SEM-UPD-02 | Incremental updates deterministically report added, changed, and removed semantic identities plus tree/root changes. | Add/change/remove diff proof | Omitted/duplicate/stale-delta proof | Semantic update diagnostics | M5B | blocked | Required |
| SEM-UPD-03 | Incremental updates report relationship, runtime-focus, state/action, and logical-bounds changes without replacing unchanged identities. | Focus/bounds/state/relationship diff proof | Identity-churn and missing-change proof | Semantic update diagnostics | M5B | blocked | Required |
| SEM-UPD-04 | Applying an update requires its declared previous revision; wrong-base consumers must resynchronize from a complete snapshot rather than accept an ambiguous delta. | Public update-chain consumer proof | Wrong-base application rejection | Update revision diagnostics | M5B | blocked | Required |
| ADAPTER-01 | RunenUI semantic snapshot/update vocabulary contains the stable IDs, tree/root/focus, properties, bounds, actions, relationships, removals, and changes required by an external accessibility adapter without platform types. | Adapter-shaped read-only conformance consumer | Missing-adapter-fact audit | Adapter mapping diagnostics | M5B | blocked | Required |
| ADAPTER-02 | No AccessKit/native accessibility dependency or vocabulary becomes authoritative in core/runtime M5A-M5D implementation. | Dependency/API audit | Platform-type leakage proof | Repository authority audit | M5B | blocked | Required |

## M5C — semantic action ingress and accessibility resolution

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SEM-ACT-01 | Public semantic action submission accepts an exact live `SemanticNodeId` plus supported `SemanticAction`, returns the canonical `WorkSequence`, and invokes no callback at submission time. | Counter semantic-action proof | Callback/mutation-before-pump proof | Semantic ingress trace | M5C | blocked | Required |
| SEM-ACT-02 | Accepted semantic action resolves the exact semantic owner to the exact live mounted target and submits the corresponding existing `SemanticCommand` with accessibility origin. | Counter + downstream convergence proof | Direct activation/second-queue bypass proof | Semantic-to-command causal trace | M5C | blocked | Required |
| SEM-ACT-03 | Missing, stale, foreign, or semantic-index-ambiguous identity is rejected structurally and never retargets a replacement. | Runtime semantic-target rejection corpus | Generation-reuse and first/last-match rejection | Semantic rejection trace | M5C | blocked | Required |
| SEM-ACT-04 | Hidden, inert, disabled/unavailable, or unsupported semantic actions reject according to the accepted state/action policy without callback or mutation. | Semantic state/action rejection corpus | Silent no-op/direct-execution proof | Semantic rejection trace | M5C | blocked | Required |
| SEM-ACT-05 | Mounted-owner integrity/status and canonical queue/closed/terminal/sequence failures return the exact owned semantic request with no partial sequence, wake, callback, focus, widget, or app mutation. | Admission atomicity proof | Capacity/status/sequence partial-commit proof | Canonical semantic rejection trace | M5C | blocked | Required |
| SEM-ACT-06 | Semantic action trace remains part of the one bounded/redacted canonical trace and serialized replay remains inert observational authority. | Trace/export/replay integration proof | Separate accessibility trace/live replay authority audit | M5 semantic causal trace + replay | M5C | blocked | Required |

## M5D — public deterministic headless testing

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| TEST-01 | `runenui_testing` is a genuine downstream crate depending only on public core/runtime APIs and no `internal-test-seams` or doc-hidden mutation bridge. | Workspace dependency/public API audit | Private seam/feature enablement compile audit | Repository authority audit | M5D | blocked | Required |
| TEST-02 | Public harness mounts with deterministic defaults or explicit config and delegates bounded pumping and deterministic time to `AppRuntime`. | Harness lifecycle/time proof | Hidden runtime authority/unbounded loop proof | Public pump/time reports | M5D | blocked | Required |
| TEST-03 | Any settle convenience takes an explicit finite budget and returns structured quiescent/exhausted/closed/terminal outcome. | Settle budget proof | Infinite/implicit pumping proof | Pump outcome diagnostics | M5D | blocked | Required |
| TEST-04 | Semantic queries return deterministic match sets or structured unique-match results distinguishing missing from ambiguous. | Query corpus | First/last arbitrary selection proof | Query diagnostics | M5D | blocked | Required |
| TEST-05 | Harness semantic action helpers act only on an exact semantic ID or a query proven unique and delegate to M5C public semantic ingress. | Counter semantic query/action proof | Ambiguous-query action/direct activation proof | Canonical semantic trace | M5D | blocked | Required |
| TEST-06 | Pointer, keyboard, committed text/composition, automation, controller commands, and direct app actions use ordinary public runtime ingress rather than callback injection. | Multi-source harness integration proof | Private injection/bypass compile audit | Existing canonical trace families | M5D | blocked | Required |
| TEST-07 | Harness exposes typed read-only semantic, layout, hit/current-paint, focus, app-state, reconciliation, trace, and replay observations without parallel expected runtime state. | Public assertion/inspection proof | Parallel expectation registry/mutation proof | Public diagnostic products | M5D | blocked | Required |
| TEST-08 | Counter and a genuine external widget pass keyboard-only, semantic/accessibility-action-only, normalized-controller-only, and applicable pointer deterministic interaction tests through the public harness. | Counter + external-widget scenario suite | Source-specific bypass/no-device-translation proof | Canonical multi-source trace | M5D | blocked | Required |
| TEST-09 | Serialized M4D3 JSONL replay integrates into public testing as offline causal observation and cannot convert replay identities into live runtime authority. | Harness replay proof | Replay-to-live conversion/submit proof | Replay diagnostics | M5D | blocked | Required |

## M5E — integration, migration, and milestone closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M5-MIG-01 | `WidgetSemanticProof` and every parallel M2 semantic callback/payload compatibility authority are removed after the production semantic cutover. | Repository migration audit | Removed-symbol/alias compile proof | Repository authority audit | M5E | blocked | Required |
| M5-MIG-02 | Renderer-facing `SurfaceFrame`/debug renderer no longer own production semantics and `SemanticNodeId` allocation is no longer coupled to mounted arena allocation. | Source/public API audit | Mixed-product/mounted-coupling proof | Repository authority audit | M5E | blocked | Required |
| M5-MIG-03 | No direct semantic activation, parallel semantic queue/trace/runtime, private-testing bypass, or compatibility alias survives M5. | Repository source/authority audit | Compile/API bypass and private-seam proof | Repository authority audit | M5E | blocked | Required |
| M5-CLOSE-01 | Counter and genuine downstream custom widgets prove the complete semantic identity/tree/update/action and deterministic public-harness pipeline across all accepted source families. | Integrated public-only M5 conformance suite | Bypass/source-specific divergence proof | Complete M5 canonical trace/replay evidence | M5E | blocked | Required |
| M5-CLOSE-02 | A source-grounded adapter review proves the accepted RunenUI semantic snapshot/update/action model maps coherently to the then-current AccessKit model without making AccessKit authoritative or adding a native bridge. | M5E adapter mapping review | Unsupported-action/no-op/platform-leak audit | Adapter compatibility record | M5E | blocked | Required |
| M5-CLOSE-03 | Every M5-specific required row plus inherited `ACCESS-01`/`ACCESS-02` is owner-accepted at one exact stable/MSRV head; current docs are truthful, exact-head CI and critical review pass, guarded merge/content identity is verified, and only then M6 becomes active. | Final M5 matrix + accepted-head verification | Premature M6/status/authority proof | Final repository/CI/acceptance evidence | M5E | blocked | Required |
