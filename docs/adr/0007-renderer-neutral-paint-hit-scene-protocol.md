# ADR 0007: Renderer-neutral paint and hit-test scene protocol

> **Category: ADR**
>
> **Status:** Accepted
>
> **Decision date:** 2026-08-18
>
> **Milestone:** M6
>
> **Reviewed baseline:** `8e09a61832e2077db0e1366472b628c9b2478880`
>
> **Acceptance condition:** This ADR is operative only after the exact M6A0
> architecture/conformance package containing it is explicitly accepted by the
> repository owner and merged. Its accepted status describes the intended state
> of that accepted squash; an unmerged branch never overrides accepted `main`.
> M6 implementation remains blocked until the required bounded post-merge M6A0
> current-contract reconciliation is also accepted and accepted-main validated.

## Context

M5 completed renderer-independent semantics and deterministic public testing.
M6 must replace the remaining renderer and hit-testing proofs with a
backend-neutral scene protocol without weakening mounted identity, displayed
input, semantics, or publication atomicity.

The accepted baseline deliberately does not already contain that protocol:

- `Widget::paint` returns `WidgetPaintProof`, a category/description proof value
  that ADR 0003 explicitly says is not a primitive scene;
- `SurfaceFrame` aligns mounted identity, logical bounds, widget type, proof
  paint, diagnostics, and computed style and exposes reverse-order rectangle hit
  testing;
- runtime pointer ingress does not use that public hit method. It copies
  `(MountedNodeId, LogicalRect)` into a second private retained
  `HitTestSnapshot` ring and resolves displayed input there;
- ADR 0005 and accepted M4 `SURFACE-*` rows already make
  `SurfaceInputContext` authoritative for logical surface identity, coordinate
  revision, exact displayed hit-test generation, bounded historical retention,
  retirement, and no retargeting through current geometry;
- public `WidgetInvalidation` has no hit-test-specific bit although runtime
  `DirtyPhases` has `HIT_TEST`; ordinary hit recomputation is currently implied
  by layout;
- non-structural publication planning clones the complete `SurfaceCache` before
  replacing dirty phase facts. Issue #59 records the resulting narrow-update
  cost;
- semantic publication is already an independently typed sibling and must not
  be folded back into renderer or hit-test authority;
- `runenui_testing` retains only an ordinary immutable `SurfacePublication`, and
  the genuine downstream external-widget package directly implements the
  proof-level paint hook.

M6 therefore needs a real scene protocol and a clean migration, not a wrapper
around the current proof frame/cache.

## Inherited authority this ADR does not supersede

This ADR refines renderer/hit publication only. It does not redefine:

- ADR 0004 mounted lifetime, `MountedNodeId`, reconciliation, lifecycle, or
  mounted-tree authority;
- ADR 0005 canonical event routing, `SurfaceId`, `SurfaceInputContext`, exact
  displayed-generation targeting, retained historical targeting, target
  validation, or no-retargeting behavior;
- ADR 0006 queue, effects, wake/redraw, trace, and transaction causality;
- accepted M4 `SURFACE-*` observations;
- accepted M5 semantic contribution, semantic identity, semantic publication,
  semantic action, or testing contracts;
- M5's staged publication transaction:
  `admit -> read-only/staged plan -> candidate-dependent final preflight -> commit`;
- runtime layout ownership, which remains intentionally limited until M7;
- production text shaping/editing/resource production, which remains M8 work;
- native host, accessibility, and renderer backends, which remain later work.

The M6 conformance matrix references inherited observations instead of creating
new IDs for already accepted behavior.

## Decision

### One publication authority, distinct sibling products

`runenui_runtime` continues to own one live surface-publication authority. A
successful publication atomically aligns distinct immutable products:

- `PaintScene` — renderer-neutral visual primitives and renderer metadata;
- `HitTestScene` — input-facing shapes, transforms/clips, pointer policy,
  deterministic order, and runtime-injected mounted targets;
- layout result/report;
- semantic publication;
- semantic diagnostics and ordinary diagnostics;
- the exact displayed `SurfaceInputContext` associated with the hit scene.

These products may share immutable internal phase storage but are not one mixed
node model. Paint consumers do not receive semantic roles, `WidgetTypeId`,
concrete control kinds, mutable mounted state, or backend handles. Semantic
consumers do not receive paint primitives. Layout and diagnostics remain
inspectable without becoming renderer input.

`SurfacePublication` is the public alignment boundary. It exposes sibling
products read-only. Extracting a snapshot never transfers live runtime authority.

### Core owns contribution vocabulary; runtime owns composed scenes

The dependency direction remains `runenui_runtime -> runenui_core`.

`runenui_core` owns public host- and renderer-neutral contribution vocabulary:
focused paint contribution types, focused hit contribution types, and the
existing widget-owned contribution seam. M6 does not justify a broad
`Element`/`Widget` source reorganization or a registry of concrete controls.

`runenui_runtime` owns scene composition, mounted-target injection, layout-to-
surface placement, deterministic global order, retained displayed scenes, dirty
phase planning, publication transaction state, and immutable public scene
snapshots. Widgets cannot author or forge `MountedNodeId`, `SurfaceId`,
`SurfaceInputContext`, scene-order identity, or live publication generations.

This follows the accepted M5 precedent: add focused production vocabulary at
its ownership seam rather than splitting unrelated lifecycle/event/layout/
semantic responsibilities.

### Coordinate spaces are explicit

Widget paint and hit contributions are authored in **owner-local logical
coordinates**. Contribution contexts expose the owner's final local logical size
and only the additional read-only facts named below; they expose no absolute
surface origin.

Runtime composes owner placement with each contributed local transform. Public
scene items/regions therefore contain an exact finite **local-to-surface affine
transform**. A consumer never consults `SurfaceFrame`, layout reports, mounted
storage, or a widget tree to recover surface placement.

Each explicit clip is self-contained: it carries its own logical clip shape and
finite **clip-to-surface affine transform**. This lets runtime preserve an
ancestor clip chain later without requiring all clips to share the item's local
space. Paint and hit consumers evaluate the transforms contained in the scene;
there is no hidden layout transform outside the immutable product.

Logical scene geometry remains logical. Raster scale does not alter this
coordinate contract.

### Paint contributions are owner-local immutable fragments

The production paint hook replaces `WidgetPaintProof` with immutable
`PaintContribution` evaluated from mounted widget state during paint work. Its
read-only context contains the owner's final local logical size and resolved
computed style. It receives no runtime arena, surface origin, semantic tree,
backend/GPU/host object, or resource cache.

A contribution is an ordered list of self-contained items. Each contributed
item owns a primitive, owner-local transform, zero or more clips, opacity, and a
snapshot-local signed layer. Runtime composes it into the surface-space contract
above. Self-contained items are preferred over unbalanced push/pop command
stacks so malformed nesting is impossible.

M6's minimum primitive vocabulary is:

- filled logical rectangle;
- stroked logical rectangle with finite non-negative logical width;
- image resource reference in a logical rectangle;
- shaped-text-run resource reference at logical placement.

Production shaping and image decoding are not primitive semantics. Text/image
primitives reference resources supplied by another owner.

All transforms must be finite. Opacity must be finite and in the closed
`[0, 1]` interval; checked construction rejects values outside that contract
rather than clamping or silently normalizing them. Layer values are ordering
facts only, never identity.

Global paint order is the stable ascending tuple:

```text
(layer, mounted logical preorder, contribution-local order)
```

Equal tuples preserve contribution order. Hash-map/storage/backend iteration
cannot redefine it. Later M7 stacking policy may choose layer values but consumes
this ordering contract rather than replacing it.

### Hit contributions are independent from paint and semantics

Hit testing is not inferred from paint primitives and paint is not inferred from
hit testing. A widget may contribute zero or more owner-local `HitRegion` values
through a distinct hit hook. The context contains the owner's final local logical
size and no surface or mounted-target authority.

Each contributed region contains:

- a logical rectangle or rounded-rectangle shape;
- an owner-local finite affine transform;
- zero or more explicit clips;
- a snapshot-local signed layer;
- one `PointerPolicy`.

`PointerPolicy` has three outcomes:

- `Target` — the topmost containing eligible region resolves to its
  runtime-injected mounted owner;
- `Block` — the topmost containing eligible region terminates physical hit
  testing with no mounted target;
- `PassThrough` — the region is skipped and lower regions remain eligible.

The default downstream hit contribution is empty. Layout participation alone
never implies pointer targetability. Built-in/downstream controls that require
direct physical pointer targeting opt in explicitly. An ancestor without its
own targetable region still participates in the accepted capture/bubble route
when a descendant is the target.

Pointer participation is **not derived from semantic state**. In particular,
semantic `disabled`, `hidden`, and `inert` flags do not automatically remove,
block, or retarget a physical hit region. An owning widget/runtime control policy
may independently change hit contribution/policy and invalidate hit testing when
its interaction contract requires that behavior. This preserves accepted routed
behavior such as disabled controls remaining physically targetable while a later
canonical command/default eligibility check suppresses activation.

M6 scene visibility means whether a region exists in the composed hit scene; it
is an explicit hit/publication fact, not an implicit read of the semantic tree.

### Hit order is exact

Runtime composes hit regions using the same stable ascending tuple as paint:

```text
(layer, mounted logical preorder, contribution-local order)
```

The public `HitTestScene` stores that deterministic order. **Topmost** means
reverse traversal of this exact order. No storage iteration, backend order, or
current-tree traversal may change it after publication.

Layer is snapshot-local ordering policy, not target identity. M7 may later
supply richer stacking decisions by selecting layer values without changing the
M6 hit-order contract.

### The hit scene is the canonical displayed input snapshot

The public immutable `HitTestScene` associated with a successful
`SurfacePublication` is the exact scene retained by `SurfacePublicationState`
for pointer targeting. Runtime must not copy it into a separate rectangle-only
snapshot or re-run `SurfaceFrame` hit testing.

A retained scene stores the exact runtime-issued `SurfaceInputContext` for that
displayed publication plus immutable region data. `SurfacePublication::input_context`
therefore names the same context as its hit scene. The retained generation ring
stores cheap immutable handles to these exact scenes.

`SurfaceInputContext::hit_test_generation()` remains the sole public displayed
hit-scene generation. M6 adds no second hit-scene generation or target namespace.
Current and retained contexts resolve only against the exact scene they name;
retired/missing/foreign/mismatched contexts retain accepted M4 behavior.

`MountedNodeId` remains the canonical route target injected by runtime into
`Target` regions. Scene order, resource IDs, authored IDs, semantic IDs, and
primitive identities never substitute for mounted target identity.

A successful surface publication may issue a fresh displayed hit generation even
when region content is unchanged, preserving accepted M4 publication semantics;
the immutable region storage may still be shared across wrappers.

Paint has no independently targetable generation because no accepted consumer
requires one. Paint snapshot identity is the enclosing immutable
`SurfacePublication`; another counter solely for symmetry is rejected.

### Hit testing uses one exact algorithm

Runtime input and public deterministic consumers use the same `HitTestScene`
semantics.

For each region from topmost to bottommost:

1. map the surface-logical point through the inverse of the region's exact
   local-to-surface transform;
2. evaluate the region shape in that local space;
3. evaluate every explicit clip using that clip's own clip-to-surface transform;
4. if the point is contained and unclipped, apply `PointerPolicy`.

A non-invertible region transform makes that region non-hittable with
deterministic diagnostic coverage; it never falls back to untransformed
geometry. A non-invertible clip transform deterministically excludes the region
for that clip and is diagnosed; it never makes the clip disappear.

On a containing eligible region:

- `Target` returns the runtime-injected mounted owner;
- `Block` returns blocked/no-target and stops;
- `PassThrough` continues to the next lower region.

This replaces both proof-level `SurfaceFrame::hit_test` authority and the private
copied rectangle resolver. A surviving debug/layout snapshot must have a
separate truthful purpose and no hit authority.

### Hit invalidation is explicit

M6 adds public `WidgetInvalidation::HIT_TEST` and includes it in `ALL`.

The dependency rules are:

- `HIT_TEST` invalidates the widget's hit contribution and dirties hit-scene
  composition;
- `LAYOUT` implies hit work because contribution context/surface placement may
  change;
- structural changes rebuild hit topology/order;
- a widget whose state changes hit shape, existence, layer, clips, transform, or
  pointer policy independently of layout must request `HIT_TEST`;
- `PAINT` alone never changes hit testing;
- `SEMANTICS` alone never changes hit testing;
- `INTERACTION` does not silently imply widget hit invalidation. Runtime-owned
  interaction policy may schedule hit work only where a later accepted control
  contract explicitly says so.

### Paint invalidation is explicit

`PAINT` invalidates widget paint contribution. Layout and relevant resolved-style
changes schedule paint work because contribution context/placement may change.
A widget whose own state changes visual output must request `PAINT`.
Semantic-only work does not dirty paint by implication.

The mounted `WidgetPaintProof` cache is removed during the M6 scene cutover.
Runtime may cache/share immutable resolved paint fragments, but no proof paint
capability remains a competing production authority.

### Retained publication uses immutable phase products

Issue #59 is the first implementation prerequisite after M6A0.

The one runtime surface-publication authority stores retained phase products via
cheap immutable handles or an equivalent persistent representation.
Non-structural planning starts from shared handles and allocates/replaces only
products whose dirty dependencies require new values.

The implementation must guarantee:

- clean/focus-only/semantic-only publication does not deep-clone every retained
  renderer/layout/hit/paint/diagnostic vector;
- unchanged phase products are demonstrably reused;
- replacement ownership is explicit per dirty phase;
- deterministic output is independent of storage representation;
- the public publication is derived from aligned phase products, never a second
  mutable cache;
- rejected/terminally failed plans preserve M5 zero-partial-commit semantics;
- semantic publication remains an independent sibling coordinated by the same
  final commit.

The internal smart-pointer/container type is not public protocol. A new cache
abstraction layer is unjustified if ordinary immutable sharing satisfies the
requirements.

### Resource references are logical, provider-owned values

M6 introduces renderer-neutral resource references with explicit kind, such as
image or shaped text run. A reference is a logical immutable value supplied to a
widget by a resource-producing owner; it is not bytes, a provider object, GPU
handle, font database object, native image, mounted identity, semantic identity,
or scene-item identity.

Reference equality is stable by value across scene snapshots within the issuing
provider's namespace. References are therefore not scene-local indices. The
provider that issued a reference owns resolution; a scene/consumer may carry and
compare the opaque logical value but cannot infer bytes or backend realization
from it.

M6 specifies reference identity/kind and deterministic consumer requirements. It
does not load, decode, shape, upload, evict, or cache resource bytes.
Deterministic M6 proofs may use fixture providers/resources. M8/M10 later provide
production text/resource producers and backend realization behind this boundary.

Missing or kind-mismatched resources are deterministic consumer/admission
errors. They never authorize reinterpretation as another primitive or concrete
widget kind.

### Scale is renderer metadata, not coordinate authority

Paint-scene metadata may carry one validated positive finite raster scale. All
layout, paint geometry, transforms/clips, hit shapes, and pointer ingress remain
in logical coordinates. Scale informs renderer/damage realization only; it does
not introduce physical-pixel/DPI/native-window types into core/runtime input and
does not replace `SurfaceInputContext` coordinate authority.

The deterministic headless default is `1.0`. Production host ownership of scale
changes belongs to M10.

### Damage is conservative and sound

`PaintScene` carries deterministic logical damage facts for the current paint
publication. Damage must never under-report changed renderer-relevant output. A
full-surface damage rectangle is valid whenever finer invalidation cannot be
proven. Empty damage is valid only when paint-scene content and all
renderer-relevant metadata are unchanged under the accepted comparison contract.

M6 does not require an optimal damage algorithm. M7/M10 may improve precision
without changing soundness.

### Capabilities validate consumers; they do not rewrite the scene

M6 defines neutral scene requirements/capabilities sufficient for a consumer to
declare which accepted primitive/resource features it can realize. Runtime
derives requirements from the canonical paint scene. Capability checking reports
unsupported requirements deterministically.

Capabilities do not make core/runtime emit backend-specific alternate scenes,
silently lower primitives into unrelated approximations, or select a concrete
renderer. A consumer accepts the canonical requirements or reports what it
cannot realize.

### Public observation uses ordinary immutable products

`runenui_testing` remains convenience authority only. It retains the latest
ordinary `SurfacePublication` and exposes/asserts public paint/hit scenes. It
does not fabricate scene IDs, mounted targets, generations, regions, publication
state, or a second hit algorithm.

The genuine downstream custom-widget package must migrate from
`WidgetPaintProof` through public contribution APIs. M6 also requires two
independent deterministic scene consumers; at least one must be a genuine
external/custom renderer consumer that interprets paint without concrete widget
kinds such as `Button`.

### Clean pre-1.0 migration

M6 preserves no proof-era renderer/hit authority through aliases.
Implementation slices cleanly remove or truthfully narrow:

- `WidgetPaintProof` and its mounted capability cache;
- renderer-facing paint claims on `SurfaceFrame`/`SurfaceNode`;
- `SurfaceFrame::hit_test` / `hit_test_id` as hit authority;
- the private copied `HitTestSnapshot` once `HitTestScene` is canonical;
- debug/test helpers that independently reproduce renderer/hit semantics;
- stale docs/support claims naming proof products as production scene inputs.

`SurfaceLayoutReport`, style reports, mounted inspection, and diagnostics may
remain where they retain separate truthful ownership. No compatibility module,
deprecated wrapper, duplicate type alias, or hidden parallel path is required
before 1.0.

## Implementation and acceptance sequence

M6A0 freezes architecture/conformance only and owns no scene behavior.

After the M6A0 architecture/conformance PR is explicitly owner-accepted,
guarded-squash-merged, and content identity is verified, perform one bounded
M6A0 current-contract reconciliation. That reconciliation records accepted ADR
0007/matrix authority, the M6 35-row blocked baseline, umbrella/pickup state, and
the exact accepted A0 base in roadmap/status/support/work-tracking/retention
owners. It must itself be owner-accepted, merged, and accepted-main validated.
**No M6 implementation branch may start before that reconciliation completes.**

Then the minimum implementation order is:

### M6A — persistent retained-publication substrate (#59)

Replace whole-`SurfaceCache` cloning with immutable shared phase products while
preserving accepted M5 transaction/failure behavior. Prove narrow publications
reuse unchanged products. Do not add a parallel cache or scene behavior unless
strictly required to prove the storage boundary.

### M6B — canonical paint/hit scene kernel and displayed-hit cutover

Introduce focused core contribution vocabulary, explicit hit invalidation,
public immutable `PaintScene`/`HitTestScene`, runtime target injection, basic
rectangle primitive/region composition, exact deterministic order, and the
canonical retained hit-scene ring. Migrate the built-in/downstream vertical
proof. Remove `WidgetPaintProof`, duplicate private hit snapshots, and
proof-level hit-resolution authority where their replacements become live.
Identity transforms, no clips, opacity `1`, layer `0`, and `Target` are sufficient
for the initial kernel; M6C completes the wider vocabulary without introducing a
second scene path.

### M6C — transforms, clips, resources, metadata, damage, and capabilities

Complete transformed/rounded/clipped composition and hit semantics, resource
references, raster-scale metadata, sound damage, pointer-policy breadth, and
consumer capability checking on the same M6B products.

### M6D — independent consumers, migration, and milestone closure

Prove two independent deterministic consumers, including one genuine downstream
renderer without concrete widget-kind knowledge; complete public testing scene
assertions; remove remaining obsolete renderer/hit proof claims; run integrated
M4/M5 inheritance plus M6 conformance; reconcile current authority and close M6.

A later critical audit may split a slice only when that reduces a real acceptance
boundary without introducing duplicate authority. M6B cannot precede accepted
M6A because doing so would knowingly build new scene products around the cache
architecture #59 is required to replace.

## Rejected alternatives

### Keep `SurfaceFrame` and wrap it as `PaintScene`

Rejected. It contains mounted/widget/debug/style facts and proof paint rather
than renderer primitives, preserving the mixed proof authority M6 must retire.

### Keep a private hit snapshot beside a public hit scene

Rejected. Two representations can diverge. The public immutable scene retained
by runtime is the one hit authority.

### Add an independent scene target/generation namespace

Rejected. `MountedNodeId` and `SurfaceInputContext` already own route-target and
displayed-generation lifetimes. Another namespace adds stale/synchronization
ambiguity without a distinct consumer.

### Derive hit testing from paint primitives

Rejected. Visual coverage and interaction policy differ; transparent/non-visual
controls, overlays, pass-through regions, and future styling make the coupling
incorrect.

### Make every layout rectangle targetable by default

Rejected. Layout participation does not imply pointer participation. Explicit
hit contribution prevents containers/text from becoming accidental targets.

### Derive physical hit policy directly from semantic disabled/hidden/inert state

Rejected. The semantic product is an independent accessibility/meaning sibling,
not physical input authority. Existing routed/default behavior may intentionally
receive pointer events for a disabled control while suppressing activation.
An owning control may choose aligned semantic and physical policy, but each
product must be invalidated/contributed through its own authority.

### Add a giant renderer/widget trait or split the whole widget module first

Rejected. #10 remains a broad concentration audit. M6 has focused paint/hit
seams; a whole-protocol refactor would mix unrelated responsibilities.

### Generate backend-specific scenes from capability negotiation

Rejected. It lets a backend influence canonical framework semantics.
Capabilities validate consumers of one scene.

### Require minimal damage immediately

Rejected. Sound conservative damage establishes the protocol; precision is an
optimization once real consumers exist.

### Add a separate M6 delivery charter immediately

Rejected as duplicate authority. ADR 0007 owns durable scene/publication
architecture, the M6 matrix owns observable acceptance, the roadmap owns durable
milestone order, and GitHub issues own volatile execution state.

## Consequences

Positive consequences:

- renderer/hit consumers receive explicit products rather than widget/debug
  proofs;
- public scene coordinates and ordering are self-contained and deterministic;
- accepted M4 displayed-input identity remains intact;
- runtime no longer needs two hit-test representations;
- custom widgets gain explicit paint/hit contribution without registration;
- semantic, paint, hit, layout, and diagnostics remain independently owned;
- #59 is resolved before real scenes multiply retained-cache copying;
- resource, scale, damage, and capability boundaries are defined without adding
  a concrete backend;
- M7–M10 can consume stable neutral products rather than reopen ownership.

Costs and constraints:

- M6 deliberately breaks proof paint/hit APIs before 1.0;
- downstream custom widgets must adopt explicit hit participation and new paint
  contribution;
- hit invalidation gains one public bit and proof burden;
- exact coordinate/order/transformed hit semantics require deterministic tests;
- resource references intentionally require fixture providers until M8/M10
  production producers exist;
- retained immutable products must preserve simple staged atomicity rather than
  hide rollback mutation.

## Acceptance

The normative observable requirements are recorded in
[`../architecture/m6-conformance-matrix.md`](../architecture/m6-conformance-matrix.md).
The M6A0 architecture/conformance merge does not by itself authorize M6A. M6
implementation remains blocked until the bounded A0 current-contract
reconciliation records the accepted squash and itself passes owner acceptance,
merge, content-identity, and accepted-main validation under repository policy.
