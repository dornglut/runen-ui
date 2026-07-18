# RunenUI

> **Category: Current contract**

RunenUI is a pre-1.0 Rust-native UI framework project. Its production goal is a stable, extensible UI kernel for deterministic headless testing, standalone desktop applications on Windows, macOS, and Linux, and embedding in engine- or editor-owned hosts.

Today RunenUI is a coherent **headless architecture proof**. It is not a production UI framework, native desktop toolkit, renderer backend, or finished control library. Current APIs are experimental and may change incompatibly while the foundations are corrected.

Milestone status: M4A and M4B are complete. M4C0 documentation/conformance
alignment is complete and owner-accepted. M4C1 is queued as the next
implementation slice; M4C2–M4C5 and M4D1–M4D3 remain blocked in sequence, and
M4 is not complete. See the [roadmap](docs/roadmap.md), [status map](docs/status-map.md),
[accepted M4C delivery charter](docs/architecture/m4c-delivery-and-routed-transaction-charter.md),
and [M4 conformance matrix](docs/architecture/m4-conformance-matrix.md).

## What exists today

The active workspace proves:

- application-owned state and typed actions with explicit `update`;
- typed transient views erased into open `Element<Action>` trees, ordinary
  component functions, recursive typed action mapping, and builder/`element!`
  authoring;
- downstream state-aware `Widget<Action>` implementations with process-local
  widget/state identity, safe checked erasure, mounted lifecycle contexts,
  selective invalidation, child-bearing construction through
  `ChildLayoutWidget` and `Container<Action>`, and the same protocol used by
  private built-in widget implementations;
- a persistent generational mounted tree with sibling-local keyed reconciliation,
  unkeyed ordinal matching, retained local state/focus/interaction slots,
  deterministic mount/update/unmount/shutdown, stale/foreign target rejection,
  separate semantic identity, capability caches, and reconciliation reports;
- the core-owned `UiApp` contract, ordered initial/update effects, declarative
  application and mounted subscriptions, keyed lifecycle work, typed host
  requests, local/send tasks, monotonic timers, and a deterministic four-budget
  scheduler with live-only generational producer authority, exact saturation,
  checked trace admission, and independent wake/redraw handshakes whose wake
  callbacks are claimed once, serialized, and invoked outside all framework
  synchronization guards;
- one ordered application transaction planner, state-current subscription
  declaration evaluation, direct completion-to-action delivery, explicit send-
  subscription `Starting -> Running` start/sink outcomes, exact ownership
  recovery, and causal scheduler trace lineage;
- deterministic queued application actions, an explicit bounded pump, mounted
  proof activation, focus traversal, scheduler-aware bounded canonical tracing,
  and mounted surface publication;
- typed style values, tokens, computed style, provenance, and diagnostics;
- explicit layout constraints, a renderer-neutral measurement-provider seam,
  and separate one-query intrinsic/child-layout snapshots per publication;
- constrained row/column measurement and arrangement with aligned frame, style, and layout diagnostics;
- mounted-preorder/parent-aligned index, frame, style, and layout products with
  matching mounted/semantic identities, parent and authored metadata, including
  after warmed structural cache changes;
- a proof-level whole-surface cache with topology-only structural snapshots,
  current-mounted style/layout phase input, exact token-content context keys,
  and independently tested actual-execution phase reports;
- a Counter application exercising the current public crates.

Important limitations remain: input behavior is press-based and proof-level,
text measurement is deterministic character counting, and routed events,
pointer identity/capture, release-inside activation, complete routed events,
complete trace v2 normalization, trace export/sinks/replay, production semantics/accessibility,
paint/hit scenes, production layout/style/text, native hosts, renderer backends,
and production controls are absent. The current runtime has one mounted root,
one focus domain, and one current publication domain. Current paint and semantic
facts remain deterministic extension proofs, not the M5/M6 production products.

## Production profiles

RunenUI targets three required profiles:

1. **Headless/test:** deterministic mounted execution, synthetic input and time, deterministic effects/tasks, semantic/layout/hit/paint inspection, and replayable traces without a native window or GPU.
2. **Standalone desktop:** Windows, macOS, and Linux with DPI and multi-window support, clipboard, cursor, IME, drag/drop, accessibility, a production event loop, and one conventional renderer backend.
3. **Embedded host:** a host-owned window and frame loop with host-provided input, resources, timing, clipboard, text, and wakeups, consuming the same renderer-neutral scene protocol without ECS, Runenwerk, or renderer assumptions in RunenUI.

Mobile, web, external UI source formats, docking, visual editing, and advanced devtools are later targets and do not block the first production release.

## Architecture direction

The accepted runtime direction is hybrid:

```text
Application state
    -> transient owned View/Element tree
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> computed style and layout
    -> semantic tree + hit-test scene + paint scene
    -> host accessibility/event integration + renderer backend
```

The transient authored tree is consumed by reconciliation and is not persistent
runtime state. The mounted tree now retains generational and semantic identity,
widget-local state, lifecycle, focus, interaction slots, operational phases,
integrity-aware capability caches, and a proof-level retained publication cache.
Tree changes rebuild every topology-dependent fact from one current mounted
preorder snapshot. Compatible style and layout changes retain topology and read
the current mounted `StyleIntent` and `LayoutStyle`; authored token-reference
changes are scheduled by reconciliation even when token content is unchanged.
No production retained-layout claim is implied.
Application and exact-mounted-generation task/subscription ownership is current;
renderer-neutral paint and hit-test scenes begin in M6.

## Canonical project documents

- [Current status](docs/status-map.md)
- [Feature and support matrix](docs/feature-support-matrix.md)
- [Production roadmap](docs/roadmap.md)
- [Architecture](docs/architecture.md)
- [Documentation retention and disposition](docs/documentation-retention-plan.md)
- [Validation](docs/tooling/validation.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [API stability](docs/api-stability.md)
- [Release policy](docs/release-policy.md)

When sources disagree, the current status, support matrix, roadmap, architecture, and accepted ADRs take precedence over older incremental design documents. Historical material is not implementation authority.

## Current API proof

The Builder API is the semantic foundation; `element!` is optional sugar over the same open view protocol:

```rust
use runenui_core::{Element, View, button, children, column, text};

#[derive(Clone, Copy)]
enum CounterAction {
    Increment,
}

fn counter_screen(value: i32) -> Element<CounterAction> {
    column(children![
        text(format!("Count: {value}")),
        button("+").on_activate(|| CounterAction::Increment),
    ])
    .gap(8_u16)
    .into_element()
}
```

Typed builders reject incompatible configuration at compile time, `children!`
has no fixed arity ceiling, and dynamic numeric/identifier constructors validate
their inputs. Components can author a local action and map their subtree into a
parent action without knowing that parent type:

```rust
use runenui_core::{Element, View, button};

enum ChildAction { Save }
enum ParentAction { Child(ChildAction) }

fn child() -> Element<ChildAction> {
    button("Save").on_activate(|| ChildAction::Save).into_element()
}

fn parent() -> Element<ParentAction> {
    child().map_action(ParentAction::Child)
}
```

Every widget explicitly declares and creates state (`type State = ();` for a
stateless widget). Mounted activation may mutate it, every capability can observe
it, and compatible reconciliation retains it. This does not imply correct
release-based activation, production controls, accessibility, paint scenes, or
native rendering.

## Validation

The repository baseline is:

```powershell
cargo +stable fmt --all
cargo validate
```

Format intentional changes with latest stable rustfmt, matching CI. `cargo validate` is the locked, read-only shared local/CI implementation. It runs stable formatting checks, locked tests, Clippy with denied warnings, Rust 1.93.0 MSRV tests, repository metadata checks, and repository-relative Markdown link validation from the resolved workspace root. Also run `git diff --check` and slice-specific checks. See [Validation](docs/tooling/validation.md).

Generated context exports are written to the ignored `context/` directory:

```powershell
py tools/context/export_repo_context.py
```

Normal profiles exclude historical legacy material. See [Context Export](tools/context/README.md).

## Release status

RunenUI has not reached a stable public API or production release. All workspace packages are `0.1.0` and publication is disabled until release infrastructure and milestone gates exist. `1.0.0` is reserved for completion of the required production profiles and the M11 hardening gate.

RunenUI is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
