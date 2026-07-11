# RunenUI Deep Audit and Remediation Backlog

**Repository:** `Crystonix/RunenUI`  
**Audited baseline:** merged `master` after PR #64 (`b27289349880be3504bcbaec714acb662d591c58`)  
**Compromised work excluded from baseline:** open PR #65  
**Audit basis:** current GitHub `master`, PRs #1–#65, current architecture documents, runtime/core source, tests, examples, CI and tooling.

---

## 1. Executive verdict

RunenUI has a coherent clean-start foundation:

- host-neutral typed element trees;
- typed application state/actions/update;
- runtime node indexing, activation, focus and input proofs;
- unified surface publication;
- typed style values, tokens, computed style, provenance and diagnostics;
- computed padding geometry;
- normalized layout constraints;
- a renderer-neutral text measurement provider contract.

The merged code through PR #64 is not fundamentally broken. Most narrow PR scopes were deliberate sequencing, not accidental shortcuts.

However, four issues now require correction before broad feature development:

1. **Repository/process state is unreliable.** PR #65 is not a source implementation; it is temporary workflow/script machinery. Several merged branches also appear to remain on the remote despite earlier cleanup claims.
2. **Authority documents are stale and contradictory.** `docs/status-map.md` and `docs/crate-map.md` still describe implemented crates as skeletons. `docs/target-api.md` shows APIs that do not exist without a strong actual-vs-target distinction.
3. **The layout/measurement track is half-integrated.** Public constraints and measurement contracts exist, but surface layout still uses the old fixed-size and `SurfaceLayoutMetrics` path.
4. **Foundational runtime concerns were intentionally deferred and must be resolved before stateful controls, accessibility, editors, docking or live reload:** stable mounted identity/reconciliation, runtime-local control state, event dispatch semantics, effects, semantics/accessibility, and a real render protocol.

The recommended strategy is not a full rewrite. Stabilize the repository, complete the layout contract, then address the foundational runtime seams in dependency order.

---

# 2. Immediate process and repository remediation

## P0.1 Close and discard PR #65

### Finding

PR #65 currently changes only:

- `.github/scripts/apply_surface_measurement.py`
- `.github/surface-measurement-cutover.trigger`
- `.github/workflows/ci.yml`
- `crates/runenui_runtime/tests/surface_frame_hit_test.rs`

It contains approximately 700 added lines but does not contain the intended runtime cutover.

### Why this matters

CI was used as a remote patch executor. This caused brittle exact-text transformations, repeated trigger commits, truncated diagnostics and workflow scheduling failures. The PR is not meaningfully reviewable.

### Required action

- close PR #65 without merge;
- delete `impl/surface-measurement-cutover`;
- do not salvage its commit history;
- recreate the implementation from current `master` in a normal working tree;
- keep `.github/workflows/ci.yml` unchanged in implementation PRs.

### Acceptance

The replacement PR contains only intended Rust, test, example and documentation files.

---

## P0.2 Establish one authoritative working checkout

### Finding

The available local RunenUI checkouts are stale around PR #57–#59 and are not connected to the current remote. They cannot safely serve as the source for continued implementation.

### Required action

- create a fresh checkout from current `master`;
- confirm `git remote -v`, branch and head SHA;
- remove or clearly archive stale reconstructed checkouts;
- regenerate the repository context export from that checkout;
- record the current baseline SHA in every Codex task.

### Acceptance

A task begins with:

```text
git status --short
git branch --show-current
git log -1 --oneline
git remote -v
```

and the working tree corresponds to current GitHub `master`.

---

## P0.3 Stop using CI to construct commits

### Required policy

CI may:

- format-check;
- lint;
- compile;
- test;
- produce diagnostics and artifacts.

CI must not:

- rewrite repository source;
- generate implementation commits;
- push feature branches;
- temporarily replace the standard workflow;
- serve as a substitute for a local working tree.

Add this rule to `AGENTS.md` or a dedicated contributor workflow document.

---

## P0.4 Clean merged branches and repository merge policy

### Finding

Workflow checkout logs show multiple merged feature branches still present remotely, including layout, computed-padding and documentation branches.

### Required action

- list all remote branches;
- delete branches whose PRs are merged and whose heads are reachable from `master`;
- retain only active or explicitly archival branches;
- enable GitHub’s automatic head-branch deletion after merge, if supported;
- use squash merge as the project default;
- consider disabling merge commits if the intended history policy is squash-only.

### Acceptance

`master` plus current active branches only.

---

## P0.5 Correct package/release metadata

### Finding

The workspace currently declares:

```toml
version = "1.0.0"
description = "A small rust-native UI lib"
license = ""
```

This does not match an early, private, unstable framework.

### Required decision

Choose one:

1. **Pre-release library direction**
   - set workspace version to `0.1.0`;
   - choose and add a real license;
   - use an accurate description;
   - add per-crate descriptions/readmes where useful.

2. **Private experimental workspace**
   - set `publish = false` on all packages;
   - still use a non-misleading internal version and description;
   - document that SemVer stability is not promised.

Do not publish crates with an empty license and accidental `1.0.0` stability signaling.

---

# 3. Documentation authority and roadmap repair

## P0.6 Replace stale status documents

### Finding

`docs/status-map.md` still says:

- `runenui_core` is a skeleton and needs a real element tree;
- `runenui_runtime` is a skeleton and needs dispatch/trace/surface frames;
- the Counter is a skeleton and still needs implementation.

All of those are already implemented.

`docs/crate-map.md` repeats the same stale skeleton status.

### Required action

Make `docs/roadmap.md` the current execution authority and revise:

- `docs/status-map.md`
- `docs/crate-map.md`
- `docs/cutover-plan.md`
- `docs/legacy-audit.md`
- crate READMEs
- example README

Use explicit statuses such as:

```text
implemented proof
active integration
planned contract
deferred
stable
```

Avoid documents that preserve historical “next step” statements indefinitely.

---

## P0.7 Separate actual API from target API

### Finding

`docs/target-api.md` shows:

```rust
use runenui::prelude::*;

Runtime::builder()
    .surface(...)
    .state(...)
    .update(...)
    .root(...)
    .run();
```

There is no `runenui` facade crate and no such runtime builder. The implemented path is based on `runenui_core`, `runenui_runtime`, `UiApp` and `AppRuntime`.

### Required action

Split documentation into:

- **Current public API**
- **Accepted target direction**
- **Unresolved design sketches**

Every non-implemented example must be visibly labeled as target-only.

Update README examples to compile against the actual workspace or label them as target syntax.

---

## P0.8 Create one remediation tracker

Add `docs/remediation-backlog.md` or replace the current roadmap with a structured tracker containing:

- ID;
- priority;
- status;
- dependency;
- affected files;
- acceptance criteria;
- source design document;
- PR number when implemented.

This audit can be used as the initial content.

---

# 4. Complete the active layout and measurement track

## P0.9 Authoritative surface measurement cutover

### Current state

Merged:

- `LayoutConstraints`;
- `MeasurementProvider`;
- `TextMeasurementRequest`;
- `TextMeasurement`;
- deterministic fallback provider.

Still active in surface publication:

- separate fixed `LogicalSize` input;
- `SurfaceLayoutMetrics`;
- character-count measurement in `surface.rs`;
- duplicate text/button metrics.

### Required design

`SurfaceBuildContext` becomes the complete borrowed publication input:

```text
StyleTokens
LayoutConstraints
MeasurementProvider
temporary explicit control-layout policy, if still needed
```

Use exactly one internal path.

### Required changes

- integrate root `LayoutConstraints`;
- integrate borrowed `MeasurementProvider`;
- remove `SurfaceLayoutMetrics`;
- remove character-count logic from `surface.rs`;
- use the provider for both standalone text and button labels;
- normalize/constrain every custom-provider result before geometry use;
- derive frame size from root constraints;
- migrate `AppRuntime`, free publication function, tests and Counter;
- update docs and status.

### Design caution

Do not create a broad public control-metrics API merely to preserve the old test hook. Button minimum outer size is temporary control policy. Keep it private or narrowly modeled until recipes/standard controls create real pressure.

### Non-goals

- no wrapping;
- no font selection/shaping;
- no child constraint propagation;
- no overflow diagnostics;
- no new crate;
- no renderer.

---

## P1.1 Introduce a single measured layout result

### Finding

The current layout recursively measures nodes and later recursively pushes nodes, causing repeated subtree measurement. Nested containers can be measured multiple times during one publication.

### Required refactor

Use a publication-local layout result indexed by `RuntimeNodeId`:

```text
ResolvedSurfaceTree
  -> measure each node once
  -> LayoutNodeResult / measured size table
  -> arrange bounds
  -> SurfaceFrame
```

A simple vector aligned with runtime node IDs is sufficient.

### Acceptance

- each text node invokes the measurement provider once per publication;
- each node has one measured desired size per publication;
- arrangement does not recursively remeasure descendants;
- tests use a counting provider to prove call counts.

Do this with constrained row/column work, not as a separate abstraction-only crate.

---

## P1.2 Propagate content-box constraints

Implement the accepted box-model order:

```text
outer constraints
  -> subtract padding
  -> content constraints
  -> measure content
  -> constrain content
  -> add padding
  -> constrain outer size
```

Required behavior:

- tight root;
- loose root shrink-to-fit;
- unbounded root intrinsic sizing;
- horizontal and vertical child constraint propagation;
- content constraints collapse to zero when padding exceeds available size;
- deterministic overflow flags/facts;
- no implicit negative geometry.

---

## P1.3 Explicit overflow diagnostics

Add a small runtime-aligned diagnostic product:

```text
RuntimeNodeId
available content size
desired content size
final outer size
overflowed width
overflowed height
```

Do not implement clipping or scrolling in the same PR.

Integrate this into the existing unified publication product rather than creating an independent traversal.

---

## P1.4 Re-run the layout crate boundary review

Only after P0.9–P1.3.

Extraction to `runenui_layout` is justified only if the implemented contracts form an independently useful subsystem with:

- constraints;
- measurement requests;
- measured/arranged results;
- multiple algorithms or meaningful algorithm policy;
- conformance tests;
- a real independent consumer or dependency boundary.

Do not extract merely because the runtime module is growing.

---

# 5. Core value and naming debt

## P1.5 Define numeric invariants for geometry/style values

### Finding

`Length::px(f32)` and `Px::new(f32)` currently accept:

- negative values;
- `NaN`;
- positive/negative infinity.

Padding, radius and gap can therefore inject invalid geometry despite normalized layout constraints.

### Required decision

Define invariants per value:

- padding: finite and non-negative;
- radius: finite and non-negative;
- gap: finite and non-negative;
- positions/transforms may eventually allow negative finite values;
- sizes must be finite and non-negative unless represented explicitly as unbounded constraints.

Choose either:

- normalized constructors with diagnostics at resolution/publication; or
- checked constructors returning an error;
- separate raw authored value and normalized computed value.

Do not silently allow `NaN` into hit testing or frame bounds.

### Tests

Property or table tests for negative, `NaN`, infinity and extreme values.

---

## P1.6 Unify `Px`, `Length`, layout intent and style naming

### Finding

The core has two equivalent logical-length wrappers:

- `Px` for layout gap;
- `Length` for visual style.

There are conversion implementations between them.

The element API also uses confusing names:

- `style()` returns `LayoutStyle`;
- `visual_style()` returns `StyleIntent`;
- `with_visual_style()` mutates authored style.

### Required redesign

Prefer one logical length type.

Recommended vocabulary:

```text
Element::layout() -> LayoutIntent
Element::style() -> StyleIntent
LogicalLength or Length
```

Then decide whether `gap` is:

- authored layout intent resolved directly; or
- a style-resolved/token-backed layout value.

Do not keep duplicate unit types solely because they were added in separate PR tracks.

---

## P1.7 Remove or justify unused public style vocabulary

### Finding

`LengthToken` and `LengthValue` are exported but no real element field uses them. They exist only in type tests and documentation.

This conflicts with the project’s stated demand-driven style policy.

### Required action

Choose one:

- use `LengthValue` for a real field such as token-backed gap or control sizing; or
- remove/de-publicize it until a real field needs it.

Apply the same audit to every prelude export.

---

## P1.8 Validate identifiers and key uniqueness

### Finding

`ElementId`, `ElementKey` and `TokenId` accept empty strings. Runtime lookup returns the first duplicate authored ID. Keys are exposed but not validated or used for mounted identity.

### Required work

- define empty/invalid identifier policy;
- detect duplicate authored IDs where uniqueness is required;
- detect duplicate sibling keys;
- produce deterministic diagnostics;
- never silently bind automation/focus/state to the first duplicate.

Do not make all IDs globally mandatory. Preserve optional authoring.

---

# 6. Mounted identity, reconciliation and runtime-local state

## P1.9 Design mounted runtime identity before stateful controls

### Finding

`RuntimeNodeId` is a pre-order index valid only for one built tree. `ElementKey` claims future identity preservation, but the runtime does not use it. Every action rebuilds the full tree and clears focus.

This is acceptable for the Counter but incompatible with:

- preserving focus;
- text editing;
- scroll position;
- hover/pressed state;
- pointer capture;
- animation;
- effect ownership;
- list reordering;
- component-local runtime state;
- hot reload.

### Required design document

Define:

```text
Authored Element tree
  -> reconciliation/mount
  -> persistent MountedNodeId
  -> current authored/runtime properties
  -> layout/semantics/render products
```

Specify:

- keyed and unkeyed child matching;
- type mismatch replacement;
- duplicate-key diagnostics;
- mounted-node lifecycle;
- focus preservation/removal;
- runtime-local state storage;
- trace identity;
- invalidation;
- full rebuild fallback.

### Important

Do not implement a virtual-DOM clone by habit. The design should fit RunenUI’s explicit element model and host-neutral runtime.

---

## P1.10 Preserve focus across rebuilds

After mounted identity exists:

- retain focus when the same logical node survives;
- move or clear focus deterministically when it disappears/becomes disabled;
- record focus transition reasons;
- keep focus scopes and traversal policy separate from tree index order.

The current unconditional `focus.clear()` after every dispatch must be retired.

---

## P1.11 Add runtime-local control state

Application state should not be forced to own every transient control detail.

The runtime needs a place for:

- pressed/hover state;
- pointer capture;
- text cursor/selection/composition;
- scroll offsets;
- disclosure/open state when control-owned;
- animation or transition state;
- measurement/resource status.

Specify what belongs to application state versus mounted control state.

---

# 7. Input, events and interaction policy

## P1.12 Consolidate the duplicate input pathways

### Finding

The runtime currently exposes overlapping seams:

- `InputIntentResolver`;
- `InputIntentHandler`;
- direct focus policy;
- direct activation policy;
- combined `handle_input_event`;
- pointer-target helper functions.

The intent traits live inside `prelude.rs`, which should normally re-export API rather than define behavior.

### Required refactor

Create one explicit event pipeline:

```text
host event
  -> normalize
  -> hit test / focus target resolution
  -> event dispatch
  -> control interaction policy
  -> application action
  -> update
```

Keep lower-level test seams only where they represent real boundaries.

Move traits/types out of the prelude module or remove them.

---

## P1.13 Define real button activation semantics

The current button activates on primary pointer **press**. Mature controls normally need:

- press begins interaction;
- pointer capture;
- release inside activates;
- release outside cancels;
- cancellation;
- disabled-state transition;
- keyboard press/release policy;
- pressed visual/runtime state.

Decide whether `on_press` literally means press or whether the public event should be `on_activate`.

For accessibility and cross-device consistency, a semantic activation event is preferable to binding app actions directly to one physical input phase.

---

## P1.14 Expand pointer identity and capture contract

Before drag, sliders, scrollbars, docking or touch:

- pointer/device ID;
- pointer type;
- capture ownership;
- cancellation;
- wheel/scroll input;
- coordinates and transforms;
- multi-pointer behavior;
- click count/pressure only when real consumers require them.

Avoid adding all fields speculatively; design the required extensible event envelope first.

---

## P1.15 Separate keyboard input from text input/IME

`Key::Character(char)` is insufficient for text editing.

Design separate streams for:

- physical/logical key events;
- text commit;
- IME composition;
- composition selection/range;
- cancellation;
- clipboard commands.

Do this before implementing a text input control.

---

## P1.16 Add event propagation only when semantics require it

Nested controls, overlays and editor tools will need some combination of:

- target;
- ancestor path;
- capture/bubble;
- handled/stop propagation;
- default behavior prevention.

Do not introduce a browser-scale event model prematurely, but do not hard-code all future interaction as direct target activation.

---

# 8. Trace, diagnostics and testing

## P1.17 Replace duplicate trace storage

### Finding

`Trace` stores both:

```text
Vec<RuntimeEvent>
Vec<TraceRecord>
```

Every event is written twice.

### Required refactor

Store one canonical record sequence.

If coarse event access must remain temporarily, expose:

- an iterator;
- a projection;
- a helper that collects events for tests.

Because the API is not stable, prefer removing the duplicate compatibility store now.

---

## P1.18 Implement the requested runtime trace product

The earlier target included a bounded trace ring and JSONL export. Current trace has four coarse events and grows without bound.

Target events should eventually include:

- mounted/unmounted;
- input observed/targeted;
- focus changed;
- activation dispatched;
- action dispatched;
- state updated;
- root reconciled/rebuilt;
- style resolved/diagnostic;
- measurement requested/completed/fallback;
- layout completed/overflow;
- frame published;
- effect requested/started/completed/cancelled.

Required capabilities:

- monotonic sequence number;
- optional mounted/runtime/authored identity;
- configurable bounded retention;
- sink/subscriber seam;
- deterministic JSONL serialization;
- test assertions without real time dependence.

Do not add wall-clock timestamps as required deterministic data.

---

## P1.19 Introduce `runenui_testing` only after the next contracts stabilize

Current runtime tests are extensive but duplicate helpers and are tied to low-level frame details.

The testing boundary becomes useful when it can provide:

- headless app harness;
- synthetic input;
- semantic queries;
- focus/action assertions;
- layout/frame assertions;
- diagnostic snapshots;
- deterministic provider/clock;
- trace assertions.

Do not create an empty crate. First extract reusable test utilities into an internal test module, then promote when multiple consumers exist.

---

## P1.20 Add conformance tests, not only feature tests

Add contract-driven suites for:

- constraint normalization;
- measure/arrange invariants;
- identity/reconciliation;
- focus survival/removal;
- duplicate IDs/keys;
- provider result sanitization;
- deterministic publication;
- accessibility semantics;
- render protocol ordering/clips.

---

# 9. Authoring API and extensibility

## P1.21 Review `element!` before adding more element kinds

### Finding

`element_macros.rs` is already roughly 460 lines for text, button, row and column. It duplicates attribute handling across brace and call syntaxes.

Scaling this pattern to dozens of controls will create a brittle macro grammar and poor diagnostics.

### Required decision

Keep:

- builder API as semantic foundation;
- `element!` as optional declarative sugar.

Before expansion, add a clean component-expression escape hatch, for example allowing an arbitrary Rust expression as a child.

Reduce duplicate syntax forms unless both have proven users.

Do not move to a procedural macro solely to solve internal duplication. First define the stable semantic authoring model and extension mechanism.

---

## P1.22 Clarify component and custom-element extensibility

Function components already work by returning `Element<Action>`, but the framework cannot yet add a genuinely new primitive/control kind without editing the closed `ElementKind` enum and every runtime match.

Define two extension levels:

1. **Composite component**
   - regular Rust function/type producing existing elements;
   - no runtime extension required.

2. **Primitive/control extension**
   - explicit contract for semantics, intrinsic measurement, interaction behavior and primitive extraction;
   - capability-based and host-neutral;
   - no renderer-owned widget behavior.

Do not introduce a fully dynamic widget trait until a real second-party custom primitive proves the requirements.

---

## P1.23 Audit component argument descriptors

`TextArgs`, `ButtonArgs` and `ContainerArgs` support macros and explicit construction, but they duplicate many builder methods.

Decide whether they are:

- intended public descriptors;
- macro implementation detail;
- future serialized/document boundary.

If public, document their distinct use case. If not, reduce exports.

---

## P1.24 Reduce prelude and flat-root API exposure

The runtime prelude currently exports nearly every low-level type, including temporary layout metrics, constraint internals, measurement internals, debug types and policy result enums.

Recommended layering:

- application authoring prelude;
- advanced runtime/layout modules;
- testing/debug imports kept explicit.

A prelude should not define `InputIntentResolver` and `InputIntentHandler`; it should only re-export selected stable interfaces.

Because the framework is pre-stable, make this breaking cleanup before a facade crate is added.

---

# 10. Effects and application/runtime contract

## P1.25 Turn the effects sketch into an accepted contract

`docs/target-api.md` contains a substantial effects direction but no implemented or independently accepted contract.

Design:

```text
update(State, Action)
update(State, Action, &mut Effects<Action>)
```

or one adapter-based equivalent.

Specify:

- typed task result mapping;
- host commands;
- subscriptions;
- cancellation/identity;
- ordering;
- lifecycle ownership;
- shutdown;
- tracing;
- deterministic testing;
- single-threaded and engine-host execution;
- Send/Sync requirements, if any.

Keep the simple update form.

Do not couple effects to rendering.

---

## P1.26 Review `Action: Clone`

Interactive activation currently clones actions stored in the element tree.

This is acceptable for small enum actions, but should be an explicit design choice.

Before broad API stability, compare:

- cloneable value actions;
- action factories/closures;
- command IDs with payloads;
- consuming mounted handlers.

Do not redesign unless a real use case is blocked; document the constraint now.

---

# 11. Semantics and accessibility

## P1.27 Add a semantic node model before broad controls

Current focusability is inferred directly from `ElementKind` and enabled state.

Introduce host-neutral semantics:

- role;
- label/name;
- description;
- enabled/disabled;
- focusable/focused;
- value/range/check state;
- actions;
- parent/child relationship;
- hidden/inert state.

Use the same mounted identity as layout and input.

---

## P1.28 Produce an accessibility tree from semantics

The host maps this tree to AccessKit/platform APIs.

Accessibility must not be implemented inside a concrete renderer.

Required initial proof:

- text;
- button;
- disabled button;
- focus;
- activation action;
- deterministic tree inspection.

Do this before adding many controls so accessibility is part of each control contract.

---

# 12. Surface, render protocol and hit testing

## P2.1 Separate semantic surface data from render primitives

`SurfaceFrame` currently combines:

- runtime identity;
- semantic node kind;
- bounds;
- computed style.

It is useful for debugging and hit testing but is not a mature renderer-neutral protocol.

Define the relationship between:

```text
Mounted/semantic tree
Layout result
Hit-test data
Paint/primitive frame
Diagnostics
```

Avoid making semantic `Button`/`Text` enum variants the long-term renderer protocol.

---

## P2.2 Define the renderer-neutral primitive protocol

Required eventual concepts:

- solid fills;
- rounded rectangles;
- borders/strokes;
- text runs/glyph/resource references;
- clips;
- transforms;
- opacity;
- z-order;
- images;
- resource handles;
- frame/surface metadata.

The protocol must support both raster/WGPU and SDF backends without embedding either backend’s material API.

---

## P2.3 Replace reverse-preorder hit testing with explicit paint/hit order

Current hit testing checks frame nodes in reverse order.

Before overlays/transforms/clips:

- define explicit stacking order;
- clip-aware hit testing;
- transforms;
- visibility/inertness;
- pointer-events policy;
- hit shapes versus layout bounds.

Keep hit testing runtime-owned or in a neutral scene module, not renderer-owned.

---

# 13. Styling track after layout/runtime foundations

## P2.4 Resolve layout-affecting style ownership

Padding is computed style, while gap remains separate direct layout intent.

Before adding margin, min/max sizes or alignment, decide which layout-affecting values participate in:

- tokens;
- recipes;
- variants;
- state layers;
- computed style;
- inheritance/fallback.

Do not let layout accumulate a parallel non-style configuration model.

---

## P2.5 Add recipes, variants and interaction states only after mounted state exists

Future styling layers:

```text
theme tokens
control recipe
variant
interaction state
local override
computed style
```

Require mounted hover/pressed/focus/disabled state first.

Do not implement recipes as static builder aliases disconnected from runtime state.

---

## P2.6 Reassess style crate extraction later

Current core ownership of token maps/resolution is an accepted temporary boundary.

Reassess `runenui_style`/`runenui_theme` only when:

- external themes;
- recipes;
- variant/state resolution;
- fallback/inheritance;
- independent conformance tests

create a meaningful dependency boundary.

---

# 14. Host, text, controls and advanced systems

## P2.7 Host contract

Define later, after effects/input/semantics/render protocol:

- surfaces/windows;
- normalized input;
- cursor;
- clipboard;
- IME;
- drag/drop;
- file dialogs;
- wake/redraw;
- accessibility bridge;
- resource/measurement provider;
- timers/subscriptions.

Runenwerk implements an adapter. A standalone desktop host can use Winit later.

---

## P2.8 Text subsystem

Add `runenui_text` only when labels are no longer sufficient.

Required pressure:

- shaping;
- font selection/fallback;
- wrapping;
- baselines;
- editable text;
- selection/cursor;
- IME;
- accessibility text ranges;
- caching/resource invalidation.

The deterministic character-count provider remains test-only.

---

## P2.9 Standard controls

Order after identity, semantics, event policy and testing:

1. label/text
2. button
3. checkbox/radio
4. slider
5. text input
6. scroll container
7. list/menu
8. overlays/popovers
9. editor controls
10. docking/workspaces

Each control contract includes semantics, interaction, style states, layout and tests.

---

## P2.10 Multi-surface, overlays, docking and live reload

These remain deferred until the foundational contracts exist.

Dependencies:

- persistent mounted identity;
- surface ownership;
- z-order/clips/transforms;
- focus scopes;
- pointer capture;
- effects/host commands;
- serialization/source identity;
- invalidation and replay.

---

# 15. Tooling and validation improvements

## P1.29 Align `cargo validate` with CI

Current `xtask` runs:

```text
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings
```

CI runs stable tests with `--locked` and also tests MSRV.

Recommended:

- add `--locked` to local test;
- provide `cargo validate-msrv` or `cargo validate --msrv`;
- optionally add `cargo doc --workspace --no-deps`;
- add Markdown relative-link validation;
- keep the default command fast enough for normal use.

---

## P1.30 Add a Codex/agent execution contract

Create `AGENTS.md` with:

- architecture authorities;
- default branch and merge policy;
- no Actions-authored source commits;
- required preflight;
- required validation;
- no compatibility layers unless explicitly approved;
- no new crates without boundary review;
- update roadmap/status with implementation;
- inspect actual files before editing;
- one cohesive PR per accepted slice;
- report exact files, tests and SHA.

This reduces repeated context consumption in Codex tasks.

---

## P1.31 Refresh context export profiles

Ensure context profiles include:

- current workspace and docs;
- `AGENTS.md`;
- current roadmap/remediation backlog;
- source/tests for the active slice;
- no generated context files;
- no stale Runenwerk paths;
- legacy only through explicit audit profile.

Create a small Codex profile rather than sending the full repository export for each task.

---

# 16. Prioritized implementation sequence

## Phase 0 — Repository stabilization

1. Close/delete PR #65 and branch.
2. Fresh authoritative checkout.
3. Delete merged branches and configure cleanup.
4. Correct version/license/publish metadata.
5. Fix stale status/crate/target documentation.
6. Add agent/contributor execution rules.

## Phase 1 — Finish the current layout contract

7. Authoritative surface measurement cutover.
8. Single measured layout result.
9. Content-box constraints and row/column propagation.
10. Overflow diagnostics.
11. Numeric geometry invariants.
12. Layout boundary review.

## Phase 2 — Runtime foundation before controls

13. Mounted identity/reconciliation design.
14. Duplicate ID/key diagnostics.
15. Focus preservation and runtime-local state.
16. Consolidated input/event pipeline.
17. Real button activation and pointer capture.
18. Trace redesign.
19. Effects contract.

## Phase 3 — Semantics and output contracts

20. Semantic/accessibility tree.
21. Testing harness/conformance suite.
22. Renderer-neutral primitive protocol.
23. Explicit stacking/clip/transform hit testing.
24. First real host/renderer adapter proof.

## Phase 4 — Framework breadth

25. Styling recipes/variants/state layers.
26. Standard controls.
27. Text subsystem.
28. External source/document model.
29. Live preview/hot reload.
30. Overlays, multi-surface and docking.
31. Stable `runenui` facade crate.

---

# 17. What is not a defect

The following decisions remain sound:

- keeping one workspace repository;
- keeping `runenui_core` and `runenui_runtime` as the only framework crates for now;
- excluding legacy crates from the active workspace;
- using typed Rust elements as the mandatory foundation;
- keeping builder APIs beneath optional macro sugar;
- keeping renderer and host integrations out of core/runtime;
- delaying `runenui_layout`, `runenui_render`, `runenui_theme` and `runenui_testing` until real pressure exists;
- using a deterministic measurement provider for tests;
- using conditional root composition instead of introducing routing for the Counter;
- preserving the simple two-argument update form.

---

# 18. Definition of a healthy next baseline

The repository is ready to continue broader roadmap work when:

- PR #65 and temporary infrastructure are gone;
- current docs accurately describe actual implementation;
- surface publication uses one constraints/provider path;
- no `SurfaceLayoutMetrics` or hidden character measurement remains;
- invalid numeric geometry cannot silently enter frames;
- layout measures each node once per publication;
- constraints and overflow behavior are covered by conformance tests;
- the next identity/event/effects designs are recorded before controls expand;
- CI is read-only and all implementation comes from a real checkout;
- merged branches are routinely deleted.
