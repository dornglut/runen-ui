# RunenUI Status Map

> **Category: Current contract**

This map reports accepted framework maturity rather than treating unmerged proof
branches, target documents, or historical code as supported behavior.

M4 is complete through M4D3. M5 is complete through M5E. M5A semantic
contribution and independent identity, the #55 readiness authority, M5B semantic
publication/incremental updates, M5C semantic action ingress/accessibility
resolution, M5D public deterministic headless testing, and M5E integrated
conformance/migration closure are owner-accepted. M6A0 architecture/conformance
authority is also owner-accepted; it defines the target protocol but implements
none of the 36 M6 behavior rows.

M6A0's exact reviewed PR #73 head
`c0169ebea044a0009a334f3d5ecc13ff8d495885` passed exact-head CI #1349 /
`32181344340`, received explicit repository-owner merge authorization, and was
guarded-squash-merged as `966778dd31e0f6b6df76ee4f6283a984fc724b36`.
Reviewed head and squash share exact complete repository tree
`fe057a3fef9ea6de053ce86ce336212f0aa3a413`. Accepted-main CI #1351 /
`32186597198` independently validated that exact squash through read-only PR #74,
which was closed unmerged. Accepted ADR 0007 and the M6 conformance matrix are
therefore current target authority.

Final M5 conformance truth is `53 total / 53 owner-accepted / 0
implementation-complete / 0 proof-complete / 0 blocked`. M4 remains `237 total /
237 owner-accepted / 0 proof-complete / 0 blocked`. M6 currently has `36 total /
0 owner-accepted / 0 implementation-complete / 0 proof-complete / 36 blocked`.
Aggregate configured M4+M5+M6 truth is therefore `326 total / 290
owner-accepted / 0 implementation-complete / 0 proof-complete / 36 blocked`.

M6 renderer-neutral paint/hit scene **implementation remains absent**. The
architecture dependency is now satisfied by accepted ADR 0007, but the bounded
post-M6A0 current-contract reconciliation must itself be accepted, merged,
tree-verified, and accepted-main validated before any implementation branch is
authorized. After that gate, [#59](https://github.com/dornglut/runen-ui/issues/59)
is the first M6A retained-publication implementation slice; it does not authorize
a renderer backend or later M6 behavior.

## Maturity states

| State | Meaning |
|---|---|
| `absent` | No accepted implementation exists. |
| `planned` | Accepted roadmap target; implementation has not started. |
| `proof` | Narrow deterministic behavior exists and is tested. |
| `partial` | Real implementation exists with major production behavior missing. |
| `usable` | Suitable for current internal examples within documented limits. |
| `stable` | Public compatibility and production support are intentionally guaranteed. |
| `deferred` | Deliberately outside the current production foundation or first release. |
| `archived` | Historical material only; not active authority or implementation. |

No framework subsystem is currently `stable`.

## Subsystem status

| Area | Current maturity | What exists | Decisive limitation | Target milestone |
|---|---|---|---|---|
| Authoring and composition | `usable` | Separate built-in views/private widgets; downstream leaves; canonical child-layout `Container`; recursive mapping; arity-free composition | Descriptions are transient reconciliation inputs, not mounted identity/state | M2–M3 complete |
| Application model | `proof` | Core-owned `UiApp`; application state/actions; ordered initial/update effects; state-derived subscriptions; typed host protocol; action/command/semantic-action/pointer/keyboard/text/composition submission through one generalized FIFO | One mounted root and no native-host translation | M4/M5C complete |
| Mounted runtime identity and indexing | `usable` | One core-owned opaque namespace backs generational `MountedNodeId`, independently allocated `SemanticNodeId`, and opaque `SurfaceId`/`SurfaceInputContext`; `MountedTree` owns separate mounted and semantic generational arenas plus exact owner/key semantic bindings; logical-preorder `MountedTreeIndex` exposes mounted identity; public semantic snapshots expose semantic IDs independently and M5C resolves them privately | Runtime-local, process-local, non-serialized, currently one logical surface; public semantic products and M5D semantic query helpers deliberately expose no mounted-owner routing shortcut | M3/M4/M5A–M5D complete |
| Keyed reconciliation | `usable` | Transactional sibling-local matching, unkeyed ordinal policy, structured duplicate-key no-reuse diagnostics, cross-parent remount, exact reports | Cross-parent movement remounts; duplicate keys preserve no ambiguous lifetime | M3 complete |
| Persistent widget state and lifecycle | `usable` | State-aware lifecycle, activation, routed-event, exact-generation focus and composition ownership, pointer/Space cleanup, and deterministic shutdown capabilities | Editable text state remains absent | M3/M4 complete |
| Events and interaction | `proof` | Core-owned semantic-command, semantic-action, pointer, focus, keyboard, committed-text, and composition protocols; checked downstream C/T/B; atomic focus out/in; exact focused/semantic binding; propagation/default control; conservative admission; release-inside activation; deterministic automation and semantic-action resolution; M5D exposes public-only deterministic harness delegates for these accepted ingress paths | No native host translation, production scrolling/editable text, or native accessibility adapter | M4/M5C–M5D complete; M8/M10 later |
| Normalized UI navigation commands | `proof` | Activation/menu commands, pointer-derived logical scroll, linear/directional focus, request/restore, focus logical-scroll derivation, and exact M5 semantic action ingress share the canonical FIFO/routing/trace across normalized sources | Production scroll mutation remains later; LogicalScroll stays routed command vocabulary rather than M5 semantic-node action vocabulary | M4/M5C complete; M7 later |
| Directional/spatial focus navigation | `proof` | Current published rectangles, nearest live scope, current eligibility, exact generation, and mounted-order tie-break satisfy DF-01–DF-20 through public commands; M5D downstream harness proves traversal/restoration using public geometry/commands | Private score is deliberately not public; multi-surface transfer remains absent | M4/M5D complete |
| Effects and scheduling | `proof` | One atomic planner; live-only generational producers; routed exact-owner work output; before-unmount revocation; `Starting -> Running` send subscriptions; tombstone-free host authority; checked mandatory trace plans; once-claimed serialized wake callbacks | Runtime supplies executor/source adapters rather than a default thread pool | M4 complete |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Separate one-query intrinsic/child-layout snapshots; component-wise minimum combination; padding/gaps; unsupported/unknown fallbacks; aligned index/frame/style/layout products; M5D fixed/configurable test surface and custom public measurement-provider proof | Linear M2 proof only; no production sizing/alignment/flex/grid/scroll/incremental layout | M5D proof; M7–M8 production |
| Focusability facts | `proof` | Widgets declare automatic/explicit/hidden focusability and nested scope policy; runtime revalidates live enabled eligibility; semantic publication projects current runtime focus only to the focused owner's visible PRIMARY; M5C semantic `RequestFocus` uses current M4 Focusable/Automatic eligibility | Native accessibility translation and cross-surface focus remain later | M4/M5B–M5C complete; M10 later |
| Semantic contribution and identity | `partial` | Owner-accepted platform-neutral `SemanticContribution`; 0..N owner-local nodes keyed by stable `SemanticKey`; roles, names/descriptions, values/states/action intent, relationships, plain text, owner/owner-local bounds; strict marker/reference validation; separate opaque semantic lifetimes reconciled by exact mounted owner + key; deterministic publication resolves those contributions into exact public semantic identities; M5C resolves exact semantic actions without exposing mounted identity; M5D exposes snapshot-scoped public testing queries/targets without recovering mounted owners | Native accessibility adapter remains absent | M5 complete; M10 later |
| Semantic publication and updates | `proof` | Independent `SurfaceId`-scoped `SemanticPublication` sibling with deterministic forest/preorder/exact-ID lookup, absolute logical bounds, resolved local/cross-owner relationships, composed state/support, runtime PRIMARY focus, typed diagnostics, non-wrapping revision, deterministic add/change/remove/focus deltas, and full-resync on wrong surface/base; M5D exposes public current/delta/full-resync testing observation | M5 has one logical surface; native adapter remains later | M5 complete; M10 later |
| Semantic action ingress and accessibility resolution | `proof` | Public `SemanticActionRequest` values constructed with `SemanticActionRequest::new(surface, target, action)` and submitted via `AppRuntime::submit_semantic_action` for exactly `Activate`, `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; exact current surface/identity/publication/support/state/readiness/freshness/capacity admission; existing FIFO/`WorkSequence`/route/default/trace convergence; queue-front and post-callback exact revalidation; exact owned-request recovery; semantic-origin callback metadata without a public semantic-to-mounted shortcut; M5D testing helpers preserve exact surface/node scope and delegate to this ingress | No semantic LogicalScroll or native accessibility adapter | M5 complete; M7/M10 later |
| Surface publication | `proof` | Fallible staged `admit -> plan -> candidate-dependent final-preflight -> commit`; fresh coordinate/hit-test counters; recoverable stationary-rehit backpressure with zero partial commit; exact terminal publication-counter taxonomy; renderer products plus independent semantic publication/diagnostic siblings; explicit renderer-only versus complete extraction/equality | One mounted root and one logical surface; retained `SurfaceCache` still deep-clones narrow non-structural plans (#59); accepted M6 target authority requires a persistent renderer-neutral scene boundary but has not implemented it | M5B complete; M6/M10 later |
| Hit testing | `proof` | Reverse-order rectangle hit testing over exact current or retained immutable publication snapshots feeds generation-safe physical pointer paths and stationary re-hit; M5D public pointer harness derives exact context and coordinates from committed publication | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, or M6 pointer policy | M4/M5D proof; M6 later |
| Debug and semantic consumption | `proof` | Renderer/debug products no longer carry production semantics; direct public semantic consumers, adapter-shaped consumers, and M5D snapshot-scoped test queries inspect the sibling semantic snapshot/update/diagnostic product; M5C exact action ingress remains private-mapping/canonical-routing authority | No native accessibility bridge or production paint scene/backend | M5 complete; M6/M10 later |
| Renderer-neutral paint scene | `absent` | Accepted ADR 0007 and M6 matrix define the target; current implementation still exposes only M2 deterministic widget paint/debug proof facts | Target authority is not implementation: no `PaintScene`/`PaintPublication`, primitives/resources, clips, transforms, layers, revisions, scale, or damage exist yet | M6 |
| Production renderer backend | `absent` | None | No conventional or SDF backend; the neutral scene implementation must pass M6 before backend work | M10, M12 |
| Deterministic text measurement | `proof` | Provider seam and Unicode-scalar-count measurements support headless tests; M5D can inject a public custom measurement provider | Fixed metrics are not font, shaping, grapheme, bidi, wrapping, or baseline layout | M5D proof; M8 production |
| Production text subsystem | `absent` | None | No font discovery, shaping, fallback, bidi, wrapping, editing, selection, clipboard, or native IME integration | M8 |
| Button behavior | `proof` | Label/enabled state and repeatable typed `on_activate` factory invoked only by routed `Activate` default; programmatic, physical release-inside, raw Enter, matched raw Space, authored automation, and semantic PRIMARY activation converge; built-in Button authors canonical semantic contribution and appears in the independent semantic product; M5D Counter/downstream tests consume these paths through public harness APIs | Native accessibility adapter, recipes, and production control breadth remain later | M4/M5 complete; M9/M10 later |
| Standard control library | `absent` | None beyond text/button proofs | No complete lifecycle/event/semantic/style/layout/keyboard/accessibility contracts | M9 |
| Host neutrality | `usable` | Active core/runtime/testing have no native window, GPU, ECS, platform-controller, AccessKit, or legacy dependencies | Neutrality alone is not a host integration contract | M10 |
| Host/platform integration | `absent` | Core application host protocol and runtime wake acknowledgment are host-neutral seams only | No native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, or multi-window adapter | M10 |
| Raw controller/gamepad platform input | `absent` | None | No device connection/identity, raw button/axis translation, normalization/dead zones, or embedded-host mapping | M10 |
| Testing and diagnostics | `partial` | Public downstream `runenui_testing` with `TestHarness<App>`, deterministic `ManualClock`, configurable fixed-surface publication, finite `SettleBudget`, snapshot-scoped semantic queries/targets, public pointer/keyboard/text/composition/automation/action/command/semantic-action delegates, state/focus/reconciliation/frame/layout/hit/paint/semantic/scheduler/trace/replay observation; genuine Counter and external-widget conformance; strict lints; deterministic trace export/sink and offline replay | No M6 scene snapshot/consumer proofs yet; no snapshot-golden framework, fuzzing, property tests, benchmarks, platform tests, or stable pre-1.0 compatibility guarantee | M5 complete; M6/M11 hardening |
| Trace and observability | `partial` | One bounded canonical M4D1-normalized sequence; deterministic JSONL v1 projection; default-redacted/explicit-full text and IME capture; optional static action labels; subordinate lazily bounded nonblocking sink; inert serialized offline replay; M5C semantic binding/rejection/default outcomes extend the same schema; M5D harness exposes read-only trace/export/replay/redaction observation | Replay and testing remain headless proof/diagnostic infrastructure, not a production observability service | M4/M5 complete |
| Source formats and devtools | `deferred` | Context-export tooling only; no UI source system | No parser, source mapping, inspector, hot reload, live preview, or visual authoring | M12 |
| Advanced editor/game systems | `deferred` | Product direction only | No virtualization, advanced data controls, animation, overlays, docking, workspaces, or advanced multi-surface systems | M12 |
| Legacy archive | `archived` | Git history and annotated tag `legacy-runenwerk-ui-archive-2026-07-11` preserve the audited tree | Removed from active content; salvage remains opt-in and must follow current architecture | M0B complete |

## Current milestone

M0–M5 are complete and owner-accepted. ADR 0005 remains routed-behavior
authority, ADR 0006 remains scheduler-behavior authority, the accepted
[M4C delivery charter](architecture/m4c-delivery-and-routed-transaction-charter.md)
continues to own M4 implementation/delivery constraints, the accepted
[M4 conformance matrix](architecture/m4-conformance-matrix.md) remains M4
observable-acceptance authority, and the accepted
[M5 conformance matrix](architecture/m5-conformance-matrix.md) records all 53 M5
rows as owner-accepted. Accepted
[ADR 0007](adr/0007-renderer-neutral-paint-hit-scene-protocol.md) and the
[M6 conformance matrix](architecture/m6-conformance-matrix.md) now own M6 target
architecture and its 36 blocked observable requirements.

M6A0 acceptance is anchored at reviewed PR #73 head
`c0169ebea044a0009a334f3d5ecc13ff8d495885`, squash/main
`966778dd31e0f6b6df76ee4f6283a984fc724b36`, shared tree
`fe057a3fef9ea6de053ce86ce336212f0aa3a413`, exact-head CI #1349 /
`32181344340`, and accepted-main CI #1351 / `32186597198` through closed-unmerged
verification PR #74. Configured truth is now `326 total / 290 owner-accepted /
36 blocked`; M6 itself remains `36/36 blocked` because architecture acceptance is
not behavior acceptance.

The bounded M6A0 current-contract reconciliation is the active pre-implementation
slice. It changes accepted status/discoverability/evidence only and must not
implement #59 or any M6 behavior. After that reconciliation is owner-accepted,
guarded-squash-merged, tree-verified, and accepted-main validated, #59 becomes
the first eligible M6A implementation slice from that exact accepted base.

Merged acceptance evidence belongs in pull requests and the
[public repository migration history](history/public-repository-migration.md).
Volatile branch, head, blocker, and next-action state belongs in the
[work-tracking system](work-tracking.md) and GitHub issues.