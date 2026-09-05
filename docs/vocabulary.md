# Vocabulary

This glossary provides short orientation definitions. Detailed semantics belong to the canonical architecture, ADR, conformance, or source owner linked from the surrounding documentation; glossary text does not duplicate those contracts.

| Term | Meaning |
|---|---|
| State | Application-owned durable data used to derive UI. |
| Action | Application-owned typed intent processed by `UiApp::update`. |
| `View<Action>` | Public conversion protocol for a typed transient authored value. |
| `Element<Action>` | Owned transient erased reconciliation input; never persistent runtime state. |
| Component | Ordinary typed Rust composition; not automatically mounted identity or local state. |
| `Widget<Action>` | State-aware runtime participant using the public widget/lifecycle/event contracts. |
| `ChildBearingWidget<Action>` | Geometry-neutral marker for widgets whose elements may structurally own children; runtime layout determines their geometry. |
| `ElementId` | Optional validated authored debug/test/automation identity; not mounted or semantic identity. |
| `ElementKey` | Validated sibling-local reconciliation key used to retain compatible mounted lifetime across reorder. |
| `TokenId` | Validated textual style-token identity. |
| `LogicalPoint` / `LogicalSize` / `LogicalRect` | Host-neutral logical geometry values shared by framework contracts. |
| `UiApp` | Core-owned application state/action/update protocol. |
| `AppRuntime` | Runtime-owned live execution wrapper for one application. |
| `MountedNodeId` | Opaque runtime-local generational identity for one mounted widget lifetime. |
| `SemanticNodeId` | Distinct opaque runtime-local generational identity for one semantic lifetime. |
| `SemanticKey` | Stable owner-local authored semantic key used to reconcile semantic identity. |
| Semantic contribution | Platform-neutral owner-local semantic description authored by a widget. |
| Semantic publication | Independent surface-scoped renderer-neutral semantic snapshot/update authority. |
| `SemanticAction` | Platform-neutral semantic action vocabulary accepted by the current semantic contract. |
| `SemanticActionRequest` | Exact surface + semantic-node + semantic-action ingress request. |
| `SurfaceId` | Opaque logical surface identity. |
| `SurfaceInputContext` | Runtime-issued displayed-generation context used to validate surface input against exact publication history. |
| Surface publication | One coherently committed set of renderer/input/semantic/diagnostic products for a surface. |
| Staged publication | Admission → staged/read-only plan → candidate-dependent preflight → commit transaction used to avoid partial publication. |
| `WorkSequence` | Runtime-issued non-wrapping identity for accepted sequenced work. |
| Pump | Explicit bounded runtime operation that processes queued/readiness work. |
| Pump budget | Caller-supplied limits for bounded progress families; the runtime does not hide an unbounded settle loop. |
| Work owner | Application lifetime or exact mounted lifetime responsible for owned asynchronous/runtime work. |
| `WorkKey` | Owner-local durable cancellation/replacement identity for keyed work. |
| Trace | One bounded canonical causal observation sequence for accepted runtime behavior. |
| Replay | Inert offline interpretation of retained/exported trace facts; never live runtime authority. |
| Focus authority | Runtime-owned exact mounted focus state; semantic focus is a projection, not a second model. |
| Command origin | Normalized source/derivation metadata for routed command work. |
| Conformance ID | Permanent identifier for one observable/proof obligation in `docs/conformance/`. |
| Proof product | Current deterministic renderer/layout/hit/etc. evidence used by the headless foundation; not automatically a production protocol. |
| Target architecture | Accepted future contract that does not imply current implementation or public API. |

The current architecture is indexed in [architecture](architecture/README.md), exact public ownership in [public API](architecture/public-api.md), current maturity in [status](status.md), and permanent observable/proof vocabulary under [conformance](conformance/README.md).
