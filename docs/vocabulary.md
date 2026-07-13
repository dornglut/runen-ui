# Vocabulary

> **Category: Current contract**

This vocabulary marks current and target terms explicitly. Target terms do not imply an implemented API.

## Current terms

| Term | Meaning |
|---|---|
| State | Application-owned durable data used to derive UI. |
| Action | Application-owned typed intent passed to `update`. |
| `update` | Application function that mutates state in response to one action. |
| `View<Action>` | Public conversion protocol for a typed transient authored value that produces one erased element. |
| `Element<Action>` | Owned transient erased node derived from state; it is consumed as reconciliation input and never retained as parallel runtime authority. |
| `Text`, `Button<Action>` | Typed built-in authored views that transfer common facts into elements and install private behavior-only widgets. |
| `Container<Action>` | Canonical typed authored view for any built-in or downstream `ChildLayoutWidget`; owns children and container-only gap before atomic erasure. |
| `Widget<Action>` | Concrete state-aware runtime participant declaring persistent state, lifecycle, activation, measurement, paint, semantic, and diagnostic proof behavior. |
| `ChildLayoutWidget<Action>` | Widget contract required for child ownership; contributes one `ChildLayout` policy independently of intrinsic measurement. |
| `ChildLayout` | Non-exhaustive M2 child arrangement proof; currently linear by axis, with descendant-preserving vertical fallback for future unknown variants. |
| Component | Ordinary Rust view composition that may use a local action and recursively map it into a parent action; it is not automatically mounted identity or state. |
| `element!` / `children!` | Thin `View` erasure and arity-free heterogeneous child collection; no parallel property grammar. |
| `LogicalLength` | Finite, non-negative device-independent distance; host scale factors later map logical to physical pixels. |
| `ElementId` | Unicode-validated optional authored debug/test/automation handle with text-based identity across static/owned storage; tree-wide duplicates are diagnosed. |
| `ElementKey` | Unicode-validated sibling-local reconciliation key; unique keyed siblings preserve mounted lifetime across reorder, while duplicates preserve no ambiguous state. |
| `TokenId` | Unicode-validated textual token identity; static literals and dynamic construction compare, order, and hash identically. |
| `UiApp` / `AppRuntime` | Current headless application contract and bound runtime wrapper. |
| `MountedNodeId` | Non-`Copy`, process-local and runtime-instance-local `(Arc token, arena slot, generation)` identity; not authored, semantic, serialized, or preorder identity. |
| `SemanticNodeId` | Distinct read-only identity namespace sharing one mounted lifetime triplet; foundation only, not the M5 semantic tree or accessibility identity contract. |
| `MountedTreeIndex` / `MountedNodeRef` | Read-only logical-mounted-preorder inspection; arena slot order is never traversal order. |
| `WidgetTypeId` | Process-local wrapped Rust `TypeId` of a concrete widget implementation; separate from authored and runtime identity and not serialized. |
| `WidgetStateTypeId` | Process-local declared widget-state type fact used with widget implementation type for compatibility. Persistent erased state is private runtime plumbing. |
| `WidgetInvalidation` | Public manual bitset selecting interaction, layout, paint, semantic, and diagnostic capability invalidation. |
| `ReconciliationGeneration` / `ReconciliationReport` | Non-forgeable completed-generation identity, exact mounted lifetime/update/move counts, and structured reconciliation diagnostics. |
| `LayoutConstraints` | Normalized finite/unbounded measurement limits. |
| `MeasurementProvider` | Borrowed synchronous intrinsic text-measurement seam with explicit stable cache identity and behavior revision. |
| `SurfacePublication` | One publication containing aligned frame, style report, and layout report. |
| `SurfacePhaseReport` | Inspectable record of proof-level tree/style/layout/hit-test/paint/semantics/diagnostics/focus work executed by the latest runtime operation. |
| `SurfaceFrame` | Current bounds/style plus open paint/semantic/diagnostic proof product; not a paint scene or accessibility tree. |
| Trace | Current proof-level record of mount/action/update/reconcile/focus/shutdown events; trace v2 remains M4. |

`on_press` is the only current button-action term. `map_action` is typed and
recursive. `element!` accepts the same
builder expression as direct authoring and introduces no separate binding names.
Identifiers reject empty or Unicode-whitespace-only text, surrounding Unicode
whitespace, and Unicode control characters while accepting ordinary Unicode.

## Accepted target terms

| Term | Meaning |
|---|---|
| Multi-surface runtime | Later support for multiple mounted roots, independent focus domains, surface lifecycle, and per-surface publication generations; M3 has one of each domain. |
| Effect | Typed request for work executed by the runtime/host after update. |
| Semantic tree | Renderer-independent accessibility/automation roles, state, relationships, and actions. |
| Layout result | Computed geometry/baselines/extents independent of paint and semantics. |
| Hit-test scene | Ordered hit shapes, clips, transforms, visibility, inertness, and pointer policy. |
| Paint scene | Renderer-neutral primitives and resource references with order, clips, transforms, and metadata. |
| Host | Owner of platform/window lifecycle, normalized events, services, timing, resources, and wakeups. |
| Renderer backend | Consumer of paint primitives/resources; never owner of widget semantics or behavior. |
