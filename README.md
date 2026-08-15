# RunenUI

> **Category: Current contract**

RunenUI is a pre-1.0 Rust-native UI framework project. Its production goal is a stable, extensible UI kernel for deterministic headless testing, standalone desktop applications on Windows, macOS, and Linux, and embedding in engine- or editor-owned hosts.

Today RunenUI is a coherent **headless architecture proof**. It is not a production UI framework, native desktop toolkit, renderer backend, or finished control library. Current APIs are experimental and may change incompatibly while the foundations are corrected.

Milestone status: M0 through M4 are complete and owner-accepted. M5 is active.
M5A semantic contribution and independent identity, the #55 semantic readiness
authority, M5B semantic publication/incremental updates, and the M5C semantic
action ingress/accessibility-resolution feature are owner-accepted. M5C exact
reviewed head `504899b79059eb94ad4474d67bba1e27eb30b374` passed exact-head CI
#1170 / `31889342640` and was guarded-squash-merged in
[PR #62](https://github.com/dornglut/runen-ui/pull/62) as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`. Reviewed head and squash share
exact tree `dfa7cb71166a3f333b560508a7e82fbeb45df000`, and accepted-main push CI
#1171 / `31903354382` passed at that exact squash. The mandatory post-M5C
current-contract reconciliation is the current gate. M5D #50 remains blocked
until that reconciliation is owner-accepted, merged, and accepted-main verified.
Current maturity, durable sequence, work ownership, and historical acceptance
evidence live in the [status map](docs/status-map.md),
[roadmap](docs/roadmap.md), [work-tracking contract](docs/work-tracking.md), and
[public repository migration history](docs/history/public-repository-migration.md).

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
  capability caches, and reconciliation reports;
- a platform-neutral semantic contribution contract in which a widget contributes
  an action-type-independent forest of zero or more owner-local semantic nodes
  keyed by stable `SemanticKey` values, with strict mounted-child marker and
  local-reference validation, roles/names/descriptions, values/states/action
  intent/relationships/text facts, and exact owner or validated owner-local
  logical bounds;
- a separate runtime-owned generational semantic arena and owner/key binding
  store issuing opaque `SemanticNodeId` lifetimes independently from mounted
  arena allocation; compatible owner/key retention and contribution reorder
  preserve identity, while key/owner removal revokes the exact lifetime and
  later slot reuse advances generation;
- an independent renderer-neutral `SemanticPublication` sibling scoped to exact
  `SurfaceId`, with deterministic roots/preorder and exact-ID lookup, absolute
  logical bounds, resolved local/cross-owner relationships, composed state and
  supported actions, runtime-derived visible-PRIMARY focus, typed semantic
  diagnostics, and no public `MountedNodeId` routing shortcut;
- checked semantic revisions and deterministic incremental updates: first
  committed snapshot revision 1, unchanged adapter-visible products retain the
  revision, changed products advance without wrapping, added/changed/removed/
  root/focus deltas are deterministic, and wrong-surface or wrong/skipped prior
  revision requires a full resynchronization;
- public exact semantic action ingress through `SemanticActionRequest` values
  constructed with `SemanticActionRequest::new(surface, target, action)` and
  submitted by `AppRuntime::submit_semantic_action`, with M5 actions limited to
  `Activate`, `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; admission checks
  the exact current semantic product, support/state/readiness/freshness and
  canonical queue/work/trace capacity without invoking widget callbacks, then
  joins the existing command FIFO/routed/default/action/update path;
- exact semantic queue-front and post-callback revalidation without synchronous
  refresh or retargeting; accepted-then-stale work becomes a canonical processing
  rejection under its accepted `WorkSequence`, while explicit `prevent_default`
  and callback-caused semantic invalidation remain trace-distinct;
- core-owned canonical `LogicalSize` and `LogicalRect` geometry shared by
  authoring and runtime; semantic contribution has no absolute surface-coordinate
  authority, while M5B runtime composition derives absolute semantic bounds;
- M5 semantic action vocabulary limited to platform-neutral `Activate`,
  `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; routed
  `SemanticCommand::LogicalScroll` remains accepted M4 command behavior rather
  than semantic-node authoring or action ingress;
- core-owned opaque mounted/time/work-sequence protocol values plus a narrow
  semantic-command event vocabulary, checked downstream event capability,
  immutable capture/target/bubble routing, independent propagation/default
  control, exact target/capacity rejection without sequence consumption,
  structured routed-integrity diagnosis, and mapped non-`Clone` output;
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
- deterministic queued application actions, exact-target mounted semantic
  commands, and exact-target semantic actions after admission; routed `Activate`
  default and route-only cancel/menu/context commands; an explicit bounded pump;
  focus traversal; scheduler-aware bounded canonical tracing with routed and M5C
  semantic causal parentage; and mounted surface publication;
- one runtime-owned exact-generation focus authority with nested scope policies,
  retained modality, current-publication directional geometry, remembered
  restoration, atomic focus-within transitions, routed `FocusOut`/`FocusIn`, M5B
  projection of final runtime focus into semantic publication without rerunning
  unchanged semantic contribution, and M5C semantic `RequestFocus` convergence
  through the same accepted focus-default authority;
- runtime-issued opaque `SurfaceId`/`SurfaceInputContext`, fresh displayed
  coordinate revision and hit-test generation on every publication, configurable
  bounded immutable historical hit-test snapshots, exact checked logical/resolved
  ingress with owned rejection recovery, and causal surface trace lineage;
- a fallible staged surface-publication transaction
  `admit -> plan -> candidate-dependent final preflight -> commit`: knowable
  counter/queue/trace failures are preflighted, required stationary re-hit queue
  fullness is recoverable with zero partial publication commit and redraw still
  pending, and redraw/hit-test/coordinate/semantic revision exhaustion retains
  exact terminal classification without wrap or saturation;
- mandatory renderer products plus independent semantic publication and semantic
  diagnostics in `SurfacePublication`; complete-product versus renderer-only
  equality/extraction is explicit, while renderer-facing `SurfaceFrame`,
  `SurfaceNode`, and debug paths no longer carry production semantic authority;
- core-owned checked pointer/device identities and complete host-neutral
  down/move/up/cancel/wheel payloads; canonical non-reentrant pointer ingress;
  separate physical, routed, pressed, and captured identities; ordered boundary
  and capture notifications; stationary publication re-hit; integrity-only
  unavailable-context cleanup; release-inside activation; route-only logical
  scrolling; and slice-local causal trace;
- deterministic JSONL v1 trace projection plus the accepted M4D3 offline replay
  foundation with replay-only trace/work identities, contiguous-sequence and
  causal-parent validation, explicit dropped-prefix incompleteness, serialized
  Counter reconstruction without live runtime authority, and M5C semantic trace
  records remaining inert observations on replay;
- typed style values, tokens, computed style, provenance, and diagnostics;
- explicit layout constraints, a renderer-neutral measurement-provider seam,
  and separate one-query intrinsic/child-layout snapshots per publication;
- constrained row/column measurement and arrangement with aligned frame, style,
  and layout diagnostics;
- mounted-preorder/parent-aligned index, frame, style, and layout products with
  matching mounted identities, parent and authored metadata, including after
  warmed structural cache changes; semantic identities remain a separate public
  semantic product rather than a renderer-product projection;
- a proof-level retained surface cache with topology/style/layout/hit/paint
  phase facts and independently tested execution reports; #59 owns removal of
  whole-cache deep cloning before or during M6 without weakening M5B atomicity;
- a Counter application exercising the current public crates, plus genuine
  downstream direct and adapter-shaped semantic publication consumers and public
  M5C semantic-action/readiness conformance.

Important limitations remain: pointer input is a deterministic logical-surface
proof without native host translation or production scrolling; text measurement
is deterministic character counting; keyboard, committed-text, composition, and
authored-ID automation remain host-neutral proof behavior without editable text
or native translation; tracing/replay remains headless observational
infrastructure. M5B supplies independent semantic publication, absolute semantic
bounds, resolved relationships, runtime focus projection, composed state/support,
typed diagnostics, and deterministic revisions/updates. M5C now supplies public
exact semantic-node action ingress/resolution for `Activate`, `RequestFocus`,
`OpenMenu`, and `OpenContextMenu` through the canonical queue/routed/default/trace
architecture. It does **not** provide semantic LogicalScroll, the M5D
`runenui_testing` harness, AccessKit/native accessibility, or multi-surface
lifecycle. Those remain M5D/M7/M10 work. Paint/hit scenes, production
layout/style/text, native hosts, renderer backends, and production controls also
remain absent. The current runtime has one mounted root, one focus domain, and
one logical surface with bounded proof-level displayed hit-test history.

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
runtime state. The mounted tree retains mounted generational identity,
widget-local state, lifecycle, focus, interaction slots, operational phases,
integrity-aware capability caches, a separate semantic arena/binding store, and
a proof-level retained renderer-facing publication cache. Widgets contribute
canonical semantic forests independently of action type; the runtime validates
and reconciles their owner-local keys into independent semantic lifetimes. M5B
composes those contributions into the separately typed surface-scoped semantic
snapshot/update/diagnostic product. M5C resolves exact current semantic action
requests through private owner/key bindings and converges accepted work on the
existing command FIFO/routed/default/action/update/trace path without exposing
mounted identity or creating a second semantic dispatcher. Tree changes rebuild
topology-dependent renderer facts from one current mounted preorder snapshot.
Compatible style and layout changes retain topology and read current mounted
style/layout state; layout movement refreshes semantic bounds without rerunning
unchanged semantic contribution. M5D next owns the unified public deterministic
headless testing harness. M6 owns renderer-neutral paint/hit scene products only
after M5 semantic/testing closure.

## Canonical project documents

- [Architecture entrypoint](ARCHITECTURE.md)
- [Testing and validation entrypoint](TESTING.md)
- [Current status](docs/status-map.md)
- [Feature and support matrix](docs/feature-support-matrix.md)
- [Production roadmap](docs/roadmap.md)
- [Work tracking](docs/work-tracking.md)
- [Detailed architecture](docs/architecture.md)
- [Public repository migration history](docs/history/public-repository-migration.md)
- [Documentation retention and disposition](docs/documentation-retention-plan.md)
- [Validation details](docs/tooling/validation.md)
- [API stability](docs/api-stability.md)
- [Release policy](docs/release-policy.md)

When sources disagree, accepted ADR behavior, the active execution charter, the conformance matrix, stable architecture contracts, current implementation/tests, and current status records take precedence over pull-request descriptions and historical material. See [Work tracking](docs/work-tracking.md) for the full authority split and pickup sequence.

## Current API proof

The Builder API remains ordinary typed Rust; `element!` is optional sugar over
the same open view protocol:

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
it, and compatible reconciliation retains it. Widgets may author canonical
semantic contribution through the same state-aware contract. The runtime owns
semantic identity and M5B publication. M5C exposes exact semantic action ingress
through `AppRuntime::submit_semantic_action` while keeping the semantic-to-mounted
binding private. Widget authoring still does not imply a native accessibility
adapter, the M5D public harness, production controls, paint scenes, or native
rendering.

## Validation

The repository baseline is:

```powershell
cargo +stable fmt --all
cargo validate
```

Format intentional changes with latest stable rustfmt, matching CI. `cargo validate` is the locked, read-only shared local/CI implementation. It runs stable formatting checks, locked tests, Clippy with denied warnings, Rust 1.93.0 MSRV tests, repository metadata checks, and repository-relative Markdown link validation from the resolved workspace root. Also run `git diff --check` and slice-specific checks. See [Testing and validation](TESTING.md).

Generated context exports are written to the ignored `context/` directory:

```powershell
py tools/context/export_repo_context.py
```

Normal profiles exclude historical legacy material. See [Context Export](tools/context/README.md).

## Release status

RunenUI has not reached a stable public API or production release. All workspace packages are `0.1.0` and publication is disabled until release infrastructure and milestone gates exist. `1.0.0` is reserved for completion of the required production profiles and the M11 hardening gate.

RunenUI is licensed under the [MIT License](LICENSE).
