# RunenUI

RunenUI targets host-neutral, renderer-neutral Rust UI for headless, standalone, and embedded application profiles. The repository is pre-1.0 and currently provides a deterministic headless framework foundation plus a proof-level real host/renderer/accessibility production spine; it is not yet a production-complete desktop UI stack.

## Current accepted foundation

The implemented foundation includes:

- typed application state/action/update and transient `View`/`Element` authoring;
- persistent keyed mounted runtime state with checked generational identity and lifecycle;
- validated logical geometry plus an accepted production style environment/cascade over the current foreground/background/padding/radius property set, alongside proof-level measurement/layout, hit-test, and renderer-facing publication products;
- bounded effects, tasks, timers, subscriptions, host requests, deterministic clocks, wake/redraw, explicit pumping, trace/export/replay;
- canonical routed pointer, keyboard, committed-text, IME, focus, automation, and semantic-command interaction;
- independent semantic identity/publication/action ingress;
- public deterministic headless application testing through `runenui_testing`;
- an accepted reusable wgpu renderer/resource edge with real offscreen/readback and native presentation proof;
- reusable winit input and AccessKit translation exercised by standalone native hosts;
- a separate winit-free downstream host proof that owns pump/publication/render/present sequencing through ordinary public contracts.

The renderer-neutral paint/hit scene protocol is complete through M6 at proof maturity, the M7 reference production spine is complete at proof maturity through concrete renderer/resource, native host/input/accessibility, and external-host evidence, and M8A is accepted at partial styling maturity through typed themes/recipes/ordered variants, canonical interaction-state styling, explicit preferences, bounded inheritance, exact provenance/diagnostics, and effect-driven invalidation. Production text/editing, responsive layout and broader style-property breadth, standard controls, multi-window lifecycle, and supported platform breadth remain later roadmap outcomes.

See [current status](docs/status.md) for capability maturity and [roadmap](docs/roadmap.md) for durable sequencing.

## Workspace

```text
runenui_core <- runenui_runtime
       ^             ^
       └──────┬──────┘
              ├── runenui_render_wgpu ──┬── reference_winit
              │                         ├── counter
              │                         └── external-host conformance
              ├── runenui_winit ────────┬── reference_winit
              │                         └── counter
              ├── runenui_testing
              ├── external-renderer conformance
              └── external-widget conformance
```

- `runenui_core` owns host-neutral public values and protocols.
- `runenui_runtime` owns live framework authority.
- `runenui_render_wgpu` is the reusable concrete renderer/resource edge over ordinary public paint publication.
- `runenui_winit` owns reusable winit input and AccessKit translation/projection, not a host loop or renderer.
- `runenui_testing` is a downstream public testing convenience layer.
- `tests/external_host` is an unpublished winit-free downstream host conformance consumer over public core/runtime/renderer contracts.
- `tests/external_renderer` is an unpublished downstream renderer-neutral conformance consumer over ordinary public core/runtime scene contracts.
- `xtask` owns repository validation tooling and has no framework dependency.

See [workspace structure](docs/architecture/workspace-structure.md) for the enforced ownership/dependency contract.

## Validation

```text
cargo validate
```

For intentional Rust edits, format first with:

```text
cargo +stable fmt --all
```

Focused tests and conformance proofs remain required for the active change. See [TESTING.md](TESTING.md) and [validation details](docs/tooling/validation.md).

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Documentation index](docs/README.md)
- [Current status](docs/status.md)
- [Roadmap](docs/roadmap.md)
- [Public API contract](docs/architecture/public-api.md)
- [ADRs](docs/adr/)
- [Conformance](docs/conformance/README.md)
- [API stability](docs/api-stability.md)
- [Release policy](docs/release-policy.md)

Live work, blockers, pull-request state, exact heads, and CI evidence belong in GitHub rather than durable documentation.

## License

RunenUI is currently publicly licensed under [`GPL-3.0-only`](LICENSE). A separate commercial-license model may be available from copyright holder(s) with sufficient rights; see [`LICENSING.md`](LICENSING.md). Until reviewed inbound terms preserve commercial relicensing authority, external PRs contributing tracked repository content are not accepted. Issue reports, design discussion, reviews, and reproducible cases may still be accepted.
