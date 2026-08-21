# RunenUI

RunenUI targets host-neutral, renderer-neutral Rust UI for headless, standalone, and embedded application profiles. The repository is pre-1.0 and currently provides a deterministic **headless framework foundation**, not a production desktop UI stack.

## Current accepted foundation

The implemented foundation includes:

- typed application state/action/update and transient `View`/`Element` authoring;
- persistent keyed mounted runtime state with checked generational identity and lifecycle;
- validated logical geometry, typed style tokens, proof-level measurement/layout, hit-test, and renderer-facing publication products;
- bounded effects, tasks, timers, subscriptions, host requests, deterministic clocks, wake/redraw, explicit pumping, trace/export/replay;
- canonical routed pointer, keyboard, committed-text, IME, focus, automation, and semantic-command interaction;
- independent semantic identity/publication/action ingress;
- public deterministic headless application testing through `runenui_testing`.

The current renderer-facing products are proof infrastructure. Production renderer-neutral paint/hit scenes, concrete native hosts/backends, production text/editing, full layout/style breadth, standard controls, and multi-window lifecycle remain later roadmap outcomes.

See [current status](docs/status.md) for capability maturity and [roadmap](docs/roadmap.md) for durable sequencing.

## Workspace

```text
runenui_core <- runenui_runtime
       ^             ^
       └──────┬──────┘
              ├── runenui_testing
              ├── counter example
              └── external-widget conformance
```

- `runenui_core` owns host-neutral public values and protocols.
- `runenui_runtime` owns live framework authority.
- `runenui_testing` is a downstream public testing convenience layer.
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
