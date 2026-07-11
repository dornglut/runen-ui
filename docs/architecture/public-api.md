# M1 Public API Contract

> **Category: Current contract**

This document records the reviewed public surface after M1. It is the durable
inventory and rationale for the breaking correction; source-level Rust docs remain
the authority for individual signatures.

## Inventory and ownership

`runenui_core` publicly owns:

- values and errors: `LogicalLength`, `LogicalLengthError`, `Color`,
  `EdgeInsets`, `Radius`, and their literal/token unions;
- authored identity: `ElementId`, `ElementKey`, `TokenId`, typed color/spacing/
  radius token references, `IdentifierError`, and literal-validation macros;
- token definition and resolution: `StyleTokens`, `TokenFamily`,
  `DuplicateTokenDefinition`, `StyleIntent`, `ComputedStyle`, provenance,
  unresolved-token diagnostics, and the two pure resolution functions;
- typed authoring: `Text`, `Button<Action>`, `Container<Action>`, their public
  constructors and kind-specific methods, `Element<Action>`, read-only built-in
  element views, `IntoElement`, `IntoElements`, and `text`/`button`/`row`/`column`;
- macros: `element!` erases one builder expression, `children!` collects any
  number of heterogeneous builders, and identifier/token literal macros validate
  compile-time literals.

All fields of public core structs are private. `ElementKind` and `Axis` are
deliberately exhaustive closed proof enums: M2 replaces the extension gate rather
than pretending this M1 built-in vocabulary is already extensible. Value unions
are exhaustive because downstream matching is part of their current authored
contract. `IntoElement` and `IntoElements` are intentionally open for downstream
builder wrappers and iterator/collection composition; they do not constitute the
M2 widget protocol.

`runenui_runtime` publicly owns:

- the open `UiApp` and `MeasurementProvider` traits;
- `AppRuntime` construction, application dispatch, read-only state/root/trace/
  focus/index access, proof input policy, activation, and surface publication;
- validated constraints, measurement requests/results and baseline errors;
- current input/event/result vocabulary;
- opaque `RuntimeNodeId`, borrowed `RuntimeNodeRef`/`RuntimeTreeIndex`, and
  read-only identity diagnostics;
- read-only generated trace, frame, style-report, layout-report, and publication
  products plus their accessors and deterministic debug formatters;
- `LogicalSize` as a non-negative finite authored/publication size and fallible
  `LogicalPoint` as a finite signed input coordinate.

All generated-product fields are private. `RuntimeNodeId`, `RuntimeTreeIndex`,
`SurfaceFrame`, `SurfaceNode`, style/layout report nodes, `Trace`, `TraceTarget`,
and `FocusState` have no normal public construction or mutation path. Legitimate
instances come from `AppRuntime`, publication, or another public runtime behavior.
`SurfaceBuildContext`, constraints, measurement requests, and typed input events
remain constructible because they are inputs rather than generated products.

Runtime input, policy-result, diagnostic, trace-event, measurement-kind, and
surface-kind enums are `#[non_exhaustive]` because their proof vocabulary will
grow. `AxisLimit` remains exhaustive because finite versus unbounded is the whole
constraint state. `UiApp`, `MeasurementProvider`, and core conversion traits are
open; no M2 widget extension point is prematurely sealed.

The core prelude contains only ordinary typed builders, identity, `Element`,
`LogicalLength`, and conversion traits. The runtime prelude contains only
`AppRuntime`, `UiApp`, `LogicalSize`, and `SurfaceBuildContext`. Specialized
style, diagnostics, input, measurement, and generated-product inspection use
explicit root imports.

No public struct exposes public fields. Public generic bounds are local:
construction and direct dispatch accept any action type; only activation paths
require `Action: Clone`, because the current immutable transient tree retains an
action while dispatch consumes a duplicate. M1 does not move this bound onto
`UiApp`, `Element`, or `AppRuntime`. Removing the final activation clone requires
the M2/M3 action/mounted-storage design rather than hidden interior mutation.

## Defects, alternatives, and decisions

### Logical values

The defect was competing unchecked `Px` and `Length` wrappers plus raw-float
geometry constructors that admitted NaN, infinity, negative extents, and unstable
float equality. Keeping both types with validation would preserve vocabulary
ambiguity; silently normalizing authored invalid values would hide mistakes; a
general unit algebra would exceed current needs. M1 therefore uses one
`LogicalLength`: finite, non-negative, privately represented, fallible from `f32`,
infallible from bounded unsigned integers, and saturating for the arithmetic used
by current layout. `LogicalSize` contains two logical lengths. Signed points are
fallible finite pairs. Constraints structurally contain valid lengths and raise an
inverted finite maximum to the minimum. Baselines reject non-finite, negative, and
out-of-height values. Runtime arithmetic saturates before product construction.

This is a logical-coordinate contract; later host scale factors map logical to
physical pixels without introducing another authored length type.

### Identity and tokens

The defect was unchecked string construction, first-match duplicate IDs, sibling
key ambiguity, and token maps that silently overwrote definitions. Dynamic ID,
key, and token constructors now return `IdentifierError`; literal macros validate
at compile time. Builder string convenience validates immediately and retains an
invalid authoring diagnostic instead of storing an invalid identifier or silently
doing nothing. Runtime indexing emits stable path-based diagnostics sorted by
duplicate path, diagnostic kind, and value. Element IDs are unique tree-wide;
keys are unique among siblings. Ambiguous authored activation returns
`ActivationResult::AmbiguousId` rather than selecting the first node.

Token definition uses `define_*` methods that return
`DuplicateTokenDefinition`; the existing value is never replaced. The unused
generic length-token family and value union were deleted. Persistent keyed
matching remains an explicit M3 boundary: M1 validates authored sibling keys but
does not reconcile or retain mounted identity.

### Typed configuration and composition

The defect was a flat `Element` builder whose `gap`, `enabled`, `disabled`, and
`on_press` calls appeared successful on incompatible kinds. Returning an error
from every flat call would retain the misleading API shape; downcast-and-ignore
was rejected; parallel typed and flat paths were rejected. `text`, `button`,
`row`, and `column` now return typed builders. Only shared identity/style methods
are shared; button and container behavior remains kind-specific. `IntoElement`
is the single erasure boundary. The old argument structs, direct `Element`
constructors, flat setters, and `*_with` helpers were removed.

Tuple implementations were inherently finite. `IntoElements` now accepts any
iterator or collection of one element-builder type, covering empty, single,
optional, vector, array, and iterator-produced children. `children!` creates a
`Vec<Element<_>>` for arbitrary heterogeneous static children and nesting. It has
no arity list. `element!` accepts exactly one builder expression and invokes the
same erasure operation; it has no separate `action=` grammar. `on_press` is the
only button-action term, while `id`, `key`, `gap`, `padding`, and children retain
one builder meaning each.

### Generated products and evolution

The defect was public constructors for traversal IDs, indexes, frames, frame
nodes/kinds, style report nodes, traces, and focus state. Private fields alone did
not prevent inconsistent products because those constructors accepted arbitrary
parts. Constructors and mutation methods are now internal. Public accessors and
debug formatters preserve inspection. Compile-fail doctests protect representative
visibility and incompatible-configuration guarantees; behavior tests construct
products through runtime/publication.

This is the smallest M1 boundary. It does not introduce generation IDs, mounted
reconciliation, widget extension, action mapping, custom layout, paint scenes,
semantics, effects, or host behavior.

## Migration table

| Removed proof API | M1 contract |
|---|---|
| `Px`, `Length`, `LengthToken`, `LengthValue`, `Spacing` alias | `LogicalLength`, typed spacing/radius values |
| unchecked `ElementId::new`, `ElementKey::new`, `TokenId::new` | fallible constructors; checked literal macros; builder diagnostics |
| `StyleTokens::with_*` / `insert_*` replacement | non-overwriting `define_* -> Result` |
| `TextArgs`, `ButtonArgs`, `ContainerArgs` | `Text`, `Button<Action>`, `Container<Action>` builders |
| `Element::{text,button,container}` and `*_with` | typed free builders plus `IntoElement` |
| flat `Element::gap/enabled/disabled/on_press` | kind-specific typed builder methods |
| tuple child implementations through eight | iterator/collection `IntoElements` and arity-free `children!` |
| nested macro grammar and `action=` | ordinary builder expression and canonical `on_press` |
| public generated-product constructors/mutators | runtime generation plus read-only accessors |
| broad core/runtime preludes | small ordinary-use preludes; explicit specialist imports |
