# M5 Conformance Matrix

> **Category: Target architecture**
>
> **Status:** Accepted
>
> **Accepted by repository owner:** 2026-08-10
>
> **Milestone:** M5

This matrix is the single M5-specific observable behavior and proof inventory.
The [M5 semantics and testing charter](m5-semantics-and-testing-charter.md) owns
implementation boundaries and slice order. The accepted
[M4 conformance matrix](m4-conformance-matrix.md) continues to own inherited
`ACCESS-01` and `ACCESS-02`; those IDs are deliberately not duplicated here.

M5A0 owns the documentation/conformance authority and repository-audit tooling
gate. It owns no framework behavior row. M5A semantic contribution and
independent identity, M5B semantic publication/incremental updates, M5C
semantic action ingress/accessibility resolution, and M5D public deterministic
headless testing are owner-accepted and fully reconciled. M5E #51 is the sole
active M5 slice from exact accepted main
`3c50f2fe0732871a3e2fdf7dba45983a23b813a1`; it owns integration, migration,
and milestone closure only.

Allowed statuses remain:

- `blocked`;
- `implementation-complete`;
- `proof-complete`;
- `owner-accepted`.

M5A semantic contribution and independent identity is owner-accepted. The
reviewed feature head `8377ced53c08d7b5be3020368ceddd3ee81294a5`
passed exact-head CI run `31497457992` / #889 and was guarded-squash-merged in
[PR #53](https://github.com/dornglut/runen-ui/pull/53) as
`e3c304600ec1777cd17a1973946a43c765df1c31`. All 38 changed-file blob
identities are byte-identical between the reviewed feature head and accepted
feature squash. Its mandatory authority/current-contract reconciliation was
then explicitly accepted and guarded-squash-merged in
[PR #54](https://github.com/dornglut/runen-ui/pull/54) as
`d7189d9d145b20edc6ad931ead1589f6277373d2`; reviewed reconciliation head and
squash share exact tree `593592d88c17a86d50d9eda1d3f90d49d8674658`, and accepted-main
push CI run `31546946245` / #898 passed at that exact squash.

The post-M5A readiness amendment freezes successor semantics before M5B/M5C
implementation. It strengthened existing observations and added only
`SEM-SUPPORT-01`, `SEM-PUB-04`, and `SEM-ACT-07` as independently necessary
acceptance observations. That amendment itself promoted no M5B+ behavior.

M5B #48 was explicitly owner-accepted at exact reviewed head
`3b9db8b37098786cc0d53d38ae5d597c3460c38b` after exact-head CI run
`31847771313` / #1082 and final critical review. It was guarded-squash-merged in
[PR #58](https://github.com/dornglut/runen-ui/pull/58) as
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`. Reviewed head and squash share
exact tree `1708d2536c6f1d202ac58dd7cb5f3cc97a438517`, proving repository-content
identity. Because the connector-origin merge did not emit the repository's
normal `push` workflow event, the exact accepted squash was independently
revalidated through the unchanged read-only pull-request CI path in temporary
PR #60; CI run `31850376490` / #1084 attempt 2 passed against exact SHA
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`. PR #60 was closed unmerged.
Exactly the 19 M5B-owned rows below are therefore `owner-accepted`.

M5B's mandatory authority/current-contract reconciliation was then explicitly
owner-accepted at exact reviewed head
`c154e91b5ba693a27eb61a4745d4184193088d5b`, passed exact-head CI #1089 /
`31851743216`, and was guarded-squash-merged in PR #61 as
`afb7f8f363a8df3eb51be1a9bc5f0f180f84190b`. Reviewed head and squash share
exact tree `e6797fb439d8b181d1532c57090915f2589e57de`, and accepted-main push CI
#1090 / `31872934604` passed at that exact squash. M5C #49 therefore activated
from that exact accepted base.

M5C's complete implementation/proof package passed exact-head CI #1166 /
`31882567707` at proof-evidence head
`7565a7a3744c50a93cb542549b8c82e6ae548084`. Final reviewed feature head
`504899b79059eb94ad4474d67bba1e27eb30b374` then passed exact-head CI #1170 /
`31889342640` and final critical review. The repository owner explicitly accepted
that exact head. PR #62 was guarded-squash-merged as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`; reviewed head and squash share
exact tree `dfa7cb71166a3f333b560508a7e82fbeb45df000`. Accepted-main push CI #1171 /
`31903354382` passed at that exact squash. Exactly the seven M5C-owned rows below
are therefore `owner-accepted`.

M5C's mandatory authority/current-contract reconciliation was subsequently
explicitly owner-accepted at exact reviewed head
`fbd0bdf44bddd660e06b4642a56f7a1d64bccab2`, passed exact-head CI #1179 /
`31914448654`, and was guarded-squash-merged in PR #63 as
`b2064f24e778bd69e2876ec09a7431d612682304`. Reviewed reconciliation head and
squash share exact tree `82625aedbdc03a5754949cffee51025e99cd6949`, and
accepted-main push CI #1180 / `31938332306` passed at that exact squash. M5D
#50 therefore activated from that exact accepted base.

M5D's final reviewed feature head
`471d2acf402a0f7d3f89a1de2a1b908fe23ff619` passed canonical exact-head CI
#1230 / `31962536977` and final critical review. The repository owner explicitly
accepted that exact head. PR #64 was guarded-squash-merged as
`72d2405211a3fd6d11e0d17680b7769df90b5ffe`; reviewed head and squash share
exact tree `bdbf19f5c2197490d6b922fb792791b205f40370`. Accepted-main push CI #1231 /
`31967898198` passed at that exact squash. Exactly the ten M5D-owned rows below
are therefore `owner-accepted`.

M5D's mandatory authority/current-contract reconciliation was subsequently
explicitly owner-accepted at exact reviewed head
`522b2770a2e6763e54e9eb6237fefc83e88d8cf9`, passed exact-head CI #1242 /
`31969642341`, and was guarded-squash-merged in PR #65 as
`3c50f2fe0732871a3e2fdf7dba45983a23b813a1`. Reviewed reconciliation head and
squash share exact tree `7e72b2738d539042ed28a032b305fc27cb45042a`, and
accepted-main CI #1244 / `32108782685` passed at that exact squash. M5E #51
therefore activated from that exact accepted base.

Current pre-owner-acceptance candidate summary for this 53-row authority:

```text
53 total unique rows
48 owner-accepted
0 implementation-complete
5 proof-complete
0 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

## M5A — semantic contribution and independent identity

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SEM-ID-01 | Every live semantic node has one opaque runtime-issued `SemanticNodeId` independent of mounted arena slot/generation allocation. | Runtime semantic-arena identity tests | Compile/API construction and mounted-coupling exclusion | Semantic identity diagnostics | M5A | owner-accepted | Required |
| SEM-ID-02 | Retaining the same mounted owner lifetime and owner-local `SemanticKey` preserves the exact semantic identity across compatible updates. | Private runtime compatible-update identity proof | Replacement/removal contrast proof | Identity retention diagnostics | M5A | owner-accepted | Required |
| SEM-ID-03 | Reordering owner-local semantic contributions preserves identities by `SemanticKey`, not contribution position. | Private runtime semantic-key reorder identity proof | Position-derived identity rejection | Identity/reconciliation diagnostics | M5A | owner-accepted | Required |
| SEM-ID-04 | Removing a local semantic key or removing/replacing its mounted owner revokes the exact semantic lifetime; later reuse never retargets the stale ID. | Runtime removal/replacement/generation-reuse proof | Stale-to-replacement no-retarget proof | Semantic lifetime diagnostics | M5A | owner-accepted | Required |
| SEM-ID-05 | Foreign IDs, semantic-slot overflow, and generation exhaustion are rejected without truncation, wrapping, or live-namespace forgery. | Runtime boundary tests | Compile/API namespace extraction and overflow proof | Semantic identity failure diagnostics | M5A | owner-accepted | Required |
| SEM-CONTRIB-01 | A widget contributes an action-type-independent semantic forest containing zero or more owner-local semantic nodes. | Core + downstream custom-widget proof | Action-mapping semantic-coupling proof | Contribution diagnostics | M5A | owner-accepted | Required |
| SEM-CONTRIB-02 | Every contributed semantic node has one owner-local `SemanticKey` unique within its exact mounted owner. | Core/runtime contribution validation | Duplicate-key first/last-match rejection | Duplicate semantic-key diagnostic | M5A | owner-accepted | Required |
| SEM-CONTRIB-03 | A zero-node owner is transparent; otherwise an owner with direct mounted children contains exactly one mounted-children marker, while an owner without mounted children contains none. | Core/runtime composition fixture | Missing/duplicate/unnecessary marker and implicit-placement rejection | Contribution-structure diagnostics | M5A | owner-accepted | Required |
| SEM-CONTRIB-04 | Semantic contribution carries platform-neutral roles, names/descriptions, real values/states/actions, relationships, and plain-text extension facts without AccessKit/native types. | Built-in + downstream semantic vocabulary proof | Platform-vocabulary/dependency exclusion audit | Contribution diagnostics | M5A | owner-accepted | Required |
| SEM-CONTRIB-05 | Recursive component action mapping leaves semantic contribution identity and content unchanged. | Downstream mapped-component proof | Mapped semantic mutation/callback duplication proof | Contribution comparison diagnostics | M5A | owner-accepted | Required |
| SEM-GEOM-01 | Canonical `LogicalSize` and `LogicalRect` are core-owned host-neutral geometry types and runtime deliberately re-exports the same authority where needed. | Core/runtime API conformance | Duplicate runtime geometry type/compatibility alias audit | Repository authority audit | M5A | owner-accepted | Required |
| SEM-GEOM-02 | A widget may author exact owner bounds or a validated owner-local logical rectangle; semantic contribution exposes no absolute surface-coordinate authority. | Core semantic-bounds + downstream owner-local authoring proof | Non-finite/negative geometry and absolute-coordinate/surface-authority exclusion | Semantic contribution / geometry diagnostics | M5A | owner-accepted | Required |

## M5B — semantic tree publication and incremental updates

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SEM-TREE-01 | Runtime composes accepted owner contributions into one deterministic renderer-independent semantic tree/forest with exact semantic identities. | Counter + downstream tree snapshot proof | Renderer/mounted-topology copy rejection | Semantic tree diagnostics | M5B | owner-accepted | Required |
| SEM-TREE-02 | Transparent mounted owners splice child semantic roots into the nearest semantic ancestor without fabricated wrapper nodes. | Transparent-owner composition proof | Artificial-wrapper/order mismatch proof | Tree-composition diagnostics | M5B | owner-accepted | Required |
| SEM-TREE-03 | Explicit mounted-child splice order determines child semantic placement deterministically, including virtual siblings before/after mounted children. | Virtual-node ordering proof | Implicit first/last splice rejection | Tree-composition diagnostics | M5B | owner-accepted | Required |
| SEM-TREE-04 | Runtime derives live `SemanticNodeId`, exact private mounted-owner/key binding, absolute logical bounds, and semantic focus; widget contribution cannot forge them and the public semantic product exposes no `MountedNodeId` routing shortcut. | Runtime-derived-facts + public snapshot proof | Widget-forged fact / public mounted-owner bypass exclusion | Semantic integrity diagnostics | M5B | owner-accepted | Required |
| SEM-REL-01 | Owner-local relationships resolve by `SemanticKey` to the exact live semantic target. | Local relationship conformance | Missing/stale local-key rejection | Relationship diagnostics | M5B | owner-accepted | Required |
| SEM-REL-02 | Cross-owner relationships resolve through unique authored `ElementId` plus optional semantic key; missing or ambiguous authored targets never select first/last. | Downstream cross-owner relationship proof | Missing/ambiguous/replacement no-retarget proof | Relationship diagnostics | M5B | owner-accepted | Required |
| SEM-STATE-01 | Effective published disabled state is authored `SemanticState.disabled` OR owner-wide `!WidgetActivation.enabled`; supported-action identity may remain observable while disabled or inert execution is unavailable. | Owner-wide + per-node disabled snapshot proof | Disabled/inert execution and support-erasure proof | Semantic state diagnostics | M5B | owner-accepted | Required |
| SEM-STATE-02 | Hidden nodes/subtrees are absent from the published semantic tree/action surface, inert nodes expose no executable availability, and mounted runtime focus projects only to the focused owner's currently published visible `SemanticKey::PRIMARY`; no visible PRIMARY yields no semantic focus plus deterministic diagnostic. | Hidden/inert/PRIMARY-focus publication proof | Hidden/inert action, authored-focus, and first/only/named fallback rejection | Semantic state/focus diagnostics | M5B | owner-accepted | Required |
| SEM-SUPPORT-01 | Published supported-action vocabulary is semantic/device-neutral and separates support from current availability: M5 supports only `Activate`, `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; route-bound LogicalScroll has no semantic action or compatibility alias and semantic scrolling remains deferred to M7. | PRIMARY/named/focus/menu support matrix proof | Route/device scroll leakage, universal-actionable gate, and compatibility-alias exclusion | Semantic support diagnostics | M5B | owner-accepted | Required |
| SEM-PUB-01 | Public semantic snapshot is an independently typed sibling product; renderer-facing `SurfaceFrame` can be consumed without semantic vocabulary. | Independent semantic/frame consumer proofs | Mixed `SurfaceNode::semantics` authority audit | Publication authority audit | M5B | owner-accepted | Required |
| SEM-PUB-02 | Semantic snapshot exposes deterministic tree order and exact-ID lookup without mutable runtime authority. | Public snapshot inspection proof | Public constructor/mutation/first-match bypass proof | Snapshot diagnostics | M5B | owner-accepted | Required |
| SEM-PUB-03 | Layout-only movement refreshes absolute semantic bounds/publication without re-running an unchanged cached widget semantic contribution. | Phase-count + bounds-update proof | Missed-bounds and redundant-callback proof | Surface/semantic phase report | M5B | owner-accepted | Required |
| SEM-PUB-04 | Surface publication is one staged admit -> plan -> candidate-dependent final-preflight -> commit transaction: recoverable stationary-rehit queue backpressure performs zero publication/cache/semantic/snapshot/trace/redraw/rehit commit with redraw still pending; fail-closed M5A semantic withdrawal commits atomically when publication succeeds; exact counter/sequence/integrity terminal failure exposes no partial new product or semantic-lifetime mutation. | Recoverable-backpressure + staged semantic-change/withdrawal + counter-exhaustion corpus | Partial commit, generic-Poisoned queue-full, wrap/saturate, lost-reservation, dirty/redraw-clear proof | Typed publication refusal/terminal diagnostics + canonical trace | M5B | owner-accepted | Required |
| SEM-UPD-01 | Each exact `SurfaceId` semantic product owns one deterministic non-wrapping revision: first committed snapshot is revision 1, unchanged adapter-visible product retains its revision/no update, and only a changed product advances after checked preflight. | Surface-scoped revision progression proof | Synthetic 0->1 delta, context/hit/coordinate-only bump, unchanged bump, and exhaustion proof | Semantic update diagnostics | M5B | owner-accepted | Required |
| SEM-UPD-02 | Incremental updates deterministically report added, changed, and removed semantic identities plus tree/root changes. | Add/change/remove diff proof | Omitted/duplicate/stale-delta proof | Semantic update diagnostics | M5B | owner-accepted | Required |
| SEM-UPD-03 | Incremental updates report relationship, runtime-focus, state/action, and logical-bounds changes without replacing unchanged identities. | Focus/bounds/state/relationship diff proof | Identity-churn and missing-change proof | Semantic update diagnostics | M5B | owner-accepted | Required |
| SEM-UPD-04 | Applying or requesting an update is scoped to exact `SurfaceId` and declared previous semantic revision; wrong surface or wrong prior revision requires full resynchronization rather than accepting an ambiguous delta. | Public surface/revision update-chain consumer proof | Wrong-surface/wrong-base acceptance rejection | Update revision diagnostics | M5B | owner-accepted | Required |
| ADAPTER-01 | RunenUI semantic snapshot/update vocabulary contains the stable IDs, tree/root/focus, properties, bounds, actions, relationships, removals, and changes required by an external accessibility adapter without platform types. | Adapter-shaped read-only conformance consumer | Missing-adapter-fact audit | Adapter mapping diagnostics | M5B | owner-accepted | Required |
| ADAPTER-02 | No AccessKit/native accessibility dependency or vocabulary becomes authoritative in core/runtime M5A-M5D implementation. | Dependency/API audit | Platform-type leakage proof | Repository authority audit | M5B | owner-accepted | Required |

## M5C — semantic action ingress and accessibility resolution

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| SEM-ACT-01 | Public `SemanticActionRequest` values constructed with `SemanticActionRequest::new(surface, target, action)` resolve only against the exact current `SurfaceId` semantic product and support/state authority, then join the ordinary command FIFO under the existing canonical non-wrapping `WorkSequence` without invoking widget callbacks at submission. | `semantic_m5c_conformance::semantic_activate_enters_the_existing_fifo_route_default_and_update_path` | `semantic_m5c_conformance::foreign_dirty_and_capacity_rejections_are_atomic_and_recover_exact_requests` + `semantic_m5c_integrity` rejection corpus | `semantic_m5c_conformance` exact acceptance/route lineage | M5C | owner-accepted | Required |
| SEM-ACT-02 | Accepted semantic requests retain exact private `SurfaceId` + `SemanticNodeId` + owner-local `SemanticKey` + mounted-owner lifetime + semantic action metadata through the existing M4 command/routed/default path with `EventSource::Accessibility`; public API exposes no mounted routing shortcut, no second dispatcher, and non-semantic/delegated commands do not inherit semantic target metadata. | `semantic_m5c_conformance` exact event/activation metadata proof | Core compile-fail mounted-shortcut exclusion + `non_semantic_and_delegated_commands_never_inherit_semantic_target_metadata` | `SemanticActionBound -> CommandSubmissionAccepted -> RoutedEventStarted` causal proof | M5C | owner-accepted | Required |
| SEM-ACT-03 | Wrong/foreign surface, missing/stale/foreign/absent-current-product semantic target, dirty semantic authority, unsupported exact binding, and accepted-then-invalid queue-front state reject without callback, retarget, synchronous semantic refresh, or first/last fallback. | Current published-target ingress proof in `semantic_m5c_conformance` | Hidden/replaced/foreign/dirty downstream corpus + same-runtime wrong/missing `semantic_m5c_integrity` + accepted M5A no-first/last arena proofs | `accepted_then_replaced_semantic_work_rejects_without_retargeting` processing-rejection trace | M5C | owner-accepted | Required |
| SEM-ACT-04 | Action support/readiness is exact: PRIMARY Activate depends on owner actionable+enabled; named Activate requires authored support plus owner enabled and node non-disabled/non-inert without an unrelated owner actionable gate; RequestFocus is PRIMARY-only and uses current M4 Focusable/Automatic eligibility; menu/context actions require exact support/state but no actionable gate. | `semantic_m5c_conformance` PRIMARY/named/focus/menu corpus | `semantic_m5c_action_readiness` + disabled/inert/named mismatch corpus | Typed `UnsupportedAction` / `UnavailableAction` evidence and canonical routed outcomes | M5C | owner-accepted | Required |
| SEM-ACT-05 | Mounted-owner integrity/status plus queue/runtime/work/trace capacity remain fail-closed and transactional; rejection returns the exact owned request and introduces no partial callback, mutation, accepted semantic trace lineage, or extra wake. | Successful canonical FIFO submission proof | `semantic_m5c_integrity` same-runtime identity, full-queue, work/trace exhaustion, terminal, sequence/reservation, callback/mutation, and wake-atomicity corpus + downstream Full/Closed proof | Canonical `RuntimeTerminal` lifecycle proof where applicable and zero `SemanticActionBound` on rejected admission | M5C | owner-accepted | Required |
| SEM-ACT-06 | Semantic action acceptance/rejection/default outcomes extend the same bounded/redacted canonical trace and exported schema, preserve exact work/causal lineage, and remain replay observation only rather than a second behavior engine. | `semantic_trace_exports_and_replays_as_inert_canonical_observation` | Trace exhaustion/reservation integrity proof + existing M4 redaction/unknown-kind replay policy | Semantic binding/rejection/default export tokens + inert `TraceReplay` correlation | M5C | owner-accepted | Required |
| SEM-ACT-07 | After routed callbacks but before semantic Activate or RequestFocus default mutation, runtime revalidates exact accepted semantic owner/key/action/current-authority facts without synchronous refresh; callback-caused invalidation suppresses the default deterministically under the accepted `WorkSequence` and is distinguishable from explicit `prevent_default`. | `callback_invalidated_activate_and_prevent_default_have_distinct_trace_outcomes` + `callback_invalidated_request_focus_suppresses_focus_default_without_refresh` | No-refresh/default-mutation assertions in downstream corpus | Distinct `SemanticDefaultTargetInvalidated` versus `SemanticDefaultSuppressed` trace proof | M5C | owner-accepted | Required |

## M5D — public deterministic headless testing harness

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| TEST-HARNESS-01 | Public headless harness can mount arbitrary downstream `UiApp`, inject pointer/keyboard/text/composition/semantic action through public types, pump explicit budgets, and inspect state/semantic/frame/trace products. | Downstream harness conformance | Private-test-seam dependency audit | Harness diagnostics | M5D | owner-accepted | Required |
| TEST-HARNESS-02 | Harness owns a manually advanced public `ManualClock` that yields deterministic `MonotonicInstant` logical time and exposes no wall-clock dependency. | Manual-clock conformance | Wall-clock/sleep exclusion audit | Logical-time trace proof | M5D | owner-accepted | Required |
| TEST-HARNESS-03 | `run_until_idle` stops when one complete pump iteration processes zero envelopes and starts zero work; redraw debt, pending invalidation alone, dormant future timers, externally pending work, and unbounded self-requeue are deterministic stopped/limited outcomes rather than reasons to block forever. | Idle-state corpus | Busy-wait/runaway cap proof | Idle/limit diagnostics | M5D | owner-accepted | Required |
| TEST-HARNESS-04 | Harness uses a deterministic configurable fixed-size rectangular layout/measurement model by default, including child order/placement for route/focus/pointer tests, while custom intrinsic measurement remains explicitly injectable through public host-facing APIs. | Fixed-layout corpus | Zero-size-only/default-magic rejection | Measurement/layout diagnostics | M5D | owner-accepted | Required |
| TEST-HARNESS-05 | Harness supplies public helpers for semantic current/delta/full-resync inspection, trace export/replay/redaction, invalid-ingress construction where public types permit it, pointer/context construction, and sequence/order inspection without exposing mounted internals. | Public helper conformance | Mounted-arena/private-seam leakage audit | Harness/replay diagnostics | M5D | owner-accepted | Required |
| TEST-SEM-01 | Harness can inject semantic actions by exact public `SemanticNodeId` + `SurfaceId` and observe canonical command/default/update results. | Public semantic-action harness proof | Direct-mounted-command bypass audit | Harness trace proof | M5D | owner-accepted | Required |
| TEST-POINTER-01 | Harness can construct valid current surface input context and inject pointer ingress without private constructors. | Public pointer harness proof | Private-constructor dependency audit | Pointer ingress trace proof | M5D | owner-accepted | Required |
| TEST-FOCUS-01 | Harness can deterministically prove focus traversal/restoration through public commands and fixed layout geometry. | Public focus harness proof | Synthetic private focus geometry audit | Focus trace proof | M5D | owner-accepted | Required |
| TEST-TIME-01 | Harness can advance logical time and deterministically prove timers/long-press/time-dependent behavior without sleeping. | Logical-time timer proof | Wall-clock/sleep audit | Scheduler trace proof | M5D | owner-accepted | Required |
| TEST-SCHED-01 | Harness reports deterministic ready-vs-pending external work and remains compatible with bounded pump/start budgets. | Scheduler harness proof | Unbounded execution audit | Scheduler trace proof | M5D | owner-accepted | Required |

## M5E — integrated conformance, migration, and closure

| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |
|---|---|---|---|---|---|---|---|
| M5-INTEG-01 | At least one genuine downstream custom widget contributes semantics, composes through recursive action mapping, publishes through the public harness, receives an exact surface-scoped semantic action, updates parent application state, and leaves canonical trace/replay evidence without a private bridge. | `m5e_closure::mapped_downstream_widget_preserves_semantics_action_runtime_and_trace_authority` | Built-in/private-path-only and semantic-to-mounted shortcut audits | Public canonical trace/replay lineage | M5E | proof-complete | Required |
| M5-INTEG-02 | Accessibility-like semantic action, pointer activation, keyboard activation, authored-ID automation activation, and programmatic activation converge on the same canonical command/routed/default/application-action architecture without duplicate action-engine paths. | Counter public-harness five-origin convergence proof + accepted M4 origin proof | Origin-specific bypass/parallel queue/default audit | Cross-origin canonical trace comparison | M5E | proof-complete | Required |
| M5-INTEG-03 | Current Counter exercises semantic snapshot inspection, exact semantic activation, public harness pumping, semantic update publication, and canonical trace/replay evidence after migration. | Counter M5E closure proof | Hidden private-test API and bare-ID semantic routing audits | Counter trace/replay proof | M5E | proof-complete | Required |
| M5-INTEG-04 | Superseded public testing helpers, M2 mixed semantic proof authority, retired semantic stubs/actions, compatibility aliases/wrappers, and dead current-contract architecture text are absent before acceptance. Retaining a retired compatibility API with documentation does not satisfy this row. | Canonical repository/public-API/current-doc audit | Fatal retired-M5 symbol/action/alias, private-seam, stale-doc, and compatibility-surface audits | Repository authority audit | M5E | proof-complete | Required |
| M5-INTEG-05 | At one frozen M5E feature head, all predecessor M5 rows and inherited M4 `ACCESS-01`/`ACCESS-02` remain `owner-accepted`, `M5-INTEG-01..04` are proof-complete, canonical stable and Rust 1.93.0 exact-head validation is green, and explicit repository-owner acceptance is the only remaining transition for the five-row M5E package itself. | Frozen-head matrix/status + exact-head stable/MSRV CI audit | Blocked/incomplete predecessor, non-green head, or self-acceptance prerequisite audit | Final pre-owner-acceptance evidence | M5E | proof-complete | Required |

## Repository-wide aggregate at pre-owner-acceptance candidate

The repository-wide permanent inventory while M5E awaits explicit owner acceptance is:

```text
290 total unique rows
285 owner-accepted
0 implementation-complete
5 proof-complete
0 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

The aggregate consists of the accepted M4 matrix plus this accepted M5 matrix.
Inherited M4 `ACCESS-01` and `ACCESS-02` remain owner-accepted through M5C; no
permanent acceptance ID is duplicated.

## Reconciliation rule

After each M5 implementation slice reaches `owner-accepted`, perform a separate
reviewed current-contract reconciliation before the next slice starts. The
reconciliation updates this matrix, roadmap/status map, feature support,
architecture/public API where needed, execution tracking, umbrella issue, and
slice issue to one accepted story. A completed feature without that
reconciliation does not activate the successor slice. M5E itself stops at the
explicit owner-acceptance gate before merge; after guarded merge, perform only
the bounded final M5 current-contract reconciliation required to make accepted
`main` truthful before closing M5 and activating M6.
