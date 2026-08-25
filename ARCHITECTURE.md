# RunenUI Architecture

RunenUI is a host-neutral, renderer-neutral Rust UI framework. The durable ownership direction is:

```text
application state and actions
    -> transient typed View/Element descriptions
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> interaction / style / layout / semantics
    -> staged surface publication
         ├── renderer-facing products
         ├── hit/input products
         ├── semantic publication
         └── diagnostics
    -> host integration and renderer backend
```

The authored tree is transient reconciliation input. The mounted runtime tree is the persistent authority for runtime identity, widget-local state, lifecycle, invalidation, focus, interaction state, work ownership, and publication coordination.

Semantic identity is independently runtime-issued and published through a renderer-independent semantic product. It is not a mounted-arena alias and must not be folded into renderer scene authority. Surface publication is staged and atomic: rejected or terminally failed publication must not expose a partial new RunenUI-owned product.

## Workspace ownership

- `runenui_core` — host-neutral public application, authoring, geometry, style, event, effect, identity, and semantic protocol values.
- `runenui_runtime` — live mounted/semantic storage, reconciliation, routing, focus/input state, scheduling, tracing, publication, and shutdown.
- `runenui_testing` — downstream deterministic testing ergonomics over ordinary public core/runtime contracts.
- concrete hosts, platform adapters, renderer backends, and product state remain outside those ownership boundaries until real implementations justify their own edge contracts.

The workspace dependency and extraction rules are defined in [workspace structure](docs/architecture/workspace-structure.md).

## Current behavior and required contracts

Code and executable tests are the evidence for what the current implementation does. Accepted ADRs, architecture/design contracts, and conformance observations define what the implementation is required to do. A mismatch is a defect or requires an explicit reviewed contract revision; implementation never silently overrides accepted architecture.

Detailed current architecture is indexed under [docs/architecture](docs/architecture/README.md). Durable decisions live in [ADRs](docs/adr/). Permanent observable/proof contracts live under [conformance](docs/conformance/README.md). High-level dependency sequence lives in the [roadmap](docs/roadmap.md). Current accepted maturity is summarized in [status](docs/status.md).

Accepted future architecture becomes current API only after implementation and acceptance. M6 is accepted current behavior through retained publication, canonical renderer-neutral `PaintPublication`/`PaintScene`/`HitTestScene` ownership, composition/resource-reference/renderer-metadata/capability semantics, two independent deterministic consumers, public testing convergence, and proof-era paint/hit migration closure. M7A is accepted current edge behavior through the reusable wgpu renderer/resource implementation, real offscreen/readback and golden proof, provider/cache realization, and renderer observations. The remaining M7B/C/D host, accessibility, and external-host integration remains target architecture and is the next owner of those edge contracts.

Exact public Rust signatures remain authoritative in source and Rustdoc; conceptual public ownership is summarized in the [public API contract](docs/architecture/public-api.md).

Live issue, branch, pull-request, head, CI-run, blocker, and pickup state belongs in GitHub and is deliberately absent from this document.
