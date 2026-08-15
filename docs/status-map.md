# RunenUI Status Map

> **Category: Current contract**

This map reports accepted framework maturity rather than treating unmerged proof
branches, target documents, or historical code as supported behavior.

M4 is complete through M4D3. M5 is active. M5A semantic contribution and
independent identity plus its mandatory post-merge reconciliation are complete.
The M5 readiness gate #55 and its mandatory reconciliation are also complete.

M5B semantic tree publication and incremental updates is now explicitly
owner-accepted and merged. Exact reviewed head
`3b9db8b37098786cc0d53d38ae5d597c3460c38b` passed exact-head CI #1082 and was
guarded-squash-merged in [PR #58](https://github.com/dornglut/runen-ui/pull/58)
as `43d23aefb81757a516ae569b3e86b9e0f2c71e23`. Reviewed and squash trees are
identical at `1708d2536c6f1d202ac58dd7cb5f3cc97a438517`. Because the connector-origin
merge did not emit the normal `push` workflow event, the exact accepted squash
was independently revalidated through the unchanged read-only pull-request CI
path in temporary PR #60; CI #1084 attempt 2 passed against exact squash
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`, and PR #60 was closed unmerged.

This post-merge authority/current-contract reconciliation records the 19 M5B
rows as owner-accepted. Reconciled M5 truth is therefore `53 total / 31
owner-accepted / 0 implementation-complete / 0 proof-complete / 22 blocked`.
Aggregate configured truth is `290 total / 266 owner-accepted / 0
implementation-complete / 0 proof-complete / 24 blocked`; the 24 blocked rows
are the remaining 22 M5-specific rows plus inherited M4 `ACCESS-01` and
`ACCESS-02`, all owned by M5C or later work.

M5C #49 remains blocked until this reconciliation is itself exact-head validated,
critically reviewed, explicitly owner-accepted, guarded-merged, and accepted-main
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
| Mounted runtime identity and indexing | `usable` | One core-owned opaque namespace backs generational `MountedNodeId`, independently allocated `SemanticNodeId`, and opaque `SurfaceId`/`SurfaceInputContext`; `MountedTree` owns separate mounted and semantic generational arenas plus exact owner/key semantic bindings; logical-preorder `MountedTreeIndex` exposes mounted identity; public M5B semantic snapshots expose semantic IDs independently | Runtime-local, process-local, non-serialized, currently one logical surface; public semantic products deliberately expose no mounted-owner routing shortcut | M3/M4/M5A–M5B complete |
| Keyed reconciliation | `usable` | Transactional sibling-local matching, unkeyed ordinal policy, structured duplicate-key no-reuse diagnostics, cross-parent remount, exact reports | Cross-parent movement remounts; duplicate keys preserve no ambiguous lifetime | M3 complete |
| Persistent widget state and lifecycle | `usable` | State-aware lifecycle, activation, routed-event, exact-generation focus and composition ownership, pointer/Space cleanup, and deterministic shutdown capabilities | Editable text state remains absent | M3/M4 complete |
| Events and interaction | `proof` | Core-owned semantic-command, pointer, focus, keyboard, committed-text, and composition protocols; checked downstream C/T/B; atomic focus out/in; exact focused binding; propagation/default control; conservative admission; release-inside activation; deterministic automation resolution | No native host translation, production scrolling/editable text, or public semantic-node action ingress/accessibility resolution | M4 complete; M5C/M8/M10 later |
| Normalized UI navigation commands | `proof` | Activation/menu commands, pointer-derived logical scroll, linear/directional focus, request/restore, and focus logical-scroll derivation share the canonical FIFO/routing/trace across normalized sources | Production scroll mutation and semantic-node action resolution remain later; LogicalScroll stays routed command vocabulary rather than M5 semantic-node authoring | M4 complete; M5C/M7 later |
| Directional/spatial focus navigation | `proof` | Current published rectangles, nearest live scope, current eligibility, exact generation, and mounted-order tie-break satisfy DF-01–DF-20 through public commands | Private score is deliberately not public; multi-surface transfer remains absent | M4 complete |
| Effects and scheduling | `proof` | One atomic planner; live-only generational producers; routed exact-owner work output; before-unmount revocation; `Starting -> Running` send subscriptions; tombstone-free host authority; checked mandatory trace plans; once-claimed serialized wake callbacks | Runtime supplies executor/source adapters rather than a default thread pool | M4 complete |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Separate one-query intrinsic/child-layout snapshots; component-wise minimum combination; padding/gaps; unsupported/unknown fallbacks; aligned index/frame/style/layout products | Linear M2 proof only; no production sizing/alignment/flex/grid/scroll/incremental layout | M7–M8 |
| Focusability facts | `proof` | Widgets declare automatic/explicit/hidden focusability and nested scope policy; runtime revalidates live enabled eligibility; M5B projects current runtime focus only to the focused owner's visible semantic PRIMARY | Public semantic-node RequestFocus ingress and accessibility resolution remain M5C | M4/M5B complete; M5C next |
| Semantic contribution and identity | `partial` | Owner-accepted platform-neutral `SemanticContribution`; 0..N owner-local nodes keyed by stable `SemanticKey`; roles, names/descriptions, values/states/action intent, relationships, plain text, owner/owner-local bounds; strict marker/reference validation; separate opaque semantic lifetimes reconciled by exact mounted owner + key; deterministic M5B publication resolves those contributions into exact public semantic identities | Public semantic-node action ingress, public testing queries/helpers, and native accessibility adapter remain absent | M5A–M5B complete; M5C–M5D sequential |
| Semantic publication and updates | `proof` | Independent `SurfaceId`-scoped `SemanticPublication` sibling with deterministic forest/preorder/exact-ID lookup, absolute logical bounds, resolved local/cross-owner relationships, composed state/support, runtime PRIMARY focus, typed diagnostics, non-wrapping revision, deterministic add/change/remove/focus deltas, and full-resync on wrong surface/base | M5 has one logical surface; public semantic action submission and native adapter remain later | M5B complete; M5C/M10 later |
| Surface publication | `proof` | Fallible staged `admit -> plan -> candidate-dependent final-preflight -> commit`; fresh coordinate/hit-test counters; recoverable stationary-rehit backpressure with zero partial commit; exact terminal publication-counter taxonomy; renderer products plus independent semantic publication/diagnostic siblings; explicit renderer-only versus complete extraction/equality | One mounted root and one logical surface; retained `SurfaceCache` still deep-clones narrow non-structural plans (#59); M6 owns the persistent renderer-neutral scene boundary | M5B complete; M6/M10 later |
| Hit testing | `proof` | Reverse-order rectangle hit testing over exact current or retained immutable publication snapshots feeds generation-safe physical pointer paths and stationary re-hit | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, or M6 pointer policy | M4 complete; M6 later |
| Debug and semantic consumption | `proof` | Renderer/debug products no longer carry production semantics; direct public semantic consumer and independent adapter-shaped consumer inspect the sibling semantic snapshot/update/diagnostic product | No native accessibility bridge or production paint scene/backend | M5B complete; M5C/M6/M10 later |
| Renderer-neutral paint scene | `absent` | M2 publishes deterministic open widget paint/debug facts | Proof facts are not primitives/resources and have no clips, transforms, layers, or damage | M6 |
| Production renderer backend | `absent` | None | No conventional or SDF backend; the neutral scene must be accepted first | M10, M12 |
| Deterministic text measurement | `proof` | Provider seam and Unicode-scalar-count measurements support headless tests | Fixed metrics are not font, shaping, grapheme, bidi, wrapping, or baseline layout | M8 |
| Production text subsystem | `absent` | None | No font discovery, shaping, fallback, bidi, wrapping, editing, selection, clipboard, or native IME integration | M8 |
| Button behavior | `proof` | Label/enabled state and repeatable typed `on_activate` factory invoked only by routed `Activate` default; programmatic, physical release-inside, raw Enter, and matched raw Space paths converge; built-in Button authors canonical semantic contribution and appears in the accepted independent M5B semantic product | Public semantic-node action resolution/accessibility, recipes, and production control breadth remain later | M4/M5A–M5B complete; M5C/M9 later |
| Standard control library | `absent` | None beyond text/button proofs | No complete lifecycle/event/semantic/style/layout/keyboard/accessibility contracts | M9 |
| Host neutrality | `usable` | Active core/runtime have no native window, GPU, ECS, platform-controller, AccessKit, or legacy dependencies | Neutrality alone is not a host integration contract | M10 |
| Host/platform integration | `absent` | Core application host protocol and runtime wake acknowledgment are host-neutral seams only | No native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, or multi-window adapter | M10 |
| Raw controller/gamepad platform input | `absent` | None | No device connection/identity, raw button/axis translation, normalization/dead zones, or embedded-host mapping | M10 |
| Testing and diagnostics | `partial` | Substantial proof-level integration tests; genuine downstream widget plus M5A/M5B semantic conformance; direct and adapter-shaped semantic consumers; deterministic clock/tasks; open layout/hit/paint inspection; strict lints; deterministic trace export/sink and accepted offline replay | No unified M5D public harness, stable public semantic query/action helpers, UI/scene snapshots, fuzzing, property tests, benchmarks, or platform tests | M5D; M11 |
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

M5 is active. M5A, the #55 readiness authority, and M5B feature implementation
are owner-accepted and merged. The exact accepted M5B feature squash is
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`, tree
`1708d2536c6f1d202ac58dd7cb5f3cc97a438517`; exact feature-head CI #1082 and
independent exact-squash CI #1084 attempt 2 passed.

This reconciliation records M5 conformance as `53 total / 31 owner-accepted / 0
implementation-complete / 0 proof-complete / 22 blocked`; aggregate configured
truth is `290 total / 266 owner-accepted / 0 implementation-complete / 0
proof-complete / 24 blocked`. The two inherited M4 blocked rows remain
`ACCESS-01` and `ACCESS-02`, owned by M5C.

M5C [#49](https://github.com/dornglut/runen-ui/issues/49) becomes eligible only
after this mandatory M5B post-merge reconciliation is itself owner-accepted,
merged, and accepted-main verified. The roadmap remains the durable milestone
authority.

Merged acceptance evidence belongs in pull requests and the
[public repository migration history](history/public-repository-migration.md).
Volatile branch, head, blocker, and next-action state belongs in the
[work-tracking system](work-tracking.md) and GitHub issues.
