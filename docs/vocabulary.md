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
| `Element<Action>` | Owned transient erased node derived from state; it contains common authored facts, children, and one safely erased widget implementation. Borrowed inspection is non-consuming, while action extraction is explicitly mutable. |
| `Text`, `Button<Action>` | Typed built-in authored views that transfer common facts into elements and install private behavior-only widgets. |
| `Container<Action>` | Canonical typed authored view for any built-in or downstream `ChildLayoutWidget`; owns children and container-only gap before atomic erasure. |
| `Widget<Action>` | Concrete runtime-participant contract declaring state and bounded activation/measurement/paint/semantic/diagnostic/lifecycle proof behavior. |
| `ChildLayoutWidget<Action>` | Widget contract required for child ownership; contributes one `ChildLayout` policy independently of intrinsic measurement. |
| `ChildLayout` | Non-exhaustive M2 child arrangement proof; currently linear by axis, with descendant-preserving vertical fallback for future unknown variants. |
| Component | Ordinary Rust view composition that may use a local action and recursively map it into a parent action; it is not automatically mounted identity or state. |
| `element!` / `children!` | Thin `View` erasure and arity-free heterogeneous child collection; no parallel property grammar. |
| `LogicalLength` | Finite, non-negative device-independent distance; host scale factors later map logical to physical pixels. |
| `ElementId` | Unicode-validated optional authored debug/test/automation handle with text-based identity across static/owned storage; tree-wide duplicates are diagnosed. |
| `ElementKey` | Unicode-validated authored sibling key with text-based identity and duplicate diagnostics; it does not yet preserve mounted identity. |
| `TokenId` | Unicode-validated textual token identity; static literals and dynamic construction compare, order, and hash identically. |
| `UiApp` / `AppRuntime` | Current headless application contract and bound runtime wrapper. |
| `RuntimeNodeId` | Preorder index valid for one built tree; not persistent identity. |
| `WidgetTypeId` | Process-local wrapped Rust `TypeId` of a concrete widget implementation; separate from authored and runtime identity and not serialized. |
| `WidgetStateTypeId` / `WidgetState` | Checked process-local state contract and opaque value used by the M2 conformance seam; not persistent storage. |
| `LayoutConstraints` | Normalized finite/unbounded measurement limits. |
| `MeasurementProvider` | Borrowed synchronous intrinsic text-measurement seam. |
| `SurfacePublication` | One publication containing aligned frame, style report, and layout report. |
| `SurfaceFrame` | Current bounds/style plus open paint/semantic/diagnostic proof product; not a paint scene or accessibility tree. |
| Trace | Current coarse headless record of mount/action/update/rebuild events. |

`on_press` is the only current button-action term. `map_action` is typed and
recursive. `element!` accepts the same
builder expression as direct authoring and introduces no separate binding names.
Identifiers reject empty or Unicode-whitespace-only text, surrounding Unicode
whitespace, and Unicode control characters while accepting ordinary Unicode.

## Accepted target terms

| Term | Meaning |
|---|---|
| Mounted tree | Persistent generational runtime identity, lifecycle, local state, and invalidation authority. |
| Mounted widget | Persistent M3 instance pairing reconciled identity with a compatible widget implementation and retained runtime-local state. |
| Effect | Typed request for work executed by the runtime/host after update. |
| Semantic tree | Renderer-independent accessibility/automation roles, state, relationships, and actions. |
| Layout result | Computed geometry/baselines/extents independent of paint and semantics. |
| Hit-test scene | Ordered hit shapes, clips, transforms, visibility, inertness, and pointer policy. |
| Paint scene | Renderer-neutral primitives and resource references with order, clips, transforms, and metadata. |
| Host | Owner of platform/window lifecycle, normalized events, services, timing, resources, and wakeups. |
| Renderer backend | Consumer of paint primitives/resources; never owner of widget semantics or behavior. |
