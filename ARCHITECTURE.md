# RunenUI Architecture

RunenUI is a host-neutral, renderer-neutral Rust UI framework. The durable ownership direction is:

```text
application state and actions
    -> transient typed View/Element descriptions
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> interaction / style / layout / semantics
         ├── renderer-neutral logical text measurement/resources
         └── runtime-owned deterministic motion sampling
    -> staged surface publication
         ├── renderer-facing products
         ├── hit/input products
         ├── semantic publication
         └── diagnostics
    -> host integration and renderer backend
```

The authored tree is transient reconciliation input. The mounted runtime tree is the persistent authority for runtime identity, widget-local state, lifecycle, invalidation, focus, interaction state, work ownership, layout orchestration, and publication coordination.

Semantic identity is independently runtime-issued and published through a renderer-independent semantic product. It is not a mounted-arena alias and must not be folded into renderer scene authority. Surface publication is staged and atomic: rejected or terminally failed publication must not expose a partial new RunenUI-owned product.

Application state owns durable domain data, including editable text documents and revisions. `runenui_core` owns host-neutral authoring, interaction, layout, style, visual, semantic, editing, and motion contracts; `runenui_text` shapes logical text and derives immutable caret/preedit maps from the same retained text layout used for measurement and paint.

`runenui_runtime` owns mounted/semantic/editing-session lifetimes, the sequenced input/action/work queue, routed defaults and focus, style/layout/text coordination, deterministic motion sampling, and staged surface publication. It does not store application documents or create a second editor, event queue, layout topology, or semantic model.

Paint, hit/input, semantic, and diagnostic publications are distinct sibling products committed coherently. Presentation transforms use the same runtime sample for visible paint, physical hit testing, directional focus, and semantic geometry. `runenui_render_wgpu` consumes immutable paint publications and owns only disposable GPU realization; `runenui_winit` translates native input and projects accessibility without becoming a runtime authority.

## Workspace ownership

- `runenui_core` — host-neutral public application, authoring, geometry, style, layout, event, effect, identity, semantic, application-owned transactional-editing, renderer-neutral visual/composition, and deterministic motion description/sampling protocol values.
- `runenui_text` — renderer-neutral production font-source policy, shaping/line breaking, text-specific constraints, immutable logical text artifacts, shaped-resource bindings, and retained-layout caret/preedit maps behind RunenUI-owned contracts.
- `runenui_runtime` — live mounted/semantic/editing-session storage, reconciliation, routing, focus/input/edit-chain state, scheduling, style/text/layout orchestration, deterministic motion lifecycle/sampling, correlated visual/hit/focus/semantic geometry, visual/composition publication, tracing, publication, and shutdown.
- `runenui_testing` — downstream deterministic testing ergonomics over ordinary public core/runtime contracts, including explicit manual-time integrated proof without private runtime authority.
- concrete hosts, platform adapters, renderer backends, and product state remain outside those ownership boundaries; accepted edge implementations do not move their authority into core/runtime/text.

The workspace dependency and extraction rules are defined in [workspace structure](docs/architecture/workspace-structure.md).

## Current behavior and required contracts

Code and executable tests are the evidence for what the current implementation does. Accepted ADRs, architecture/design contracts, and conformance observations define what the implementation is required to do. A mismatch is a defect or requires an explicit reviewed contract revision; implementation never silently overrides accepted architecture.

Detailed current architecture is indexed under [docs/architecture](docs/architecture/README.md). Durable decisions live in [ADRs](docs/adr/). Permanent observable/proof contracts live under [conformance](docs/conformance/README.md). High-level dependency sequence lives in the [roadmap](docs/roadmap.md). Current accepted maturity is summarized in [status](docs/status.md).

Accepted target ADRs do not establish shipped API or native-platform qualification. [Current status](docs/status.md) records accepted maturity; [conformance](docs/conformance/README.md) retains permanent behavioral and proof obligations.

Exact public Rust signatures remain authoritative in source and Rustdoc; conceptual public ownership is summarized in the [public API contract](docs/architecture/public-api.md).

Live issue, branch, pull-request, head, CI-run, blocker, and pickup state belongs in GitHub and is deliberately absent from this document.
