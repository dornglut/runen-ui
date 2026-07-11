# Vocabulary

> **Category: Current contract**

This vocabulary marks current and target terms explicitly. Target terms do not imply an implemented API.

## Current terms

| Term | Meaning |
|---|---|
| State | Application-owned durable data used to derive UI. |
| Action | Application-owned typed intent passed to `update`. |
| `update` | Application function that mutates state in response to one action. |
| `Element<Action>` | Immutable transient UI description derived from state. |
| `element!` | Optional macro sugar for the current built-in element grammar. |
| `ElementId` | Optional authored debug/test/automation handle; uniqueness is not yet validated. |
| `ElementKey` | Stored authored key intended for future reconciliation; currently does not preserve identity. |
| `UiApp` / `AppRuntime` | Current headless application contract and bound runtime wrapper. |
| `RuntimeNodeId` | Preorder index valid for one built tree; not persistent identity. |
| `LayoutConstraints` | Normalized finite/unbounded measurement limits. |
| `MeasurementProvider` | Borrowed synchronous intrinsic text-measurement seam. |
| `SurfacePublication` | One publication containing aligned frame, style report, and layout report. |
| `SurfaceFrame` | Current semantic/bounds/style proof product; not a mature paint protocol. |
| Trace | Current coarse headless record of mount/action/update/rebuild events. |

The implemented button binding is `action=` in `element!`; builder calls use `on_press`. There is no current `on_change` binding.

## Accepted target terms

| Term | Meaning |
|---|---|
| View | Transient declarative description reconciled into mounted widgets; exact public API requires an ADR. |
| Mounted tree | Persistent generational runtime identity, lifecycle, local state, and invalidation authority. |
| Component | View composition that may map local actions into parent actions; not necessarily retained. |
| Widget/control | Mounted lifecycle participant with events, layout, paint, semantics, and runtime-local state. |
| Effect | Typed request for work executed by the runtime/host after update. |
| Semantic tree | Renderer-independent accessibility/automation roles, state, relationships, and actions. |
| Layout result | Computed geometry/baselines/extents independent of paint and semantics. |
| Hit-test scene | Ordered hit shapes, clips, transforms, visibility, inertness, and pointer policy. |
| Paint scene | Renderer-neutral primitives and resource references with order, clips, transforms, and metadata. |
| Host | Owner of platform/window lifecycle, normalized events, services, timing, resources, and wakeups. |
| Renderer backend | Consumer of paint primitives/resources; never owner of widget semantics or behavior. |
