# RunenUI

> **Category: Current contract**

RunenUI is a pre-1.0 Rust-native UI framework project. Its production goal is a stable, extensible UI kernel for deterministic headless testing, standalone desktop applications on Windows, macOS, and Linux, and embedding in engine- or editor-owned hosts.

Today RunenUI is a coherent **headless architecture proof**. It is not a production UI framework, native desktop toolkit, renderer backend, or finished control library. Current APIs are experimental and may change incompatibly while the foundations are corrected.

Milestone status: M0 through M5 are complete and owner-accepted. M6A0
architecture/conformance authority and its bounded current-contract reconciliation
are also accepted, but no M6 scene behavior is implemented. Accepted ADR 0007
and the 36-row M6 matrix freeze the renderer-neutral paint/hit scene target; all
36 M6 behavior rows remain `blocked`. The first M6A implementation slice is
[issue #59](https://github.com/dornglut/runen-ui/issues/59), which owns only the
persistent retained-publication substrate required by `SCENE-PUB-01..05` and
must preserve the accepted staged publication transaction and semantic-product
separation. It does not authorize M6B scene APIs, a renderer backend, or later
M6 behavior. Current maturity, durable sequence, work ownership, and historical
acceptance evidence live in the [status map](docs/status-map.md),
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
- a public downstream `runenui_testing` crate whose `TestHarness<App>` composes
  one ordinary `AppRuntime<App>` with deterministic `ManualClock` authority,
  non-zero configurable fixed-surface publication, explicit bounded pumping and
  finite settle outcomes, snapshot-scoped semantic queries/targets, ordinary
  public interaction ingress, and read-only state/focus/reconciliation/frame/
  layout/hit/paint/semantic/trace/replay inspection without private runtime
  seams or a parallel expected-state model;
- exact testing semantic targets retain `SurfaceId + SemanticNodeId` scope and
  semantic actions delegate to accepted M5C ingress; ambiguous queries do not
  choose first/last, the testing layer exposes no semantic-to-`MountedNodeId`
  shortcut or bare-ID surface reconstruction, and settling never hides a wall-
  clock wait or unbounded execution loop;
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
  phase facts and independently tested execution reports; #59 owns the first M6A
  persistent retained-publication implementation after the M6A0 reconciliation,
  without weakening M5B atomicity;
- a Counter application plus genuine downstream widgets exercising the current
  public crates through semantic publication/action and the M5D testing harness,
  including pointer, keyboard/text/composition, controller focus/restoration,
  deterministic scheduler/time, redaction, export, and replay conformance.

Important limitations remain: pointer input is a deterministic logical-surface
proof without native host translation or production scrolling; text measurement
is deterministic character counting; keyboard, committed-text, composition, and
authored-ID automation remain host-neutral proof behavior without editable text
or native translation; tracing/replay remains headless observational
infrastructure. M5B supplies independent semantic publication, absolute semantic
bounds, resolved relationships, runtime focus projection, composed state/support,
typed diagnostics, and deterministic revisions/updates. M5C supplies public
exact semantic-node action ingress/resolution for `Activate`, `RequestFocus`,
`OpenMenu`, and `OpenContextMenu` through the canonical queue/routed/default/trace
architecture. M5D supplies the public deterministic `runenui_testing` harness on
top of those existing public contracts. Accepted M5 still does **not** provide
semantic LogicalScroll, AccessKit/native accessibility, or multi-surface
lifecycle; those remain M7/M10 work. Paint/hit scenes, production
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
mounted identity or creating a second semantic dispatcher. M5D adds a separate
downstream testing crate that composes the same public runtime, publication,
input, clock, semantic, trace, and replay APIs without acquiring live runtime
authority. Tree changes rebuild topology-dependent renderer facts from one
current mounted preorder snapshot. Compatible style and layout changes retain
topology and read current mounted style/layout state; layout movement refreshes
semantic bounds without rerunning unchanged semantic contribution. M5E integrated
conformance and migration closure is accepted. M6 now owns the next architecture
boundary: renderer-neutral paint/hit scene products. ADR 0007 and the M6
conformance matrix are accepted target authority, the bounded M6A0
current-contract reconciliation is complete, and #59 is the first M6A
implementation slice for the persistent retained-publication substrate. No M6
scene behavior is accepted until its owning conformance rows complete their
normal implementation/proof/owner-acceptance lifecycle.

## Canonical project documents

- [Architecture entrypoint](ARCHITECTURE.md)
- [Testing and validation entrypoint](TESTING.md)
- [Current status](docs/status-map.md)
- [Feature and support matrix](docs/feature-support-matrix.md)
- [Production roadmap](docs/roadmap.md)
- [Work tracking](docs/work-tracking.md)
- [Detailed architecture](docs/architecture.md)
- [M5 semantics and testing charter](docs/architecture/m5-semantics-and-testing-charter.md)
- [M5 conformance matrix](docs/architecture/m5-conformance-matrix.md)
- [ADR 0007 renderer-neutral paint/hit scene protocol](docs/adr/0007-renderer-neutral-paint-hit-scene-protocol.md)
- [M6 conformance matrix](docs/architecture/m6-conformance-matrix.md)
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
binding private. M5D exposes public deterministic testing convenience through
`runenui_testing` without moving runtime authority into that crate. Widget
authoring still does not imply a native accessibility adapter, production
controls, paint scenes, or native rendering.

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
