# RunenUI Status Map

> **Category: Current contract**

This map reports accepted framework maturity rather than treating unmerged proof
branches, target documents, or historical code as supported behavior.

M4 is complete through M4D3. M5 is active. M5A semantic contribution and
independent identity plus its mandatory post-merge reconciliation are complete.
The M5 readiness gate #55 and its mandatory reconciliation are also complete.
M5B semantic publication/incremental updates is fully accepted and reconciled.

M5C semantic action ingress and accessibility resolution is owner-accepted and
merged. Exact reviewed head `504899b79059eb94ad4474d67bba1e27eb30b374`
passed exact-head CI #1170 / `31889342640` and was guarded-squash-merged in
[PR #62](https://github.com/dornglut/runen-ui/pull/62) as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`. Reviewed head and squash share
exact tree `dfa7cb71166a3f333b560508a7e82fbeb45df000`, and accepted-main push CI
#1171 / `31903354382` passed at that exact squash.

The seven M5C-owned M5 rows and inherited M4 `ACCESS-01`/`ACCESS-02` are therefore
owner-accepted. Current M5 truth is `53 total / 38 owner-accepted / 0
implementation-complete / 0 proof-complete / 15 blocked`. Aggregate configured
truth is `290 total / 275 owner-accepted / 0 implementation-complete / 0
proof-complete / 15 blocked`.

The sole current M5 gate is the mandatory post-M5C authority/current-contract
reconciliation. M5D #50 remains blocked until that reconciliation is exact-head
validated, critically reviewed, explicitly owner-accepted, guarded-merged, and
accepted-main verified.

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
| Mounted runtime identity and indexing | `usable` | One core-owned opaque namespace backs generational `MountedNodeId`, independently allocated `SemanticNodeId`, and opaque `SurfaceId`/`SurfaceInputContext`; `MountedTree` owns separate mounted and semantic generational arenas plus exact owner/key semantic bindings; logical-preorder `MountedTreeIndex` exposes mounted identity; public semantic snapshots expose semantic IDs independently and M5C resolves them privately | Runtime-local, process-local, non-serialized, currently one logical surface; public semantic products deliberately expose no mounted-owner routing shortcut | M3/M4/M5A–M5C complete |
| Keyed reconciliation | `usable` | Transactional sibling-local matching, unkeyed ordinal policy, structured duplicate-key no-reuse diagnostics, cross-parent remount, exact reports | Cross-parent movement remounts; duplicate keys preserve no ambiguous lifetime | M3 complete |
| Persistent widget state and lifecycle | `usable` | State-aware lifecycle, activation, routed-event, exact-generation focus and composition ownership, pointer/Space cleanup, and deterministic shutdown capabilities | Editable text state remains absent | M3/M4 complete |
| Events and interaction | `proof` | Core-owned semantic-command, semantic-action, pointer, focus, keyboard, committed-text, and composition protocols; checked downstream C/T/B; atomic focus out/in; exact focused/semantic binding; propagation/default control; conservative admission; release-inside activation; deterministic automation and semantic-action resolution | No native host translation, production scrolling/editable text, or native accessibility adapter | M4/M5C complete; M8/M10 later |
| Normalized UI navigation commands | `proof` | Activation/menu commands, pointer-derived logical scroll, linear/directional focus, request/restore, focus logical-scroll derivation, and exact M5 semantic action ingress share the canonical FIFO/routing/trace across normalized sources | Production scroll mutation remains later; LogicalScroll stays routed command vocabulary rather than M5 semantic-node action vocabulary | M4/M5C complete; M7 later |
| Directional/spatial focus navigation | `proof` | Current published rectangles, nearest live scope, current eligibility, exact generation, and mounted-order tie-break satisfy DF-01–DF-20 through public commands | Private score is deliberately not public; multi-surface transfer remains absent | M4 complete |
| Effects and scheduling | `proof` | One atomic planner; live-only generational producers; routed exact-owner work output; before-unmount revocation; `Starting -> Running` send subscriptions; tombstone-free host authority; checked mandatory trace plans; once-claimed serialized wake callbacks | Runtime supplies executor/source adapters rather than a default thread pool | M4 complete |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Separate one-query intrinsic/child-layout snapshots; component-wise minimum combination; padding/gaps; unsupported/unknown fallbacks; aligned index/frame/style/layout products | Linear M2 proof only; no production sizing/alignment/flex/grid/scroll/incremental layout | M7–M8 |
| Focusability facts | `proof` | Widgets declare automatic/explicit/hidden focusability and nested scope policy; runtime revalidates live enabled eligibility; semantic publication projects current runtime focus only to the focused owner's visible PRIMARY; M5C semantic `RequestFocus` uses current M4 Focusable/Automatic eligibility | Native accessibility translation and cross-surface focus remain later | M4/M5B–M5C complete; M10 later |
| Semantic contribution and identity | `partial` | Owner-accepted platform-neutral `SemanticContribution`; 0..N owner-local nodes keyed by stable `SemanticKey`; roles, names/descriptions, values/states/action intent, relationships, plain text, owner/owner-local bounds; strict marker/reference validation; separate opaque semantic lifetimes reconciled by exact mounted owner + key; deterministic publication resolves those contributions into exact public semantic identities; M5C resolves exact semantic actions without exposing mounted identity | Public M5D testing queries/helpers and native accessibility adapter remain absent | M5A–M5C complete; M5D next |
| Semantic publication and updates | `proof` | Independent `SurfaceId`-scoped `SemanticPublication` sibling with deterministic forest/preorder/exact-ID lookup, absolute logical bounds, resolved local/cross-owner relationships, composed state/support, runtime PRIMARY focus, typed diagnostics, non-wrapping revision, deterministic add/change/remove/focus deltas, and full-resync on wrong surface/base | M5 has one logical surface; native adapter remains later | M5B complete; M10 later |
| Semantic action ingress and accessibility resolution | `proof` | Public `SemanticActionRequest { surface, target, action }` for exactly `Activate`, `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; exact current surface/identity/publication/support/state/readiness/freshness/capacity admission; existing FIFO/`WorkSequence`/route/default/trace convergence; queue-front and post-callback exact revalidation; exact owned-request recovery; semantic-origin callback metadata without a public semantic-to-mounted shortcut | No semantic LogicalScroll, native accessibility adapter, or M5D harness | M5C complete; M5D/M7/M10 later |
| Surface publication | `proof` | Fallible staged `admit -> plan -> candidate-dependent final-preflight -> commit`; fresh coordinate/hit-test counters; recoverable stationary-rehit backpressure with zero partial commit; exact terminal publication-counter taxonomy; renderer products plus independent semantic publication/diagnostic siblings; explicit renderer-only versus complete extraction/equality | One mounted root and one logical surface; retained `SurfaceCache` still deep-clones narrow non-structural plans (#59); M6 owns the persistent renderer-neutral scene boundary | M5B complete; M6/M10 later |
| Hit testing | `proof` | Reverse-order rectangle hit testing over exact current or retained immutable publication snapshots feeds generation-safe physical pointer paths and stationary re-hit | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, or M6 pointer policy | M4 complete; M6 later |
| Debug and semantic consumption | `proof` | Renderer/debug products no longer carry production semantics; direct public semantic consumers and adapter-shaped consumers inspect the sibling semantic snapshot/update/diagnostic product; M5C exact action ingress remains private-mapping/canonical-routing authority | No native accessibility bridge or production paint scene/backend | M5B–M5C complete; M6/M10 later |
| Renderer-neutral paint scene | `absent` | M2 publishes deterministic open widget paint/debug facts | Proof facts are not primitives/resources and have no clips, transforms, layers, or damage | M6 |
| Production renderer backend | `absent` | None | No conventional or SDF backend; the neutral scene must be accepted first | M10, M12 |
| Deterministic text measurement | `proof` | Provider seam and Unicode-scalar-count measurements support headless tests | Fixed metrics are not font, shaping, grapheme, bidi, wrapping, or baseline layout | M8 |
| Production text subsystem | `absent` | None | No font discovery, shaping, fallback, bidi, wrapping, editing, selection, clipboard, or native IME integration | M8 |
| Button behavior | `proof` | Label/enabled state and repeatable typed `on_activate` factory invoked only by routed `Activate` default; programmatic, physical release-inside, raw Enter, matched raw Space, authored automation, and semantic PRIMARY activation converge; built-in Button authors canonical semantic contribution and appears in the independent semantic product | Native accessibility adapter, recipes, and production control breadth remain later | M4/M5A–M5C complete; M9/M10 later |
| Standard control library | `absent` | None beyond text/button proofs | No complete lifecycle/event/semantic/style/layout/keyboard/accessibility contracts | M9 |
| Host neutrality | `usable` | Active core/runtime have no native window, GPU, ECS, platform-controller, AccessKit, or legacy dependencies | Neutrality alone is not a host integration contract | M10 |
| Host/platform integration | `absent` | Core application host protocol and runtime wake acknowledgment are host-neutral seams only | No native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, or multi-window adapter | M10 |
| Raw controller/gamepad platform input | `absent` | None | No device connection/identity, raw button/axis translation, normalization/dead zones, or embedded-host mapping | M10 |
| Testing and diagnostics | `partial` | Substantial proof-level integration tests; genuine downstream widget plus M5A/M5B semantic and M5C action/readiness conformance; direct and adapter-shaped semantic consumers; deterministic clock/tasks; open layout/hit/paint inspection; strict lints; deterministic trace export/sink and accepted offline replay | No unified M5D public harness, stable public semantic query/action helpers, UI/scene snapshots, fuzzing, property tests, benchmarks, or platform tests | M5D; M11 |
| Trace and observability | `partial` | One bounded canonical M4D1-normalized sequence; deterministic JSONL v1 projection; default-redacted/explicit-full text and IME capture; optional static action labels; subordinate lazily bounded nonblocking sink; inert serialized offline replay; M5C semantic binding/rejection/default outcomes extend the same schema | Replay is a headless causal proof model, not the M5D public testing harness or a production observability service | M4/M5C complete; M5D later |
| Source formats and devtools | `deferred` | Context-export tooling only; no UI source system | No parser, source mapping, inspector, hot reload, live preview, or visual authoring | M12 |
| Advanced editor/game systems | `deferred` | Product direction only | No virtualization, advanced data controls, animation, overlays, docking, workspaces, or advanced multi-surface systems | M12 |
| Legacy archive | `archived` | Git history and annotated tag `legacy-runenwerk-ui-archive-2026-07-11` preserve the audited tree | Removed from active content; salvage remains opt-in and must follow current architecture | M0B complete |

## Current milestone

M0–M4 are complete and owner-accepted. ADR 0005 remains routed-behavior
authority, ADR 0006 remains scheduler-behavior authority, the accepted
[M4C delivery charter](architecture/m4c-delivery-and-routed-transaction-charter.md)
continues to own M4 implementation/delivery constraints, and the accepted
[M4 conformance matrix](architecture/m4-conformance-matrix.md) remains M4
observable-acceptance authority.

M5 is active. M5A, the #55 readiness authority, M5B, and the M5C feature
implementation are owner-accepted and merged. M5C exact reviewed head
`504899b79059eb94ad4474d67bba1e27eb30b374` passed exact-head CI #1170 /
`31889342640`; accepted feature squash/main is
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`, with exact reviewed/squash tree
`dfa7cb71166a3f333b560508a7e82fbeb45df000` and accepted-main push CI #1171 /
`31903354382` success.

Current conformance truth is `53 total / 38 owner-accepted / 0
implementation-complete / 0 proof-complete / 15 blocked`; aggregate configured
truth is `290 total / 275 owner-accepted / 0 implementation-complete / 0
proof-complete / 15 blocked`. Inherited M4 `ACCESS-01` and `ACCESS-02` are now
owner-accepted through M5C.

The current execution gate is the mandatory M5C post-merge reconciliation on
`codex/m5c-authority-reconciliation`, created from exact accepted feature squash
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`. M5C
[#49](https://github.com/dornglut/runen-ui/issues/49) remains open for this
reconciliation only. M5D [#50](https://github.com/dornglut/runen-ui/issues/50)
remains blocked until the reconciliation is owner-accepted, merged, and
accepted-main verified. The roadmap remains the durable milestone authority.

Merged acceptance evidence belongs in pull requests and the
[public repository migration history](history/public-repository-migration.md).
Volatile branch, head, blocker, and next-action state belongs in the
[work-tracking system](work-tracking.md) and GitHub issues.
