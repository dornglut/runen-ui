# RunenUI Production Roadmap

> **Category: Current contract**

This roadmap is the gated execution authority from the current headless proof to a reviewed production release. A milestone is complete only when its behavioral exit criteria pass; types, documents, or isolated proofs are not completion. Volatile branch, head, pull-request, blocker, and next-action state is owned by the [work-tracking system](work-tracking.md), not by this roadmap.

## Status legend

| State | Meaning |
|---|---|
| `active` | Approved work currently in progress. |
| `queued` | Defined and next when dependencies pass. |
| `blocked` | Defined but cannot start until listed dependencies pass. |
| `deferred` | Deliberately later than the first production foundation or release. |
| `complete` | All exit criteria and required proofs pass. |

Historical foundations—typed application flow, transient element descriptions, deterministic headless proofs, typed style resolution, explicit constraints, measurement-provider contracts, and aligned publication diagnostics—are retained inputs. They do not complete any production milestone by themselves.

## Non-negotiable sequencing

- Do not add broad controls before M2–M5 establish extensibility, mounted identity, events, semantics, and public testing.
- Do not implement renderer backends before M6 implements and proves the accepted neutral paint and hit-test protocols.
- Do not implement interaction-state styling before mounted hover, pressed, focus, and disabled state exist.
- Do not implement editable text before the M4 event model, M5 semantics, and M8 text contracts are coherent.
- Do not manufacture target crates without independent ownership or dependency pressure.
- Do not make signals or observables a competing primary application-state model.

## M0 — Repository authority and governance reset

**Status:** `complete`.

**Goal:** Make repository documentation, active content, metadata, governance, and validation truthful for a pre-1.0 production-readiness program.

**Why now:** Stale skeleton documents, active historical material, `1.0.0` metadata, missing licensing/governance, and divergent validation undermine every later design and release claim.

**Included work:**

- **M0A — production authority documentation (`complete`):** README, architecture framing, status map, feature/support matrix, M0–M12 roadmap, and documentation disposition.
- **M0B — archival/removal (`complete`):** annotated legacy tag; remove `legacy/`, obsolete maps/plans, fake target API, completed incremental documents, and migrated audit backlog; add concise history/ADR records; keep normal context profiles free of legacy material.
- **M0C — release/governance baseline (`complete`):** reset packages to `0.1.0`, disable publishing, add accurate metadata and license files, pin contributor toolchain/MSRV policy, add contribution/security/conduct/changelog/agent/release/API-stability guidance, and align local validation with CI.

**Explicit non-goals:** Any M1 API repair; mounted runtime; controls; renderer protocol/backend; layout expansion; production text; host implementation.

**Dependencies:** A clean then-current default branch, recoverable Git history, and the archival tag before legacy deletion.

**Required proofs/tests:** Full workspace format/test/Clippy/MSRV validation, `cargo validate`, Markdown relative-link check, context-profile checks, manifest metadata inspection, and critical stale-reference/diff review.

**Exit criteria:** README and canonical architecture are truthful; status and support coverage are complete; every document has a disposition; obsolete/duplicate docs and active `legacy/` are gone after archival; normal audits exclude legacy; packages are pre-1.0 and non-publishable; real licensing and governance exist; toolchain/MSRV/release/API policies are explicit; local validation and CI share one baseline; links and checks pass; no false implementation or production claim remains.

**Unblocks:** M1.

## M1 — Public API and core vocabulary repair

**Status:** `complete`.

**Goal:** Remove prototype compatibility traps before more framework code depends on them.

**Why now:** Invalid floats/IDs, ambiguous naming, silent no-op methods, closed/exhaustive public shapes, tuple limits, and public generated-product constructors would multiply migration and correctness cost.

**Included work:** Numeric invariants and logical units; layout/style naming; validated IDs/keys and duplicate diagnostics; remove or implement dead token vocabulary; typed control-specific configuration; unlimited children; prelude reduction; protected generated products; public enum/trait semver strategy.

**Explicit non-goals:** Mounted reconciliation, custom widget implementation, new controls, renderer backends, or layout algorithm expansion.

**Dependencies:** M0 complete.

**Required proofs/tests:** Invalid-value tables/property tests; duplicate ID/key diagnostics; compile-time or behavioral tests for invalid control configuration; unlimited-child macro/builder proof; public visibility/API tests; migration of all examples and docs.

**Exit criteria:** Invalid values cannot silently enter normal public paths; ambiguous identity is diagnosed; invalid element configuration cannot silently no-op; child authoring has no fixed tuple ceiling; public API is deliberately pre-1.0 and generated products cannot be freely forged.

**Completion:** One validated logical-length vocabulary replaced competing raw
wrappers; IDs, keys, and token IDs validate; duplicate authored identity and token
definitions are deterministic and non-overwriting; typed builders replaced flat
no-op setters; iterator/collection children plus `children!` removed arity limits;
preludes and generated-product construction were restricted; enum/trait and
`Action: Clone` policy was reviewed. Public API and compile-fail tests, Counter,
and authority documents were migrated. Owner-review corrections additionally
prove representation-independent textual identity, one Unicode grammar for
literal/dynamic IDs and tokens, true numeric-preorder diagnostics, and finite
saturating derived geometry. See the [M1 public API contract](architecture/public-api.md).

**Unblocks:** M2 and safer M3 implementation.

## M2 — Extensible view/widget/component architecture

**Status:** `complete`.

**Goal:** Let external crates and reusable components participate without modifying closed core enums.

**Why now:** Mounted identity, controls, semantics, layout, and rendering must be built on an open participant contract rather than hardcoded matches.

**Included work:** Public transient View/Element protocol; separate built-in
authored views/private widgets; widget/state identity and safe erasure; external
leaf and child-layout boundaries; canonical container authoring; component action
mapping; lifecycle-only state seam; proof-level intrinsic measurement, child
layout, aligned publication, paint, semantics, diagnostics, and testing.

**Explicit non-goals:** Broad built-in control library, production reconciliation implementation beyond the contract needed for proof, renderer backend, or facade crate.

**Dependencies:** M1; accepted ADR for View/Widget/type erasure coordinated with the M3 identity/storage design.

**Required proofs/tests:** An external test crate defines a custom control through public APIs; a child maps local actions to parent actions; the custom control participates in deterministic event, layout, paint-proof, semantic-proof, and diagnostic paths.

**Exit criteria:** Core enums are no longer the extension gate; component and widget concepts are distinct; public custom-widget and action-mapping contracts are coherent and tested.

**Completion:** ADR 0003 defines the view/element/component/widget vocabulary,
safe owned erasure, process-local widget/state type identity, recursive typed
action mapping, and the lifecycle seam for M3. Built-in text, button, and
container convert to private behavior widgets using the same erased protocol as
downstream widgets. `ChildLayoutWidget`, `ChildLayout`, and canonical
`Container<Action>` separate child ownership/layout from intrinsic measurement.
Runtime traversal, activation/focus, layout, publication, and debug inspection
remain concrete-type-neutral. A genuine
non-publishable downstream package proves custom state/lifecycle, mapped actions,
explicit mutable non-`Clone` activation, vertical/horizontal/nested child layout
with gaps, fixed/text/unsupported intrinsic minimums, independent one-query
measurement/layout snapshots, descendant-preserving fallbacks, preorder/parent
aligned index/frame/style/layout products, hit testing, paint/semantic facts,
diagnostics, and public inspection on stable and Rust 1.93.0. Widget/state
mismatches are category-accurate and generic control-label vocabulary is neutral.
Closed M1 dispatch types were removed. Its lifecycle-only state seam was the
accepted input to M3 and has now been removed by the state-aware mounted
protocol; ADR 0004 supersedes that provisional boundary.

**Unblocks:** M3 and future controls.

## M3 — Mounted runtime and reconciliation

**Status:** `complete`.

**Goal:** Establish persistent runtime identity, lifecycle, state, and granular invalidation.

**Why now:** Focus, capture, editing, scrolling, animations, semantics, tasks, overlays, and safe targeting all require persistent generational identity.

**Included work:** Mounted node arena; generational IDs; keyed/type/position reconciliation; mount/update/unmount; duplicate key diagnostics; widget-local state; focus retention; hover/pressed/capture/scroll slots; semantic identity; dirty/invalidation phases; lifecycle resource ownership; stale-target rejection; multi-surface-ready identity.

**Explicit non-goals:** Full event routing/effects (M4), accessibility adapter (M5), production paint protocol (M6), production layout/text/control breadth.

**Dependencies:** M2 protocol and accepted reconciliation/storage ADR.

**Required proofs/tests:** Keyed reorder preserves local state; compatible rebuild retains identity and focus; removal runs lifecycle and cancels owned resources; stale IDs cannot address replacement nodes; duplicate keys are deterministic; invalidation affects only required phases.

**Exit criteria:** The authored tree is demonstrably transient and mounted state persistent; generational safety and lifecycle tests pass; unconditional focus clearing and preorder identity authority are removed.

**Completion record:** ADR 0004 establishes the mounted tree as sole runtime
authority. Generation preflight, transactional mismatch replacement,
arena-live lifecycle ordering, operational invalidation, retained proof
publication, topology-only snapshots, current mounted style/layout resolution,
independent phase-entry instrumentation, structured diagnostics, and restored
M1/M2 conformance are implemented. The full stable and Rust 1.93.0 validation
matrix passes. M4 boundaries remain intact.

**Unblocks:** M4–M7 and mounted control behavior.

## M4 — Events, effects, scheduling, and trace v2

**Status:** `complete`.

**Goal:** Provide one correct interaction pipeline and deterministic application-work runtime.

**Why now:** Controls, accessibility actions, text input, scrolling, hosts, and testing require consistent routing, commands, queues, effects, time, observation, cancellation, and wake behavior.

**Included work:** Host-event normalization with exact displayed-generation `SurfaceInputContext` retention/rejection and integrity-only cleanup for same-runtime/surface terminal pointer input; observable target/current-target/phase facts; the normative routed/derived event policy; capture/target/bubble; exact framework-default versus route-only semantic-command behavior; pointer IDs/device kind/capture/cancellation; deterministic multi-pointer geometry-triggered boundary updates; release-inside semantic activation and `on_activate`; focus scopes; keyboard commands; abstract next/previous and directional navigation frozen by a public-outcome corpus; activate, cancel/back, menu, context, and logical-scroll commands; input-modality tracking; controller/accessibility-stub/automation/programmatic convergence; separate text/IME streams; one action/work queue with readiness checkpoints and four independent pump budgets; `initial_effects` and update effects; state-derived application subscriptions and dedicated complete-set mounted subscription declarations; local/send tasks with one-attempt terminal executor refusal; timers; owner-local `WorkKey` cancellation/replacement with commit-bound private generations; one exact response-kind-validated application host protocol; lifecycle cancellation; deterministic clock/executor; configured saturation; race-free wake/redraw request/acknowledgment; terminal integrity policy; bounded structured trace plus bounded subordinate export sink and replay foundation.

**Explicit non-goals:** Platform-specific host implementation, full semantics adapter, production text editing, renderer backend, or multi-surface lifecycle.

**Dependencies:** Mounted identity/lifecycle; accepted event and effects ADRs.

**Architecture gate:** ADR 0005 and ADR 0006 were accepted by the repository
owner on 2026-07-14.

### M4 implementation slices

#### M4A — Canonical queue, pump, activation, and trace foundation

**Status:** `complete`.

**Delivered scope:**

- one private sequenced work queue;
- the application-action envelope family;
- non-wrapping `WorkSequence` allocation;
- an explicit processed-envelope pump;
- removal of direct dispatch authority;
- queue-backed proof activation and repeatable `on_activate` factories;
- queue saturation with exact unaccepted-action recovery;
- terminal and shutdown foundations;
- one bounded canonical trace with an exclusive eviction watermark.

M4A is a delivery label only. It does not appear in permanent public API or
subsystem vocabulary, and this slice does not complete M4.

#### M4B — Application work and deterministic scheduler

**Status:** `complete`; owner-accepted and squash-merged in [archive PR #75](history/public-repository-migration.md#accepted-imported-milestone-history).

**Goal:** Implement the complete ADR 0006 application-work contract and
scheduler.

**Included work:**

- move the final public `UiApp` contract into `runenui_core`;
- add `HostProtocol`, `Effects`, `IntoEffects`, default-empty
  `initial_effects`, ordered update effects, and application subscriptions;
- add dedicated complete-set mounted subscription declarations and owner-local
  subscription invalidation;
- evaluate application declarations from current transaction state and mounted
  declarations at their exact queued reconciliation envelope, without retained
  declaration-value caches;
- add `WorkKey`, application and mounted work owners, and a private generational
  work registry;
- add committed effect-start and cancellation envelopes;
- support local non-`Send` tasks, send-capable tasks, exactly one executor start
  attempt, structured executor refusal, and optional typed start-failure
  mapping;
- add a deterministic monotonic clock plus one-shot and repeating timers;
- implement exact host request/response-kind validation;
- implement all four pump budgets, real readiness checkpoints, and complete
  quiescence criteria;
- enforce configured live-work and transaction-output limits;
- add a one-state-mutex wake request/transport/claim state machine, explicit
  callback-in-flight serialization with no framework lock held across host code,
  and independent revisioned redraw;
- keep task, send-subscription, and host producer authority generational,
  live-only, tombstone-free, and revoked before unmount callbacks;
- use checked operation-specific mandatory trace admission and exact accepted-
  action lineage when tracing is enabled;
- complete shutdown/lifecycle cancellation for implemented work and the full
  post-mutation terminal/poison policy.
- attach actual accepted work sequences and basic causal lineage across request,
  generation, start, completion/cancellation, and resulting action facts.

Every new work family must enter the existing canonical sequenced queue. No task
queue, timer execution loop, host queue, or subscription queue may become a
second processing authority. M4B adds trace facts for every scheduler/work
behavior it implements; basic scheduler observability is not deferred to M4D.

**Explicit non-goals:** Routed input events, pointer capture, focus scopes,
text/IME routing, an external trace sink, replay, and a native host
implementation.

**Exit criteria:** The public application contract is core-owned with no
runtime-owned competitor; initial/update/subscription ordering is deterministic;
all ADR 0006 scheduler/work conformance rows in this scope pass on stable and
MSRV.

The final scheduler-integrity correction, stable/MSRV proof package, exact-head
CI, owner review, and squash merge are accepted. M4B does not by itself claim
routed events, trace export/replay, or M4 completion.

The accepted [M4C delivery charter](architecture/m4c-delivery-and-routed-transaction-charter.md)
is the implementation and delivery authority for the remaining slices. The
[M4 conformance matrix](architecture/m4-conformance-matrix.md) is their
observable acceptance and proof authority.

#### M4C0 — Conformance ownership and decision closure

**Status:** `complete`; owner-accepted in [archive PR #76](history/public-repository-migration.md#accepted-imported-milestone-history).

**Goal:** Freeze stable matrix IDs, one delivery owner per observation, explicit
positive/negative/trace proof ownership, accepted M4B status, and repository-wide
execution authority before runtime work resumes.

**Dependencies:** Accepted and squash-merged M4B; owner acceptance of the M4C
delivery charter.

**Included work:** Charter acceptance; in-place matrix normalization; aggregate
row splits; exact M4C1 inventory; roadmap, current API, status, support, README,
changelog, ADR, and architecture alignment.

**Explicit non-goals:** Rust changes, tests, public API placeholders, routed
events, semantic commands, surface contexts, pointer/focus/input behavior, or
trace export/replay.

**Proof ownership:** Matrix uniqueness/schema/count audit; cross-document truth
searches; documentation-only diff review; stable/MSRV `cargo validate`; exact-head
CI.

**Exit criteria:** The accepted charter and matrix are one consistent execution
authority; every normative row has a unique permanent ID, proof owners, slice,
status, and gate; no current document claims M4C/M4D implementation or M4
completion.

**Completion record:** The corrected authority head passed exact-head CI, the
repository owner accepted M4C0, and no runtime implementation or API scaffold was
introduced.

#### M4C1 — Routed semantic-command kernel

**Status:** `complete`; owner-accepted and squash-merged in [archive PR #77](history/public-repository-migration.md#accepted-imported-milestone-history).

**Goal:** Implement the shared runtime namespace, core-owned protocol identity
values, one immutable routed semantic-command transaction, exact admission and
commit, semantic `Activate`, route-only cancel/menu/context commands, and basic
causal trace.

**Dependencies:** Accepted and merged M4C0.

**Included work:** `ID-01`–`ID-04`, `ROUTE-01`–`ROUTE-13`, `CMD-01`–`CMD-14`,
and `MIGRATION-01`–`MIGRATION-05` from the conformance matrix.

**Explicit non-goals:** Surface context; pointer/logical scrolling; focus
scopes; keyboard/text/IME; authored-ID automation resolution; semantic
accessibility resolution; export or replay.

**Proof ownership:** Public core/runtime API proofs, downstream routed-widget
conformance, Counter programmatic activation, rejection/admission boundaries,
slice-local causal trace, stable/MSRV validation, exact-head CI, owner review,
and merge.

**Exit criteria:** One queued non-reentrant routed command path exists, all M4C1
rows are owner-accepted, and no direct activation authority survives outside
explicitly bounded focus-only transition helpers.

**Implementation record:** One shared core-owned runtime namespace now backs
mounted and semantic identities; core owns the opaque mounted/time/sequence
protocol values and narrow routed event vocabulary. The runtime owns exact
target validation, one canonical command envelope, immutable route snapshots,
route-wide bridge preflight, checked transaction admission, propagation/default
control, conservative maximum-safe admission, ordered commit, semantic
activation, route-only commands, structured routed-integrity diagnosis, and the
M4C1 causal graph. Submission rejection recovers exact inputs without work/trace
sequence consumption or a trace record. Direct runtime/pointer/keyboard
activation authority is removed; Counter and a genuine downstream mapped event
widget use public command APIs. All 36 corrected M4C1 rows are `owner-accepted`.

**Completion record:** The accepted feature head
`cb3c7c139304231ba3b636cf53951507f348d485` passed canonical exact-head CI and
was owner-accepted. [archive PR #77](history/public-repository-migration.md#accepted-imported-milestone-history) was squash-merged on 2026-07-19 as
`44ceee29c73cea1237fefbd30db4baf2cd97b93d`.

#### M4C2 — Displayed-generation surface context

**Status:** `complete`; owner-accepted and squash-merged in [archive PR #99](history/public-repository-migration.md#accepted-imported-milestone-history).

**Goal:** Bind neutral input targeting to runtime-issued logical surface,
coordinate revision, and exact retained displayed hit-test generation.

**Dependencies:** Accepted and merged M4C1 shared namespace and routing kernel;
accepted governance/work-tracking closure; accepted behavior-preserving runtime,
trace, and surface authority decomposition.

**Included work:** `SurfaceId`, `SurfaceInputContext`, current/previous and
configurable bounded retention, exact historical targeting, and retired,
missing, foreign-runtime, and foreign-surface outcomes.

**Explicit non-goals:** Pointer stream identity, terminal pointer cleanup,
multi-window lifecycle, cross-surface focus, or M6 paint/hit scenes.

**Proof ownership:** `SURFACE-*` runtime/publication proofs, checked adapter and
negative isolation cases, slice-local causal trace, stable/MSRV validation, and
exact-head CI.

**Exit criteria:** Every `SURFACE-*` row is owner-accepted and no accepted input
is retargeted through another publication.

**Completion record:** The sanitized branch implements one shared-namespace
logical surface, fresh coordinate/display generations, configurable bounded
immutable hit-test retention, exact current/historical targeting, checked logical
and resolved-target ingress, owned rejection recovery, canonical FIFO convergence,
and slice-local causal trace. The accepted feature head
`8127c6143948354f2820f4779c92d2fa9daf79ca` passed the complete exact-head local
baseline and final review. Hosted CI run `29802050579` failed before step 1 with
no job steps; the owner recorded the documented infrastructure-only waiver.
[archive PR #99](history/public-repository-migration.md#accepted-imported-milestone-history) was squash-merged as `9dbf2b6bc781b4e29e3e9ce10388742eccc90124`.

#### M4C3 — Pointer lifecycle

**Status:** `complete`; owner-accepted and squash-merged in
[PR #15](https://github.com/dornglut/runen-ui/pull/15).

**Goal:** Implement pointer/device identity, physical path, pressed ownership,
true capture, deterministic boundaries, logical scrolling, terminal unavailable-
context cleanup, and release-inside activation.

**Dependencies:** Accepted and merged M4C2 displayed-generation contexts.

**Included work:** All `PTR-*`, `CAP-*`, and `BOUNDARY-*` rows plus pointer-owned
migration and logical-scroll behavior.

**Explicit non-goals:** Focus scopes/directional policy, final keyboard/text/IME,
production scrolling mutation, native device translation, or M6 hit scenes.

**Proof ownership:** Runtime/Counter/downstream pointer conformance, multi-pointer
and stationary-geometry proofs, terminal negative cleanup, slice-local causal
trace, stable/MSRV validation, and exact-head CI.

**Exit criteria:** Every M4C3 row is owner-accepted; primary activation is
release-inside; no press-only pointer helper remains.

**Completion record:** The accepted feature head
`01b7ae018abeaff8d316764afba5bc8cde074381` passed exact-head CI run
`29996101708` and was owner-accepted. PR #15 was squash-merged as
`2fc165b9386f55c061d61232400375b13ad175bf`. All 28 M4C3 rows are
`owner-accepted`; M4 remains active and incomplete.

#### M4C4 — Focus scopes and modality

**Status:** `complete`; owner-accepted and squash-merged in
[PR #22](https://github.com/dornglut/runen-ui/pull/22).

**Goal:** Implement root/nested focus scopes, next/previous and directional
navigation, exact-generation restoration, focus transition/focus-within order,
retained modality, and normalized-controller navigation.

**Dependencies:** Accepted and merged M4C3 pointer lifecycle, the accepted
post-merge authority update, and current layout rectangles.

**Included work:** All `FOCUS-*`, `DF-01`–`DF-20`, and `MOD-*` rows.

**Explicit non-goals:** Raw controller/gamepad translation, accessibility tree,
multi-window focus, keyboard/text/IME routing, or public scoring formulas.

**Proof ownership:** Every directional corpus vector through the public command
queue, scope/transition negative cases, slice-local causal trace, stable/MSRV
validation, and exact-head CI.

**Exit criteria:** Every M4C4 row is owner-accepted, every DF vector passes, and
no direct focus-command bypass remains.

**Completion record:** The accepted feature head
`f3201a83583af0c1d148bec87cd9140ff42795b7` passed exact-head CI run
`30006170403` and was owner-accepted. PR #22 was squash-merged as
`f95571634a9c6528e5834e9589b048ad5197bd15`. All 32 M4C4 rows are
`owner-accepted`; M4 remains active and incomplete.

#### M4C5 — Keyboard, text, IME, automation, and M4C closure

**Status:** `complete`; owner-accepted and squash-merged in
[PR #27](https://github.com/dornglut/runen-ui/pull/27). Its post-merge authority
reconciliation is complete and M4D1 subsequently completed.

**Goal:** Complete keyboard activation policy, separate committed-text and IME
streams, exact composition ownership, authored-ID automation resolution, and
all remaining canonical-path migrations.

**Dependencies:** Accepted and merged M4C4 focus scopes/modality and its accepted
authority reconciliation.

**Included work:** All `KEY-*`, `TEXT-*`, `IME-*`, and `AUTOMATION-*` rows;
remaining M4C migration; complete Counter and downstream M4C conformance.

**Explicit non-goals:** Editable text behavior, semantic accessibility mapping,
native IME objects, trace export/sink, or replay.

**Proof ownership:** Counter/downstream keyboard and IME proofs, automation
ambiguity/stale and sequence-exhaustion cases, complete M4C causal trace,
stable/MSRV validation, exact-head CI, independent review, owner acceptance,
and merge.

**Exit criteria:** Every owned M4C5 row is owner-accepted,
physical/programmatic sources converge on one canonical path, direct input
helpers are gone, and the public automation sequence-exhaustion exception is
recorded without weakening ordinary command or accepted-work terminal policy.

**Completion record:** The accepted feature head
`d0d2ef1d53a8ab1d940beb4155f5f991229f042e` passed exact-head CI run
`30843238697` and independent rereview. PR #27 was squash-merged as
`284ecdcfe107e0a7afc88e4bf4fc82eecc52a226`. All fifteen M4C5-owned rows are
`owner-accepted`; M4 remains active and incomplete. Rejected composition starts
recover a generation-free `CompositionStartRequest`, while public authored-ID
automation work/trace-sequence exhaustion is inert and recoverable rather than
terminalizing the runtime. Direct commands and already-accepted work retain the
ordinary terminal exhaustion policy.

#### M4D1 — Complete trace schema

**Status:** `complete`; owner-accepted and squash-merged in
[PR #39](https://github.com/dornglut/runen-ui/pull/39).

**Goal:** Normalize the complete event/surface/pointer/focus/composition/modality
and scheduler trace schema with logical causality and suppressed delivery.

**Dependencies:** Accepted and merged M4C5 implementation with slice-local causal
parentage plus its accepted post-merge authority reconciliation.

**Included work:** All `TRACE-EVENT-*` rows and complete M4 reconstruction fields.

**Explicit non-goals:** Repairing missing earlier causal parents, JSONL export,
external sink, or replay.

**Proof ownership:** End-to-end causal schema/reconstruction, retention and
redaction-boundary proofs, terminal cases, stable/MSRV validation, exact-head CI.

**Exit criteria:** Every `TRACE-EVENT-*` row is owner-accepted and one canonical
trace reconstructs the complete in-memory M4 lifecycle.

**Completion record:** The accepted feature head
`990c49edb5b68c37dd3b7d37dd3f1196a9557c7a` passed canonical exact-head CI run
`31269401262` / #657 and the frozen complete-diff review. PR #39 was
squash-merged as `2fe269366386d7aee9de2a2573498b64ad486293`. The accepted
implementation normalizes typed input, composition, authored-automation, and
application-action trace facts; preserves redacted text/preedit metrics and
checked ranges; reconstructs cleanup/terminal/shutdown ancestry; proves
non-`Debug` action identity; and closes the full Counter/public reconstruction.
All ten `TRACE-EVENT-*` rows are owner-accepted through the post-merge authority
reconciliation.

#### M4D2 — Export and external sink

**Status:** `complete`; owner-accepted and squash-merged in
[PR #41](https://github.com/dornglut/runen-ui/pull/41).

**Goal:** Add deterministic versioned JSONL, default text/IME redaction, optional
application labels, and a behaviorally subordinate bounded external sink.

**Dependencies:** Accepted and merged M4D1 complete schema plus its accepted
post-merge authority reconciliation.

**Included work:** All `TRACE-EXPORT-*` rows, same-record sink diagnostics,
recursion prevention by construction, and transaction isolation.

**Explicit non-goals:** A second ordering authority, unbounded/blocking delivery,
generic `Action: Debug`, replay, or M4 completion.

**Proof ownership:** Snapshot/export, exact JSON escaping, committed-text and IME
redaction/full-capture boundaries, downstream non-`Debug` labels, full/closed
sink diagnostics, recursion isolation, four-state behavioral isolation,
stable/MSRV validation, and exact-head CI.

**Exit criteria:** Every `TRACE-EXPORT-*` row is owner-accepted and sink state can
change neither canonical trace ordering/identity nor application/runtime
behavior.

**Completion record:** The accepted feature head
`1bd7dcfdbb46dec52da62faabb739c835e971c80` passed canonical exact-head CI run
`31321448821` / #712 and the frozen complete-diff review. PR #41 was guarded-
squash-merged as `8c67655ffce438c2e35e6478e7299bd704033b8b`; all 23 changed-file
blob identities match between the reviewed feature head and accepted squash.
The accepted implementation adds deterministic JSONL v1 projection, default-
redacted and explicit-full text/IME capture, optional static action labels,
lazy bounded subordinate sink delivery, receiver-side serialization, same-record
`Delivered`/`Full`/first-`Closed` diagnostics, one-way sink retirement, and
capacity-zero diagnostic dormancy without a second trace/history authority. All
ten `TRACE-EXPORT-*` rows become owner-accepted through this post-merge authority
reconciliation.

**Unblocks:** M4D3.

#### M4D3 — Replay and milestone closure

**Status:** `complete`; owner-accepted and guarded-squash-merged in
[PR #43](https://github.com/dornglut/runen-ui/pull/43).

**Goal:** Complete replay foundation, Counter causal reconstruction, final
migration/current-document cleanup, and the exact M4 acceptance gate.

**Dependencies:** Accepted and merged M4D2 plus its accepted post-merge authority
reconciliation.

**Included work:** `REPLAY-*`, final `MIGRATION-*`, and `M4-CLOSE-*` rows; final
public API/status/support audit; stable/MSRV validation and exact-head CI.

**Explicit non-goals:** M5 semantic/accessibility implementation, M6 scenes,
editable text, native hosts, or later milestone work.

**Proof ownership:** Counter replay, complete matrix/duplicate audit, downstream
M4 conformance, repository truth audit, `cargo validate`, exact-head CI, owner
review, and merge.

**Completion record:** The accepted feature head
`b5f72ccaa89a9fb54d81ec3f35701cbdfbc9ba5d` passed canonical exact-head CI run
`31398930987` / #765, including exact checkout-revision proof and the stable plus
Rust 1.93.0 repository validation authority, and passed the final critical cold
review. PR #43 was guarded-squash-merged as
`596f0d823b9833d71a038cc4aebe834c7b94e4a6`; all 16 feature changed-file blob
identities are byte-identical between the reviewed feature head and accepted
squash. The accepted implementation adds an inert offline JSONL v1 replay model
with replay-only identities, exact retained-sequence/causal validation, explicit
dropped-prefix incompleteness, Counter serialized causal reconstruction, final
retired-authority enforcement, and public Counter/downstream M4 closure proofs.
All eight M4D3-owned rows are `owner-accepted` through the final M4 authority
reconciliation. The normative matrix is `237 total / 235 owner-accepted / 0
proof-complete / 2 blocked`; the only blocked rows are M5-owned `ACCESS-01` and
`ACCESS-02`. M4 is complete.

**Exit criteria:** Every M4-gating matrix row is owner-accepted, M4 is explicitly
accepted and merged, and no transitional authority remains.

**Unblocks after completion:** M5 semantics and deterministic public testing.

**Required proofs/tests:** The normative [M4 conformance matrix](architecture/m4-conformance-matrix.md) and every vector in the [directional-focus corpus](architecture/m4-directional-focus-corpus.md) must pass through public APIs. They cover exact event-family and command-default policy; current/previous/retired/foreign/missing surface generations, no retargeting, and terminal pointer cleanup; cross-pointer publication order; pointer/keyboard/normalized-controller/accessibility-stub/automation/programmatic convergence; deterministic focus/scopes/restoration; capture/composition/boundary/cancellation/release cases; exact initial work ordering, state-derived application subscriptions, and owner-local complete mounted declarations; readiness checkpoints and separate pump budgets; deterministic task/timer/subscription/host ordering; one-attempt executor start/refusal and optional failure mapping; all four same-batch keyed cancellation/start cases; exact host response-kind validation; queue limits and no-silent-drop behavior; wake/redraw races; terminal integrity; bounded canonical trace reconstruction and bounded sink backpressure/recursion behavior; and idempotent shutdown.

**Exit criteria:** One canonical event path remains; overlapping input-intent/direct dispatch paths and `on_press` are removed; correct semantic activation passes; effects and scheduling are deterministic, lifecycle-bound, and saturation-aware; no wake can be lost; trace has sequence/generation/surface/target/owner facts and bounded retention; every required conformance row passes on stable and MSRV.

**Unblocks:** M5, M8, M9, and M10 host integration.

## M5 — Semantics and deterministic public testing

**Status:** `complete`.

**Goal:** Make renderer-independent accessibility semantics and framework-level testing first-class.

**Why now:** Every production control must ship with semantics, keyboard/accessibility behavior, and stable public tests rather than retrofit them later.

**Included work:** Semantic tree with stable IDs, roles, names, descriptions, values, states, relationships, actions, bounds, and text-range extensions; incremental semantic updates; AccessKit-neutral adapter foundation; public headless harness; synthetic input/actions including normalized navigation/controller commands; deterministic clock/tasks; semantic/layout/hit/paint assertions; public replay and testing integration over the replay model and engine accepted in M4D3.

**Explicit non-goals:** Native platform accessibility bridge, production text ranges, full control library, renderer backend.

**Dependencies:** M4 complete; mounted identity and canonical commands/effects are accepted inputs.

**Required proofs/tests:** Counter and custom-widget proofs operate via semantic queries/actions; keyboard-only, semantic navigation/activation, accessibility-action, and controller-only deterministic headless interaction tests that require no platform device translation; stable IDs across compatible updates; disabled/hidden/inert behavior; tests use public harness rather than private runtime internals.

**Exit criteria:** Semantic output is independent of rendering; public deterministic tests can drive and inspect the framework; AccessKit mapping seams are coherent; accessibility requirements are mandatory in later control gates.

**Completion record:** M5 closes at `53 total / 53 owner-accepted / 0 blocked`; configured M4+M5 authority closes at `290/290 owner-accepted`. M5E's final reviewed feature head `7f3e0c9e881ff384516459db66436e662c5fb790` passed exact-head CI #1294 / `32130312467`, received explicit repository-owner merge authorization, and was guarded-squash-merged in PR #67 as `b07ae423d6a3573a4dd8a96a7ce5d6b5b1f0be1e`. Reviewed head and squash share exact complete repository tree `c5dc7fa000496d76c35e98f3a481fc1de5762f4c`; accepted-main CI #1296 / `32135074552` validated that exact squash through unchanged read-only PR #68, which was closed unmerged. Final authority reconciliation PR #69 promotes only the five M5E rows and aligns current-contract authority; no M6 implementation is part of M5 closure.

**Unblocks:** M6, M9, M10, and accessible text integration.

## M6 — Renderer-neutral paint and hit-test scene protocol

**Status:** `active` at the accepted architecture/readiness boundary; all 36 M6 behavior rows remain `blocked`.

**Goal:** Publish backend-neutral paint and hit-test products without widget semantics.

**Why now:** Backends and advanced interaction need stable scenes with explicit order, clips, transforms, resources, and generation identity.

**Included work:** Paint primitives; text/image/resource references; fills, borders/strokes, clips, transforms, opacity, stacking/layers, frame metadata, scale and damage facts; separate hit-test scene with shapes, visibility, inertness, pointer policy, clips/transforms/order; scene generations; deterministic snapshots; backend capabilities.

**Explicit non-goals:** Concrete desktop/SDF backend, production text shaping, full layout/styling expansion, or semantic widget kinds in renderer input.

**Dependencies:** Mounted generation identity and completed M5 semantic separation are satisfied. The render-protocol architecture dependency is satisfied by accepted [ADR 0007](adr/0007-renderer-neutral-paint-hit-scene-protocol.md) and the accepted [M6 conformance matrix](architecture/m6-conformance-matrix.md). Implementation remains gated on completion of the bounded post-M6A0 current-contract reconciliation.

**Required proofs/tests:** Two independent deterministic consumers; custom backend proof renders without knowing `Button`; hit tests respect clips/transforms/visibility/order; scene snapshots are stable and generational targets reject stale input.

**Exit criteria:** M2 widget paint/semantic proof facts are no longer mistaken for
the renderer protocol; paint, hit, semantics, layout, and diagnostics are
distinct authoritative products; no backend-specific vocabulary leaks into
public scenes.

**Accepted M6A0 gate:** PR #73 accepted ADR 0007 and the 36-row M6 matrix from exact reviewed head `c0169ebea044a0009a334f3d5ecc13ff8d495885`; exact-head CI #1349 / `32181344340` passed. Guarded squash `966778dd31e0f6b6df76ee4f6283a984fc724b36` has the identical reviewed tree `fe057a3fef9ea6de053ce86ce336212f0aa3a413`, and accepted-main CI #1351 / `32186597198` validated that exact squash through read-only PR #74. A0 accepts architecture/conformance only, not behavior.

**Pickup gate:** The bounded M6A0 current-contract reconciliation is the final pre-implementation gate and must itself be owner-accepted, guarded-squash-merged, tree-verified, and accepted-main validated. Only then does issue #59 become the first M6A implementation slice from that exact accepted reconciliation squash. No #59/runtime implementation belongs in the reconciliation.

**Unblocks:** M7 rendering integration and M10 backends.

## M7 — Production layout and styling

**Status:** `blocked` by M6.

**Goal:** Support normal responsive applications, tools, scrolling, overlays, and stateful visual policy.

**Why now:** Production controls and apps need complete sizing, alignment, box, scroll, and style-state behavior on persistent nodes and neutral scenes.

**Included work:** Adopt-versus-build layout ADR; sizing/min/max/fill/shrink; flex and alignment; baselines and wrap; stack/absolute/overlay; full box model; clipping and scrolling; scroll extents; incremental layout; themes, recipes, variants, interaction state, user preferences, high contrast, and reduced motion.

**Explicit non-goals:** Unicode shaping/editing implementation, full controls, native host/backend, virtualization, or manufactured crate extraction.

**Dependencies:** Mounted interaction state, semantic/testing contracts, and scene protocol.

**Required proofs/tests:** Layout conformance edge cases; responsive settings app; scroll input/focus/semantics behavior; state-layer and resolution-precedence tests; two consumers before extraction; no generic-layout control-size constants.

**Exit criteria:** Control gallery/settings layouts require no ad hoc geometry; scrolling is input-, focus-, and semantics-aware; incremental invalidation passes; style precedence is inspectable; layout dependency choice is reviewed behind RunenUI contracts.

**Unblocks:** M8–M10 control and host breadth.

## M8 — Production text subsystem

**Status:** `blocked` by M6–M7.

**Goal:** Support internationalized display and editable text through a mature text stack.

**Why now:** Text is foundational to controls and accessibility but requires stable events, semantics, layout, resources, scheduling, and renderer scenes.

**Included work:** Reviewed text-stack ADR; font database/discovery/provider and fallback; shaping, script/language, bidi, line breaking, wrapping, alignment, baselines; glyph/resource caches; editing, selection, caret, clipboard, IME; semantic text ranges and mapping; deterministic fixtures.

**Explicit non-goals:** Hand-written Unicode shaping, platform-specific behavior in the core text model, or all advanced rich-text/editor features.

**Dependencies:** Separate text/IME events, semantics, production layout/style, paint resource protocol, and host provider seams.

**Required proofs/tests:** Multilingual scripts, emoji, combining marks, RTL/bidi, fallback, wrapping, baselines, selection/caret, IME flows, accessible ranges, deterministic headless fixtures, invalidation, and cache/resource budgets on desktop platforms.

**Exit criteria:** Deterministic scalar-count metrics cannot be selected accidentally for a production profile; display/edit text contracts pass conformance; ownership between text, host resources, renderer glyphs, and semantics is explicit.

**Unblocks:** Complete M9 controls and M10 desktop IME/clipboard proof.

## M9 — Standard control library

**Status:** `blocked` by M6–M8.

**Goal:** Provide coherent production controls built on public framework contracts.

**Why now:** Only after identity, events, semantics, scenes, layout/style, text, and testing exist can controls be complete rather than hardcoded primitive variants.

**Included work:** Label/text, button, checkbox, radio, toggle, slider, progress, text field, scroll container, list, menu, popover, tooltip, dialog, and tabs. Every control includes lifecycle state, canonical events/commands, semantics, style states, layout, keyboard operation, normalized controller/navigation operation where applicable, accessibility actions, and deterministic tests. Controller applicability is defined per control and application profile; editable text is not required to expose every editing operation through a controller.

**Explicit non-goals:** Advanced tree/data-grid/editor controls, docking, or product-specific navigation frameworks.

**Dependencies:** M2–M8 gates.

**Required proofs/tests:** Complete control gallery; keyboard-only and applicable normalized controller/navigation operation; semantic query/action coverage; pointer capture/cancellation; focus scopes and overlays; text-field IME/editing; themes/variants; third-party custom-control parity.

**Exit criteria:** No control-specific behavior remains embedded in generic tree indexing/layout; every required control passes interaction, semantic, accessibility, layout, style, and deterministic conformance.

**Unblocks:** M10 reference applications and M11 release candidates.

## M10 — Host and backend production profiles

**Status:** `blocked` by M6–M9.

**Goal:** Run real standalone desktop applications and embedded-host UI through common contracts.

**Why now:** Native integration and backends should prove stable framework protocols, not dictate them.

**Included work:** Host contract; reference desktop adapter; one conventional renderer backend; platform accessibility bridges; clipboard, IME, cursor, drag/drop; DPI/resize/safe areas; multi-window/surface lifecycle; resource providers; shutdown/device-loss behavior; host-owned gamepad connection/disconnection, device identity, raw button/axis translation, dead-zone and normalization policy, and mapping into normalized UI commands; external embedded-host adapter and controller-mapping proof; optional SDF profile only after neutral/conventional proof. RunenUI core/runtime consume normalized commands rather than platform controller APIs.

**Explicit non-goals:** Mobile/web, simultaneous competing production backends, or Runenwerk/ECS assumptions in RunenUI.

**Dependencies:** Effects/event/semantic/scene/layout/text/control contracts; reviewed conventional-renderer and unsafe-code ADRs.

**Required proofs/tests:** Reference apps on Windows/macOS/Linux; DPI/resize/pointer/keyboard/controller/IME/accessibility/device-loss/shutdown smoke tests; controller connect/disconnect, identity, button/axis normalization, and dead-zone cases on relevant platforms/devices; multi-window lifecycle; packaging examples; embedded host owns window/frame loop, maps controllers, and consumes the neutral protocol.

**Exit criteria:** Required desktop services and accessibility work across all three platforms; one supported conventional renderer exists; external embedding works without framework ownership leakage.

**Unblocks:** M11 production hardening.

## M11 — Production hardening and first stable release

**Status:** `blocked` by M6–M10.

**Goal:** Make support, compatibility, security, performance, and release claims enforceable, then deliberately release `1.0.0`.

**Why now:** Stability is a verified product property, not a version-number shortcut.

**Included work:** Windows/macOS/Linux CI; stable and MSRV policy enforcement; docs/doctests/examples; feature combinations; publish dry runs; dependency/license/security policy enforcement; API/semver checks; benchmarks/budgets; property/fuzz/stress tests; optional Miri/sanitizers; packaging; resource/memory tests; release automation; facade crate when justified; public API review.

**Explicit non-goals:** M12 editor/game/authoring breadth or lowering release gates to match missing behavior.

**Dependencies:** All required production profiles and M0–M10 exit criteria.

**Required proofs/tests:** Release-candidate runs of control gallery, settings/form, large list, text editor, overlays/dialogs, multi-window, embedded host, keyboard/accessibility apps, and at least one controller-only game-oriented UI reference proof; relevant cross-platform/device controller tests; documented budgets; cross-platform matrix; semver and supply-chain checks.

**Exit criteria:** No unresolved P0/P1 correctness defects; production profile/support matrix passes; performance budgets and compatibility policy are enforced; release checklist and artifacts pass; public API review approves `1.0.0`.

**Unblocks:** Stable release and post-v1 maintenance.

## M12 — Advanced editor, game, and authoring systems

**Status:** `deferred` until the production kernel is proven.

**Goal:** Build advanced application systems on the stable kernel.

**Why now:** These features are valuable but depend on almost every foundational contract and must not distort the first production release.

**Included work:** Virtualization; advanced list/tree/data-grid/editor controls; animation/time; overlays and advanced multi-surface systems; docking/workspaces; inspector/devtools; external source formats; hot reload/live preview; advanced replay; production SDF backend if not earlier; optional mobile/web profiles.

**Explicit non-goals:** Weakening M0–M11 requirements or moving product-specific state/host ownership into core.

**Dependencies:** Stable or sufficiently proven kernel capabilities from M1–M11; separate ADRs for animation and source/authoring systems.

**Required proofs/tests:** Large-data stress, persistence/migration, drag/overlay/multi-surface correctness, replay determinism, authoring diagnostics, and host/backend conformance appropriate to each slice.

**Exit criteria:** Defined per reviewed advanced slice; no M12 item is required to declare the first production kernel complete unless explicitly promoted through a reviewed release decision.

**Unblocks:** Additional product profiles and ecosystem growth.

## Primary milestone ownership

Every production capability has one primary owner even when it depends on earlier work:

| Capability family | Primary milestone |
|---|---|
| Repository truth, archival, metadata, governance, validation baseline | M0 |
| Core vocabulary and public API safety | M1 |
| Authoring/component/custom-widget protocol | M2 |
| Persistent identity, lifecycle, local state, invalidation | M3 |
| Events, effects, queues, scheduling, trace | M4 |
| Normalized navigation/controller commands and modality tracking | M4 |
| Semantics, accessibility model, public deterministic testing | M5 |
| Paint/hit scenes and renderer-neutral protocol | M6 |
| Production layout, scrolling, themes, recipes, state styling | M7 |
| International and editable text | M8 |
| Standard controls | M9 |
| Native hosts, platform bridges, conventional backend, embedded proof | M10 |
| Raw controller device lifecycle, translation, and normalization | M10 |
| Cross-platform hardening, budgets, release, `1.0.0` | M11 |
| Advanced editor/game/authoring systems | M12 |

## Definition of roadmap completion

The roadmap reaches its first stable completion only when RunenUI has deterministic mounted headless execution, real applications on Windows/macOS/Linux, an external embedded-host proof, correct pointer/keyboard/controller-gamepad/text/IME/clipboard/accessibility behavior for applicable controls and application profiles, production text and responsive layout, standard controls, one conventional backend, a neutral protocol suitable for SDF/engine consumption, public deterministic replay/testing, documented performance budgets, cross-platform validation, no unresolved P0/P1 architecture defects, and a reviewed `1.0.0` release. Controller applicability does not require every text-editing action to be practical through a controller.
