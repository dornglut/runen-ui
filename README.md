# RunenUI

RunenUI is a pre-1.0 Rust-native UI framework for deterministic headless testing, standalone desktop applications, and embedding in host-owned engines or editors. It is designed to remain host-neutral and renderer-neutral: application behavior, mounted UI state, semantics, paint/hit products, platform integration, and rendering have explicit ownership boundaries.

RunenUI is currently a coherent **headless framework proof**, not a production desktop toolkit. The accepted foundation covers typed application state/actions, transient view authoring, persistent mounted identity and reconciliation, routed interaction, deterministic effects/scheduling, renderer-independent semantics, and public deterministic testing. Production paint/hit scenes, native hosts, renderer backends, international text, and a complete control library are still later roadmap work.

## Current capabilities

- typed `UiApp` state/action/update flow and transient `View`/`Element` authoring;
- open downstream `Widget` participation with persistent runtime-owned mounted state;
- generational mounted identity, keyed reconciliation, lifecycle, focus, pointer/keyboard/text/IME routing, and deterministic command processing;
- deterministic effects, tasks, timers, subscriptions, host requests, wake/redraw signaling, bounded trace export, and inert replay;
- renderer-independent semantic contribution, identity, publication, updates, actions, diagnostics, and exact surface-scoped targeting;
- typed style and proof-level layout/measurement with explicit invalidation and staged surface publication;
- downstream `runenui_testing::TestHarness` using ordinary public runtime APIs with deterministic time, bounded settling, synthetic interaction, semantic queries, and read-only inspection.

Decisive limitations remain: there is no production renderer-neutral paint/hit scene yet, no native window/event-loop adapter, no renderer backend, no production shaping/editing text stack, no standard production control library, and no multi-window lifecycle. See [current status](docs/status.md) for the maintained maturity map.

## Workspace

The active workspace keeps dependency direction explicit:

```text
runenui_core <- runenui_runtime
       ^             ^
       └──────┬──────┘
              ├── runenui_testing
              ├── counter example
              └── external-widget conformance

xtask  (repository tooling only)
```

- `runenui_core` owns host-neutral public values and protocols.
- `runenui_runtime` owns live mounted/semantic state, routing, scheduling, publication, and runtime mutation.
- `runenui_testing` is downstream public testing ergonomics and owns no live runtime authority.

The detailed ownership rules are in [Architecture](ARCHITECTURE.md) and [workspace structure](docs/architecture/workspace-structure.md).

## Validation

The canonical merge-readiness command is:

```text
cargo validate
```

It is repository-owned, deterministic, read-only, and used by CI. Intentional Rust edits should be formatted with `cargo +stable fmt --all` before validation. See [Testing](TESTING.md) and [validation details](docs/tooling/validation.md).

## Documentation

Start with the [documentation index](docs/README.md). The main durable authorities are:

- [Architecture](ARCHITECTURE.md) — concise system and ownership map;
- [Current status](docs/status.md) — accepted capability maturity and decisive limits;
- [Roadmap](docs/roadmap.md) — durable outcome sequence and dependencies;
- [ADRs](docs/adr/) — accepted durable decisions;
- [Conformance](docs/conformance/README.md) — permanent observable/proof contracts;
- [API stability](docs/api-stability.md) and [release policy](docs/release-policy.md);
- [Testing](TESTING.md) and [repository tooling](docs/tooling/).

Live work, branch state, pull requests, validation runs, and delivery evidence belong in GitHub rather than durable Markdown. Repository-wide governance defaults are maintained by [Dornglut Engineering](https://github.com/dornglut/engineering).

RunenUI is licensed under the [MIT License](LICENSE).
