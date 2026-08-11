# RunenUI Status Map

> **Category: Current contract**

This map reports accepted framework maturity rather than treating unmerged proof
branches, target documents, or historical code as supported behavior.

M4 is complete through M4D3. M5 is active. M5A semantic contribution and
independent identity plus its mandatory post-merge reconciliation are complete.
The reviewed M5A feature head `8377ced53c08d7b5be3020368ceddd3ee81294a5`
was guarded-squash-merged in [PR #53](https://github.com/dornglut/runen-ui/pull/53)
as `e3c304600ec1777cd17a1973946a43c765df1c31`. Its explicitly accepted
reconciliation head `66c2e2a5e2adf3709f93e8d45821a5844986dc0c` was
 guarded-squash-merged in [PR #54](https://github.com/dornglut/runen-ui/pull/54)
as `d7189d9d145b20edc6ad931ead1589f6277373d2`; reviewed and squash trees
are identical and accepted-main CI #898 passed at that exact squash.

Accepted `main` therefore still owns the reconciled M5 matrix state `50 total /
12 owner-accepted / 0 implementation-complete / 0 proof-complete / 38 blocked`.
The only remaining blocked M4 rows are M5C-owned `ACCESS-01` and `ACCESS-02`.
Issue #55 is the current readiness/authority gate before M5B. Its candidate
matrix authority strengthens successor observations and adds only
`SEM-SUPPORT-01`, `SEM-PUB-04`, and `SEM-ACT-07`, producing candidate state `53
total / 12 owner-accepted / 41 blocked`. Those three added rows are not accepted
framework behavior merely because they exist on the active branch. M5B #48
remains blocked until #55 is exact-head validated, critically reviewed,
explicitly owner-accepted, merged/reconciled as required, and accepted-main
verified.

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
| Application model | `proof` | Core-owned `UiApp`; application state/actions; ordered initial/update effects; state-derived subscriptions; typed host protocol; action/command/pointer/keyboard/text/composition submission through one generalized FIFO | One mounted root and no native-host translation | M4 complete |
| Mounted runtime identity and indexing | `usable` | One core-owned opaque namespace backs generational `MountedNodeId`, independently allocated `SemanticNodeId`, and opaque `SurfaceId`/`SurfaceInputContext`; `MountedTree` owns separate mounted and semantic generational arenas plus owner/key semantic bindings; logical-preorder `MountedTreeIndex` exposes mounted identity; foreign/stale/missing distinction remains exact | Runtime-local, process-local, non-serialized, currently one logical surface; semantic IDs are not yet published through an independent semantic product | M3/M4 complete; M5A complete |
| Keyed reconciliation | `usable` | Transactional sibling-local matching, unkeyed ordinal policy, structured duplicate-key no-reuse diagnostics, cross-parent remount, exact reports | Cross-parent movement remounts; duplicate keys preserve no ambiguous lifetime | M3 complete |
| Persistent widget state and lifecycle | `usable` | State-aware lifecycle, activation, routed-event, exact-generation focus and composition ownership, pointer/Space cleanup, and deterministic shutdown capabilities | Editable text state remains absent | M3/M4 complete |
| Events and interaction | `proof` | Core-owned semantic-command, pointer, focus, keyboard, committed-text, and composition protocols; checked downstream C/T/B; atomic focus out/in; exact focused binding; propagation/default control; conservative admission; release-inside activation; deterministic automation resolution | No native host translation, production scrolling/editable text, or semantic-node accessibility action resolution | M4 complete; M5C/M8/M10 later |
| Normalized UI navigation commands | `proof` | Activation/menu commands, pointer-derived logical scroll, linear/directional focus, request/restore, and focus logical-scroll derivation share the canonical FIFO/routing/trace across normalized sources | Production scroll mutation and semantic accessibility target resolution remain later; LogicalScroll stays routed command vocabulary rather than M5 semantic-node authoring | M4 complete; M5C/M7 later |
| Directional/spatial focus navigation | `proof` | Current published rectangles, nearest live scope, current eligibility, exact generation, and mounted-order tie-break satisfy DF-01–DF-20 through public commands | Private score is deliberately not public; multi-surface transfer remains absent | M4 complete |
| Effects and scheduling | `proof` | One atomic planner; live-only generational producers; routed exact-owner work output; before-unmount revocation; `Starting -> Running` send subscriptions; tombstone-free host authority; checked mandatory trace plans; once-claimed serialized wake callbacks | Runtime supplies executor/source adapters rather than a default thread pool | M4 complete |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Separate one-query intrinsic/child-layout snapshots; component-wise minimum combination; padding/gaps; unsupported/unknown fallbacks; aligned index/frame/style/layout products | Linear M2 proof only; no production sizing/alignment/flex/grid/scroll/incremental layout | M7–M8 |
| Focusability facts | `proof` | Widgets declare automatic/explicit/hidden focusability and nested scope policy; runtime revalidates live enabled eligibility | Runtime focus is not yet projected into the M5B semantic product; #55 freezes PRIMARY-only projection before implementation | M4 complete; #55 gate; M5B later |
| Semantic contribution and identity | `partial` | Owner-accepted M5A platform-neutral `SemanticContribution`; 0..N owner-local nodes keyed by stable `SemanticKey`; roles, names/descriptions, values/states/action intent, relationships, plain text, owner/owner-local bounds; strict marker/reference validation; separate opaque semantic lifetimes reconciled by exact mounted owner + key; downstream action-mapping and geometry conformance | No independently published semantic tree/update product, absolute semantic bounds, runtime-derived focus, resolved cross-owner relationships, semantic-node action ingress, public semantic queries, or accessibility adapter | M5A complete; #55 readiness; M5B–M5D sequential |
| Surface publication | `proof` | Context-bearing publication with fresh coordinate revision/displayed hit-test generation, topology-only renderer-product cache, current mounted style/layout reads, exact token-content key, bounded immutable snapshot retention, independent phase-entry/report proofs, and temporary carriage of canonical M5A `SemanticContribution` on `SurfaceNode` | One mounted root and one logical surface; renderer-facing semantics carriage is transitional and has no semantic IDs; #55 freezes atomic publication/cutover semantics and M5B implements them | M4 complete; #55 gate; M5B/M6/M10 later |
| Hit testing | `proof` | Reverse-order rectangle hit testing over exact current or retained immutable publication snapshots feeds generation-safe physical pointer paths and stationary re-hit | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, or M6 pointer policy | M4 complete; M6 later |
| Debug/semantic frame consumption | `proof` | Deterministic text rendering can inspect temporary canonical semantic contribution alongside paint/diagnostic facts | Debug output is not the M5B semantic tree/update product, accessibility adapter, paint scene, or backend | M5B–M6 |
| Renderer-neutral paint scene | `absent` | M2 publishes deterministic open widget paint/debug facts | Proof facts are not primitives/resources and have no clips, transforms, layers, or damage | M6 |
| Production renderer backend | `absent` | None | No conventional or SDF backend; the neutral scene must be accepted first | M10, M12 |
| Deterministic text measurement | `proof` | Provider seam and Unicode-scalar-count measurements support headless tests | Fixed metrics are not font, shaping, grapheme, bidi, wrapping, or baseline layout | M8 |
| Production text subsystem | `absent` | None | No font discovery, shaping, fallback, bidi, wrapping, editing, selection, clipboard, or native IME integration | M8 |
| Button behavior | `proof` | Label/enabled state and repeatable typed `on_activate` factory invoked only by routed `Activate` default; programmatic, physical release-inside, raw Enter, and matched raw Space paths converge; built-in Button authors canonical M5A role/name/state/Activate contribution | No independent semantic publication/action-resolution/accessibility contract, recipes, or production control breadth | M4 complete; M5A complete; M9 later |
| Standard control library | `absent` | None beyond text/button proofs | No complete lifecycle/event/semantic/style/layout/keyboard/accessibility contracts | M9 |
| Host neutrality | `usable` | Active core/runtime have no native window, GPU, ECS, platform-controller, AccessKit, or legacy dependencies | Neutrality alone is not a host integration contract | M10 |
| Host/platform integration | `absent` | Core application host protocol and runtime wake acknowledgment are host-neutral seams only | No native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, or multi-window adapter | M10 |
| Raw controller/gamepad platform input | `absent` | None | No device connection/identity, raw button/axis translation, normalization/dead zones, or embedded-host mapping | M10 |
| Testing and diagnostics | `partial` | Substantial proof-level integration tests; genuine downstream widget and M5A semantic conformance; deterministic clock/tasks and open widget/style/layout inspection; strict lints; deterministic trace export/sink plus accepted offline replay foundation | No unified M5D public harness, stable semantic queries/actions, UI/scene snapshots, fuzzing, property tests, benchmarks, or platform tests | M5D; M11 |
| Trace and observability | `partial` | One bounded canonical M4D1-normalized sequence; deterministic JSONL v1 projection; default-redacted/explicit-full text and IME capture; optional static action labels; subordinate lazily bounded nonblocking sink; inert serialized offline replay with explicit dropped-prefix incompleteness | Replay is a headless causal proof model, not the M5D public testing harness or a production observability service | M4 complete; M5D later |
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

M5 is active. M5A semantic contribution and independent identity plus its
mandatory reconciliation are complete. The accepted reconciliation squash and
current accepted `main` checkpoint is
`d7189d9d145b20edc6ad931ead1589f6277373d2`, with accepted-main CI #898
successful at that exact SHA.

Accepted-main M5 conformance remains `50 total / 12 owner-accepted / 0
implementation-complete / 0 proof-complete / 38 blocked`; aggregate configured
accepted truth remains `287 total / 247 owner-accepted / 0
implementation-complete / 0 proof-complete / 40 blocked`, where the 40 blocked
rows are the remaining 38 M5 rows plus inherited M4 `ACCESS-01` and `ACCESS-02`.

Issue [#55](https://github.com/dornglut/runen-ui/issues/55) is now the sole
execution gate. It freezes the under-specified M5B/M5C successor authority for
PRIMARY semantic focus, composed disabled/support semantics, private
semantic-to-mounted resolution, surface-scoped requests/revisions, exact stale
and post-callback revalidation, staged atomic publication and failure taxonomy,
and clean renderer/semantic cutover. It also performs only the bounded pre-1.0
removal of route-bound semantic LogicalScroll; no M5B publication or M5C ingress
implementation belongs to #55.

The #55 candidate conformance authority is `53 total / 12 owner-accepted / 41
blocked`, adding only `SEM-SUPPORT-01`, `SEM-PUB-04`, and `SEM-ACT-07` while
leaving all M5B+ observations blocked. M5B #48 may begin only after #55 is
exact-head validated, critically reviewed, explicitly owner-accepted,
merged/reconciled as required, and accepted-main verified. The roadmap remains
the durable milestone authority.

Merged acceptance evidence belongs in pull requests and the
[public repository migration history](history/public-repository-migration.md).
Volatile branch, head, blocker, and next-action state belongs in the
[work-tracking system](work-tracking.md) and GitHub issues.
