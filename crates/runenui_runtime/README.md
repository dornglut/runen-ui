# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns deterministic mounted execution and surface publication
for the RunenUI headless proof.

The deterministic application-work scheduler is the M4B slice currently in
review. M4C routed interaction and M4D trace export/replay remain blocked.

`AppRuntime` binds application state, typed actions, update, and transient root
authoring. Each authored root is consumed by sibling-local reconciliation into a
private safe generational arena. Unique keys match regardless of sibling
position; unkeyed children match by unkeyed ordinal; duplicate keys preserve no
ambiguous state; and cross-parent moves remount. Compatible nodes retain mounted
and semantic IDs, widget state, interaction slots, focus, and clean capability
caches. A checked update runs before the newly authored widget description is
committed; mismatch replaces the old subtree immediately.

Mounted IDs contain a runtime-instance `Arc` token, arena slot, and non-wrapping
generation. Lowest vacant slots are reused first; `u64::MAX` slots retire.
Stale same-runtime and foreign-runtime targets have distinct results. Logical
mounted preorder, never arena order, drives inspection, focus traversal, and
publication.

The runtime executes mount/update in preorder, unmount in postorder, replacement
before new mount, and shutdown exactly once through explicit `shutdown`,
`into_state`, and `Drop`. Nodes remain arena-live through unmount; slot release
and state drop follow the hook. One runtime-owned generalized FIFO is the only
work-order authority. `submit_action` returns a non-wrapping
`WorkSequence` or the exact unaccepted action, and the explicit
processed-envelope pump handles a caller-bounded number of envelopes without
recursion. Each action update completes reconciliation and focus validation
before the next envelope begins.

The canonical FIFO also sequences effect starts/cancellations, final mapped
application actions, timer firings, and complete-set subscription
reconciliation. Ready local/send task and subscription values, host responses,
and typed start failures map directly to one final action envelope instead of
passing through an action-bearing completion envelope. Readiness checkpoints
share four explicit per-pump budgets for
processed envelopes, completion imports, local-work polls, and timer promotions.
`PumpReport` exposes exact counters, exhaustion flags, serviceable readiness,
future deadlines, and publication dirtiness. Local futures use wake-aware
eligibility; send payloads cross a bounded ingress and map to non-`Send` actions
only after UI-thread generation validation.

Application and exact mounted owners share one generational work registry.
Keyed replacement and cancellation never retarget a newer generation. Manual or
host monotonic time drives deterministic one-shot/repeating timers. Typed host
requests expose opaque runtime-local tokens and validate response kind before
mapping. Atomic wake requests and revisioned redraw take/acknowledgment remain
separate; shutdown closes producers and reports per-family cancellation counts.

Transitional proof activation preflights runtime status, target capability,
generation capacity, queue capacity, work sequencing, and mandatory trace
sequences before it mutates persistent widget state or invokes an action
factory. Its subscription invalidation, primary action, and auxiliary outputs
commit through one plan: owner cancellation cleanup, mounted subscription
reconciliation, primary action, then auxiliary collector order. It returns
without pumping; state-only activation remains distinct. Queue-full, closed,
and terminal outcomes invoke no mutable callback. Reconciliation reports record
the completed generation and exact mounted/updated/unmounted/moved lifetime
counts.

State-aware activation, measurement, child layout, paint, semantics, and
diagnostics use integrity-aware caches per mounted node. `WidgetInvalidation`
clears only declared capabilities and schedules operational tree/style/layout/
hit-test/paint/semantics/diagnostics/focus phases. The runtime retains the last
proof publication. Its context key contains root constraints, an exact owned
style-token content snapshot, and measurement-provider identity/revision. A
provider must change identity or revision whenever measurement behavior changes.
The topology snapshot retains mounted/semantic identity, parent, authored ID,
widget type, and ordered children only. Style resolution looks up current
mounted `StyleIntent`; layout constructs publication-local resolved nodes from
current mounted `LayoutStyle`. Reconciliation schedules authored token-reference
and gap changes independently from context-key comparison. Explicit topology/
style/layout/hit-test/paint/semantic/diagnostic functions build
`SurfacePhaseReport` only after they run, while private test-only probes count
entry into those functions independently; clean publication executes none.
This is a whole-surface proof cache, not a production retained layout cache.

`ReconciliationDiagnostic` is structured. Duplicate-key records contain the
key, parent path, and all old/new occurrence paths. Payload mismatch remains an
integrity error even when deterministic publication fallbacks are used.

`MountedTreeIndex`, `SurfaceFrame`, `SurfaceStyleReport`, and
`SurfaceLayoutReport` are generated read-only products with identical mounted
ID, semantic ID, parent, and authored-ID sequences. Tree changes collect one
current mounted preorder snapshot and rebuild every node-aligned fact. The
current row/column layout,
measurement provider, paint facts, and semantic facts remain bounded headless
proofs.

The current single-root/focus/publication domain has exactly one mounted root,
one active focus domain, and one current publication domain. Its trace is one
bounded canonical record sequence with
non-wrapping trace identities, opaque exact scheduler-work identity and
structured outcomes, actual accepted work sequences, basic causal lineage, and
an exclusive eviction watermark. There is no
routed event model, pointer identity or true capture, release-inside activation,
trace sink/export/replay or full trace-v2 normalization,
production semantic tree/accessibility adapter, renderer-neutral paint/hit
scene, production layout/style/text, native host, or renderer backend.

See the [public API contract](../../docs/architecture/public-api.md), workspace
[status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md),
and [roadmap](../../docs/roadmap.md).
