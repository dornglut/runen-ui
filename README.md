# RunenUI

> **Category: Current contract**

RunenUI is a pre-1.0 Rust-native UI framework project. Its production goal is a stable, extensible UI kernel for deterministic headless testing, standalone desktop applications on Windows, macOS, and Linux, and embedding in engine- or editor-owned hosts.

Today RunenUI is a coherent **headless architecture proof**. It is not a production UI framework, native desktop toolkit, renderer backend, or finished control library. Current APIs are experimental and may change incompatibly while the foundations are corrected.

Milestone status: M0 through M4 are complete and owner-accepted. M5 is active;
M5A semantic contribution and independent identity plus its mandatory post-merge
reconciliation are complete. The accepted M5A feature head
`8377ced53c08d7b5be3020368ceddd3ee81294a5` was guarded-squash-merged in
[PR #53](https://github.com/dornglut/runen-ui/pull/53) as
`e3c304600ec1777cd17a1973946a43c765df1c31`. Its explicitly accepted
reconciliation head `66c2e2a5e2adf3709f93e8d45821a5844986dc0c` was guarded-squash-merged
in [PR #54](https://github.com/dornglut/runen-ui/pull/54) as
`d7189d9d145b20edc6ad931ead1589f6277373d2`; reviewed and squash trees are
identical, and accepted-main CI #898 passed at that exact squash. Issue #55 is
the current readiness/authority gate before M5B: it freezes publication,
focus/support, surface-scoped semantic action targeting, publication atomicity,
and clean renderer/semantic cutover semantics, plus one bounded pre-1.0 removal
of route-bound semantic LogicalScroll. M5B #48 remains blocked until #55 is
separately accepted and merged. Current maturity, durable sequence, work
ownership, and historical acceptance evidence live in the
[status map](docs/status-map.md), [roadmap](docs/roadmap.md),
[work-tracking contract](docs/work-tracking.md), and
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
- an owner-accepted, platform-neutral semantic contribution contract in which a
  widget contributes an action-type-independent forest of zero or more
  owner-local semantic nodes keyed by stable `SemanticKey` values, with strict
  mounted-child marker and local-reference validation, roles/names/descriptions,
  values/states/action intent/relationships/text facts, and exact owner or
  validated owner-local logical bounds;
- a separate runtime-owned generational semantic arena and owner/key binding
  store issuing opaque `SemanticNodeId` lifetimes independently from mounted
  arena allocation; compatible owner/key retention and contribution reorder
  preserve identity, while key/owner removal revokes the exact lifetime and
  later slot reuse advances generation;
- core-owned canonical `LogicalSize` and `LogicalRect` geometry shared by
  authoring and runtime; semantic contribution has no absolute surface-coordinate
  authority, and recursive action mapping preserves semantic contribution
  content exactly;
- M5 semantic authoring actions limited to platform-neutral `Activate`,
  `RequestFocus`, `OpenMenu`, and `OpenContextMenu`; routed
  `SemanticCommand::LogicalScroll` remains part of accepted M4 command behavior
  rather than semantic-node authoring;
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
- deterministic queued application actions and exact-target semantic commands,
  routed `Activate` default and route-only cancel/menu/context commands, an
  explicit bounded pump, focus traversal, scheduler-aware bounded canonical
  tracing with routed causal parentage, and mounted surface publication;
- one runtime-owned exact-generation focus authority with nested scope policies,
  retained modality, current-publication directional geometry, remembered
  restoration, atomic focus-within transitions, and routed `FocusOut`/`FocusIn`;
- runtime-issued opaque `SurfaceId`/`SurfaceInputContext`, fresh displayed
  coordinate revision and hit-test generation on every publication, configurable
  bounded immutable historical hit-test snapshots, exact checked logical/resolved
  ingress with owned rejection recovery, and causal surface trace lineage;
- core-owned checked pointer/device identities and complete host-neutral
  down/move/up/cancel/wheel payloads; canonical non-reentrant pointer ingress;
  separate physical, routed, pressed, and captured identities; ordered boundary
  and capture notifications; stationary publication re-hit; integrity-only
  unavailable-context cleanup; release-inside activation; route-only logical
  scrolling; and slice-local causal trace;
- deterministic JSONL v1 trace projection plus the accepted M4D3 offline replay
  foundation with replay-only trace/work identities, contiguous-sequence and
  causal-parent validation, explicit dropped-prefix incompleteness, and serialized
  Counter reconstruction without live runtime authority;
- typed style values, tokens, computed style, provenance, and diagnostics;
- explicit layout constraints, a renderer-neutral measurement-provider seam,
  and separate one-query intrinsic/child-layout snapshots per publication;
- constrained row/column measurement and arrangement with aligned frame, style, and layout diagnostics;
- mounted-preorder/parent-aligned index, frame, style, and layout products with
  matching mounted identities, parent and authored metadata, including after
  warmed structural cache changes; semantic identities are deliberately not a
  singular projection of those renderer-facing products and remain runtime-owned
  until M5B publishes the independent semantic product;
- a proof-level whole-surface cache with topology-only structural snapshots,
  current-mounted style/layout phase input, exact token-content context keys,
  and independently tested actual-execution phase reports;
- a Counter application exercising the current public crates.

Important limitations remain: pointer input is a deterministic logical-surface
proof without native host translation or production scrolling; text measurement
is deterministic character counting; M4C5's owner-accepted raw keyboard,
committed-text, composition, and authored-ID automation remain host-neutral
proof behavior without editable text or native translation; M4D1's accepted
in-memory trace schema is normalized and causally reconstructable; M4D2 adds
accepted deterministic JSONL v1 projection, default-redacted/explicit-full
text/IME capture, optional static action labels, and a subordinate lazily bounded
nonblocking trace sink; and M4D3 adds an accepted inert offline causal replay
model over that serialized projection. M5A supplies production semantic
contribution authoring and independent runtime semantic lifetimes, but it does
**not** yet publish the independent semantic tree, translate owner-local bounds
into absolute semantic bounds, derive runtime focus into that product, resolve
cross-owner relationships, expose semantic-node action ingress, provide the
public `runenui_testing` harness, or add AccessKit/native accessibility. #55
freezes those successor contracts but implements none of M5B/M5C runtime
behavior. Those remain M5B–M5D work. Paint/hit scenes, production
layout/style/text, native hosts, renderer backends, and production controls also
remain absent. The current runtime has one mounted root, one focus domain, and
one logical surface with bounded proof-level displayed hit-test history.
`SurfaceNode::semantics()` temporarily carries the canonical M5A contribution
during the M5B cutover; it is not the independent semantic product and carries
no public semantic identity.

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
runtime state. The mounted tree now retains mounted generational identity,
widget-local state, lifecycle, focus, interaction slots, operational phases,
integrity-aware capability caches, a separate semantic arena/binding store, and
a proof-level retained renderer-facing publication cache. Widgets contribute
canonical semantic forests independently of action type; the runtime validates
and reconciles their owner-local keys into independent semantic lifetimes. #55
freezes the successor publication/action contract before implementation. M5B
then owns composition of those accepted contributions into a separately typed,
absolute-bounds/focus-aware, surface-scoped semantic snapshot and update product.
Tree changes rebuild every topology-dependent renderer fact from one current
mounted preorder snapshot. Compatible style and layout changes retain topology
and read the current mounted `StyleIntent` and `LayoutStyle`; authored
token-reference changes are scheduled by reconciliation even when token content
is unchanged. No production retained-layout claim is implied. Application and
exact-mounted-generation task/subscription ownership is current;
renderer-neutral paint and hit-test scenes begin in M6.

## Canonical project documents

- [Architecture entrypoint](ARCHITECTURE.md)
- [Testing and validation entrypoint](TESTING.md)
- [Current status](docs/status-map.md)
- [Feature and support matrix](docs/feature-support-matrix.md)
- [Production roadmap](docs/roadmap.md)
- [Work tracking](docs/work-tracking.md)
- [Detailed architecture](docs/architecture.md)
