# Layout Boundary Review

## Review status

Formal review completed on 2026-07-11 against `master` commit `633ad932cb2478bbe1c54bf136c86f5b022d2da9`, after the constraints, measurement-provider, measured-result, child-propagation, and overflow-diagnostic work was merged.

## Decision

Do not extract `runenui_layout` yet.

The implemented layout contracts are materially better than the earlier placeholder pass, but they still form one runtime-owned surface-publication pipeline rather than an independently valuable crate boundary.

Extraction now would require one of three undesirable outcomes:

1. make a layout crate depend on runtime identity, which reverses the intended dependency direction;
2. move or genericize runtime identity, geometry, style-preparation, and surface-output contracts before another consumer requires that work; or
3. extract only low-level vocabulary while leaving the actual layout orchestration in runtime, producing a nominal crate with little independent value.

Keep layout, measurement orchestration, layout diagnostics, arrangement, and surface publication in `runenui_runtime`.

The next implementation task is the renderer-neutral primitive/frame protocol. That work may clarify geometry, hit-testing, and surface-output ownership, but it must not extract `runenui_layout` as a side effect.

## Why the previous numeric rule is insufficient

The earlier review said to create `runenui_layout` when at least three listed criteria were true. That counting rule is retired.

RunenUI now has explicit constraints, a measurement-provider contract, computed style affecting geometry, and layout diagnostics. A simple count would therefore suggest extraction even though the actual ownership boundary is still absent.

Crate extraction requires both:

1. sufficiently mature contracts; and
2. hard boundary pressure that a crate would enforce or serve.

Contract maturity without independent ownership pressure is not enough.

## Implemented baseline

The current publication pipeline is:

```text
Element<Action>
  + StyleTokens
  + root LayoutConstraints
  + MeasurementProvider
  -> RuntimeTreeIndex and RuntimeNodeId-aligned resolved surface tree
  -> one publication-local measured result
  -> measurement-free row/column arrangement
  -> SurfaceFrame
  + SurfaceStyleReport
  + SurfaceLayoutReport
  -> SurfacePublication
```

The implementation provides:

- normalized finite and unbounded constraints;
- renderer-neutral synchronous text measurement requests and results;
- a borrowed measurement-provider seam;
- deterministic headless measurement;
- computed padding in measurement and placement;
- loose finite cross-axis child constraints without implicit stretch;
- intrinsic unbounded main-axis sizing;
- deterministic diagnostic-only overflow;
- exactly one text or button-label measurement per publication;
- runtime-node-aligned frame, style, and layout products;
- behavioral tests for constraints, measurement, placement, overflow, padding, hit testing, and invalid-provider sanitization.

These are real prerequisites for a future layout boundary. They do not yet establish an independent owner.

## Current ownership and coupling

### Authored model

`runenui_core` owns the authored UI model:

```text
Element<Action>
ElementKind<Action>
Axis
LayoutStyle
StyleIntent
ComputedStyle
StyleTokens
```

The current layout pass directly matches `ElementKind` and consumes resolved computed padding. There is no neutral layout-tree input independent of the authored/runtime surface preparation.

### Runtime identity

`runenui_runtime` owns:

```text
RuntimeNodeId
RuntimeTreeIndex
AppRuntime
input targeting
focus
activation
trace targets
```

Layout measurement requests may carry `RuntimeNodeId`, and `SurfaceLayoutNode` is keyed by `RuntimeNodeId`. The measured result is therefore an observation of the runtime tree, not an identity-independent layout product.

Moving these APIs into `runenui_layout` now would either create a dependency from layout back to runtime or require premature identity genericization.

### Surface output

`surface.rs` currently owns:

```text
LogicalSize and LogicalRect
SurfaceNodeKind
SurfaceNode
SurfaceFrame
SurfaceBuildContext
SurfacePublication
layout measurement and arrangement
SurfaceLayoutReport
bounds hit testing
```

Arrangement produces `SurfaceFrame` directly, and hit testing consumes that frame. Layout output and renderer-facing surface output do not yet have separate ownership contracts.

### Measurement

`MeasurementProvider` is renderer-neutral, but `TextMeasurementRequest` uses runtime-owned `LogicalSize`, `LayoutConstraints`, and optional `RuntimeNodeId` observation identity.

The seam is sufficient for current runtime publication. It is not yet proof that measurement belongs in a separate crate.

### Tests and consumers

The conformance coverage is valuable, but it remains runtime coverage:

- layout implementation is in `runenui_runtime`;
- layout behavior tests are in `runenui_runtime` tests;
- the Counter consumes the public runtime publication API;
- the debug renderer consumes `SurfaceFrame`, not an independent layout subsystem;
- no backend, host adapter, testing crate, or external repository consumes layout output independently.

There is therefore no second consumer whose dependency needs Cargo enforcement.

## Dependency review

The current Cargo graph is intentionally simple:

```text
runenui_core
  <- runenui_runtime
  <- examples/counter
```

`runenui_runtime` has one framework dependency: `runenui_core`.

A useful future direction could be:

```text
runenui_core
  <- runenui_layout
  <- runenui_runtime
```

That direction is not implementable cleanly with the current contracts because layout currently references runtime identity and directly produces runtime-owned surface products.

No current dependency cycle, optional feature boundary, build-time isolation requirement, or external consumer would be improved by adding the crate now.

## Formal extraction evaluation

### Contract prerequisites

| Requirement | Current status | Review result |
|---|---|---|
| Explicit finite/unbounded constraints | Implemented | Satisfied |
| Abstract measurement service | Implemented for text and button labels | Satisfied for the current algorithm |
| Measured result separate from arrangement | Implemented within one publication | Satisfied |
| Computed style affects layout | Padding is implemented | Satisfied but narrow |
| Deterministic layout diagnostics | Implemented and runtime-node aligned | Satisfied |
| Behavioral conformance coverage | Implemented in runtime tests | Satisfied locally |

### Boundary-pressure requirements

| Requirement | Current status | Review result |
|---|---|---|
| Multiple meaningful layout algorithms | Only one small row/column stacking policy | Not satisfied |
| Identity-independent layout input/output | Contracts use `RuntimeNodeId` and runtime surface preparation | Not satisfied |
| Independent layout consumer | None | Not satisfied |
| Cargo-enforced dependency need | None demonstrated | Not satisfied |
| Meaningful optionality or feature isolation | None | Not satisfied |
| Independent conformance harness | Tests remain runtime-owned | Not satisfied |
| Stable separation from render/hit-test output | Layout directly produces `SurfaceFrame` | Not satisfied |

The prerequisite column is mature enough to continue development. The boundary-pressure column is not.

## Extraction gate

Create `runenui_layout` only when all of the following are true.

### Required contract conditions

1. Layout input and output have ownership independent of `SurfacePublication` internals.
2. Geometry ownership is explicit for `LogicalSize`, `LogicalRect`, constraints, and computed layout boxes.
3. Runtime observation identity is either removed from the core layout contract or represented through a deliberate neutral/generic identity contract.
4. Measurement requests have the typography and resource inputs required by real layout behavior.
5. Layout conformance cases can run without constructing the full application runtime pipeline.

### Required boundary pressure

At least one hard boundary reason must also exist:

1. a renderer, host integration, testing crate, or external repository consumes layout independently;
2. Cargo must prevent a real forbidden dependency direction or isolate optional dependencies;
3. multiple substantial layout algorithms need an independently owned implementation and conformance surface;
4. layout has meaningful build, feature, or release optionality separate from runtime.

Do not create a crate solely because several vocabulary types now exist.

Do not genericize node identity, relocate geometry, or split the publication pipeline only to satisfy this gate artificially.

## What remains in `runenui_runtime`

Keep these together for now:

```text
LayoutConstraints
AxisConstraints and AxisLimit
MeasurementProvider
TextMeasurementRequest and TextMeasurement
DeterministicMeasurementProvider
LogicalSize and LogicalRect
publication-local measured layout result
row/column measurement and arrangement
LayoutOverflow
SurfaceLayoutNode and SurfaceLayoutReport
SurfaceNode and SurfaceFrame
SurfaceBuildContext and SurfacePublication
SurfaceFrame hit testing
AppRuntime::publish_surface
```

Internal module cleanup may be justified later if `surface.rs` becomes difficult to maintain, but an internal module split is not evidence for a new crate.

## Future `runenui_layout` ownership

Once the extraction gate is met, a dedicated layout crate should own:

```text
constraints
measurement inputs and outputs
intrinsic sizing
layout-tree or layout-adapter contract
row, column, flex, grid, stack, and absolute algorithms
computed layout boxes
layout diagnostics
layout conformance cases
```

It must not own:

```text
application state or actions
runtime focus and activation
native windows or host event loops
renderer backends
surface resource management
product UI
```

## Future `runenui_render` relationship

The next renderer-neutral primitive/frame protocol review should decide ownership for:

```text
frame metadata
paint primitives
text runs
clips
transforms
z-order
image and resource references
hit-test-relevant geometry
```

That protocol may consume arranged logical boxes, but it must not own layout policy.

Do not move `SurfaceFrame` or geometry types merely to make the long-term diagram look complete. Move them only when the protocol has an actual independent consumer and stable responsibility.

## Revisit triggers

Reopen this review when one or more of these events occur:

1. a second meaningful layout algorithm is implemented;
2. text shaping, wrapping, baselines, alignment, min/max sizing, or intrinsic negotiation materially expands the layout contract;
3. the renderer-neutral primitive protocol establishes a separate arranged-box consumer;
4. `runenui_testing` needs reusable layout conformance without the application runtime;
5. Runenwerk or another host/backend needs layout output independently;
6. runtime identity is decoupled from layout input/output;
7. Cargo can enforce a demonstrated dependency or optionality boundary.

Until then, keep the boundary explicit in modules and tests rather than manufacturing a crate split.

## Final verdict

The formal review confirms the existing direction:

```text
keep layout in runenui_runtime
continue using the measured-result and diagnostic contracts
perform renderer-neutral primitive/frame protocol design next
do not extract runenui_layout yet
```
