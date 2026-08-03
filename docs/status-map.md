# RunenUI Status Map

> **Category: Current contract**

This map reports the maturity of the active implementation at the current revision. It does not treat types, target documents, or historical code as implemented behavior.

M4C5 is proof-complete on this review branch for its fifteen owned conformance
rows, but it is not owner-accepted or merged. The entries below describe that
implemented proof without promoting it to accepted support; independent review,
owner acceptance, merge, and the separate M4C5 authority-reconciliation PR
remain required before M4D1 can begin. Exact branch-head validation evidence
belongs in the pull request and work-tracking system.

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
| Application model | `proof` | Core-owned `UiApp`; application state/actions; ordered initial/update effects; state-derived subscriptions; typed host protocol; action/command/pointer/keyboard/text/composition submission through one generalized FIFO | One mounted root; M4C5 input proof awaits owner acceptance and has no native-host translation | M4B/M4C1 complete; M4C2/M4C3/M4C4 owner-accepted; M4C5 proof-complete |
| Mounted runtime identity and indexing | `usable` | One core-owned opaque namespace backs generational `MountedNodeId`, distinct `SemanticNodeId`, and opaque `SurfaceId`/`SurfaceInputContext`; logical-preorder `MountedTreeIndex`; foreign/stale/missing distinction; authored-ID diagnostics | Runtime-local, process-local, non-serialized, and currently one logical surface | M3/M4C1 complete; M4C2 owner-accepted |
| Keyed reconciliation | `usable` | Transactional sibling-local matching, unkeyed ordinal policy, structured duplicate-key no-reuse diagnostics, cross-parent remount, exact reports | Cross-parent movement remounts; duplicate keys preserve no ambiguous lifetime | M3 complete |
| Persistent widget state and lifecycle | `usable` | State-aware lifecycle, activation, routed-event, exact-generation focus and composition ownership, pointer/Space cleanup, and deterministic shutdown capabilities | M4C5 input proof awaits owner acceptance; editable text state remains absent | M3/M4B/M4C1 complete; M4C2/M4C3/M4C4 owner-accepted; M4C5 proof-complete |
| Events and interaction | `proof` | Core-owned semantic-command, pointer, focus, keyboard, committed-text, and composition protocols; checked downstream C/T/B; atomic focus out/in; exact focused binding; propagation/default control; conservative admission; release-inside activation; deterministic automation resolution | No native host translation, production scrolling/editable text, semantic accessibility resolution, or accepted M4C5 support | M4C1/M4C2/M4C3/M4C4 owner-accepted; M4C5 proof-complete |
| Normalized UI navigation commands | `proof` | Activation/menu commands, pointer-derived logical scroll, linear/directional focus, request/restore, and focus logical-scroll derivation share the canonical FIFO/routing/trace across normalized sources | Production scroll mutation and source-specific target resolution remain later | M4C1 complete; M4C3/M4C4 owner-accepted; M4C5/M5 later |
| Directional/spatial focus navigation | `proof` | Current published rectangles, nearest live scope, current eligibility, exact generation, and mounted-order tie-break satisfy DF-01–DF-20 through public commands | Private score is deliberately not public; multi-surface transfer remains absent | M4C4 owner-accepted |
| Effects and scheduling | `proof` | One atomic planner; live-only generational producers; routed exact-owner work output; before-unmount revocation; `Starting -> Running` send subscriptions; tombstone-free host authority; checked mandatory trace plans; once-claimed serialized wake callbacks | Runtime supplies executor/source adapters rather than a default thread pool | M4B and M4C1 complete |
| Styling | `partial` | Colors, padding, radius, typed tokens, computed style, provenance, missing-token diagnostics | No themes, recipes, variants, interaction states, typography, borders, fallback, or preferences | M7–M8 |
| Layout and measurement | `proof` | Separate one-query intrinsic/child-layout snapshots; component-wise minimum combination; padding/gaps; unsupported/unknown fallbacks; aligned index/frame/style/layout products | Linear M2 proof only; no production sizing/alignment/flex/grid/scroll/incremental layout | M7–M8 |
| Focusability facts | `proof` | Widgets declare automatic/explicit/hidden focusability and nested scope policy; runtime revalidates live enabled eligibility | This is not the M5 semantic accessibility model | M4C4 owner-accepted; M5 later |
| Semantic/accessibility tree | `absent` | Widgets publish minimal deterministic role/name/enabled/action-intent proof facts and mounted lifetimes expose `SemanticNodeId` | Proof facts are not a production semantic tree and have no relationships, values, semantic actions, AccessKit adapter, or accessibility claim | M5, M10 |
| Surface publication | `proof` | Context-bearing publication with fresh coordinate revision/displayed hit-test generation, topology-only renderer-product cache, current mounted style/layout reads, exact token-content key, bounded immutable snapshot retention, and independent phase-entry/report proofs | One mounted root and one logical surface; retained input snapshots are not production retained layout or M5/M6 products | M4C2 owner-accepted; M6/M10 later |
| Hit testing | `proof` | Reverse-order rectangle hit testing over exact current or retained immutable publication snapshots feeds generation-safe physical pointer paths and stationary re-hit | No explicit hit scene, stacking contract, clips, transforms, visibility/inertness, or M6 pointer policy | M4C2/M4C3 owner-accepted; M6 later |
| Debug/semantic frame consumption | `proof` | Deterministic text rendering includes open widget paint/semantic/diagnostic proof facts | Debug output is not a paint scene, semantic tree, accessibility product, or backend | M5–M6 |
| Renderer-neutral paint scene | `absent` | M2 publishes deterministic open widget paint/debug facts | Proof facts are not primitives/resources and have no clips, transforms, layers, or damage | M6 |
| Production renderer backend | `absent` | None | No conventional or SDF backend; the neutral scene must be accepted first | M10, M12 |
| Deterministic text measurement | `proof` | Provider seam and Unicode-scalar-count measurements support headless tests | Fixed metrics are not font, shaping, grapheme, bidi, wrapping, or baseline layout | M8 |
| Production text subsystem | `absent` | None | No font discovery, shaping, fallback, bidi, wrapping, editing, selection, clipboard, or IME | M8 |
| Button behavior | `proof` | Label/enabled state and repeatable typed `on_activate` factory invoked only by routed `Activate` default; programmatic, physical release-inside, raw Enter, and matched raw Space paths converge | M4C5 keyboard proof awaits owner acceptance; no production semantics, recipes, or accessibility contract | M4C1 complete; M4C3/M4C4 owner-accepted; M4C5 proof-complete; M9 later |
| Standard control library | `absent` | None beyond text/button proofs | No complete lifecycle/event/semantic/style/layout/keyboard/accessibility contracts | M9 |
| Host neutrality | `usable` | Active core/runtime have no native window, GPU, ECS, platform-controller, or legacy dependencies | Neutrality alone is not a host integration contract | M10 |
| Host/platform integration | `absent` | Core application host protocol and runtime wake acknowledgment are host-neutral seams only | No native event loop/window, DPI, clipboard, cursor, IME, drag/drop, accessibility bridge, or multi-window adapter | M4, M10 |
| Raw controller/gamepad platform input | `absent` | None | No device connection/identity, raw button/axis translation, normalization/dead zones, or embedded-host mapping | M10 |
| Testing and diagnostics | `partial` | Substantial proof-level integration tests; genuine downstream widget conformance; deterministic clock/tasks and open widget/style/layout inspection; strict lints | No unified M5 public harness, stable semantic queries, replay, snapshots, fuzzing, property tests, benchmarks, or platform tests | M4–M5, M11 |
| Trace and observability | `partial` | One bounded canonical sequence covers scheduler, routed command, surface, pointer, focus selection/policy/restoration/transition/notification/focus-within, modality, and M4C5 keyboard/text/composition/automation facts | M4C5 proof awaits owner acceptance; no normalized trace-v2 schema, external sink, JSONL export, or replay | M4B/M4C1/M4C2/M4C3/M4C4 owner-accepted; M4C5 proof-complete; M4D blocked |
| Source formats and devtools | `deferred` | Context-export tooling only; no UI source system | No parser, source mapping, inspector, hot reload, live preview, or visual authoring | M12 |
| Advanced editor/game systems | `deferred` | Product direction only | No virtualization, advanced data controls, animation, overlays, docking, workspaces, or advanced multi-surface systems | M12 |
| Legacy archive | `archived` | Git history and annotated tag `legacy-runenwerk-ui-archive-2026-07-11` preserve the audited tree | Removed from active content; salvage remains opt-in and must follow current architecture | M0B complete |

## Current milestone

M0–M3, M4A, M4B, and M4C0–M4C4 are complete. ADR 0005 remains routed-behavior authority, ADR 0006 remains scheduler-behavior authority, the accepted [M4C delivery charter](architecture/m4c-delivery-and-routed-transaction-charter.md) owns implementation and delivery constraints, and the [M4 conformance matrix](architecture/m4-conformance-matrix.md) owns observable acceptance.

M4C5's fifteen owned rows are proof-complete on the implementation review
branch, not owner-accepted. Keyboard, committed-text, composition, and
authored-ID automation behavior therefore remains pending independent review,
owner acceptance, merge, and the separate M4C5 authority-reconciliation PR.
M4D1–M4D3 remain blocked in sequence; semantic accessibility mapping, export,
sink, and replay are not upgraded. M4 remains active and incomplete.

Merged acceptance evidence belongs in pull requests and the [public repository migration history](history/public-repository-migration.md). Volatile branch, head, blocker, and next-action state belongs in the [work-tracking system](work-tracking.md) and GitHub issues.
