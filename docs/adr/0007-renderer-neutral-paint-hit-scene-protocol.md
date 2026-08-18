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
- that private snapshot also validates whether a checked resolved
  `MountedNodeId` belonged to the exact named historical surface generation,
  preserving accepted M4 `SURFACE-10` independently from point-hit geometry;
- that same private snapshot also supplies `current_focus_geometry()` for M4
  directional focus, so the current proof accidentally conflates an all-mounted
  published-layout geometry view with pointer hit authority even though those
  consumers have different future participation rules;
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
- `SurfaceBuildContext` already owns explicit host-neutral inputs for one surface
  publication (style tokens, root constraints, and measurement provider), making
  it the natural future boundary for neutral raster scale rather than a renderer
  setter or native host type;
- `runenui_testing` retains only an ordinary immutable `SurfacePublication`, and
  the genuine downstream external-widget package directly implements the
  proof-level paint hook.

M6 therefore needs a real scene protocol and clean migration, not a wrapper
around the current proof frame/cache.

## Inherited authority this ADR does not supersede

This ADR refines renderer/hit publication only. It does not redefine:

- ADR 0004 mounted lifetime, `MountedNodeId`, reconciliation, lifecycle, or
  mounted-tree authority;
- ADR 0005 canonical event routing, `SurfaceId`, `SurfaceInputContext`, exact
  displayed-generation targeting, retained historical targeting, target
  validation, or no-retargeting behavior;
- accepted M4 focus eligibility, scope/navigation selection, directional-focus
  geometry semantics, or the `DF-*` conformance corpus;
- ADR 0006 queue, effects, wake/redraw, trace, and transaction causality;
- accepted M4 `SURFACE-*` observations;
- accepted M5 semantic contribution, identity, publication, action, or testing
  contracts;
- M5's staged publication transaction:
  `admit -> read-only/staged plan -> candidate-dependent final preflight -> commit`;
- runtime layout ownership, intentionally limited until M7;
- production text shaping/editing/resource production, which remains M8 work;
- native host, accessibility, and renderer backends, which remain later work.

The M6 matrix references inherited observations rather than duplicating them.

## Decision

### One publication authority, distinct sibling products

`runenui_runtime` continues to own one live surface-publication authority. A
successful publication atomically aligns distinct immutable products:

- `PaintPublication` — the renderer-facing immutable update/snapshot for one
  exact logical surface, containing its `PaintRevision`, optional base revision,
  history-independent `PaintScene`, exact logical surface size, `RasterScale`,
  and sound damage;
- `HitTestScene` — the exact displayed-input snapshot, containing ordered
  input-facing shapes/transforms/clips/pointer policy, exact runtime-injected
  mounted-target membership for the published generation, deterministic order,
  and its exact `SurfaceInputContext`;
- layout result/report plus the retained layout-phase facts required by current
  runtime focus selection;
- semantic publication;
- semantic diagnostics and ordinary diagnostics.

`PaintScene` is reusable renderer-neutral logical visual content nested in the
paint publication. It is not a second live publication authority. These products
may share immutable internal phase storage but are not one mixed node model.
Paint consumers do not receive semantic roles, `WidgetTypeId`, concrete control
kinds, mutable mounted state, or backend handles. Semantic consumers do not
receive paint primitives. Layout and diagnostics remain inspectable without
becoming renderer input.

`SurfacePublication` is the public alignment boundary. It exposes sibling
products read-only. `SurfacePublication::paint_publication()` exposes the exact
paint publication aligned with that surface commit; convenience access to its
scene, if provided, is derived from that same value and does not duplicate state.
Extracting a snapshot never transfers live runtime authority.

`PaintScene` equality/content identity is history-independent: two scenes with
the same logical items/resources/order compare as the same scene regardless of
which predecessor caused their current damage or which logical surface extent
currently hosts them. Publication-relative damage, revision lineage, and target
surface extent are therefore deliberately not stored as scene content. This lets
#59 share an unchanged `PaintScene` across paint publications.

Scene requirements are a deterministic **derived view of `PaintScene` content**,
not another authoritative stored product. Runtime/consumers may cache that view
internally, but any cache must be exactly reconstructible from the immutable
scene and cannot diverge from it.

### Paint publication revisions are renderer update identity

`PaintRevision` is a runtime-owned public non-zero, non-wrapping revision value,
following the accepted `SemanticRevision` precedent. A revision is meaningful
only for the exact `SurfaceId` carried by its `PaintPublication`; consumers do
not compare revisions across surfaces as one global sequence.

The first accepted paint publication for one surface is revision `1`, has no base
revision, and carries full-surface damage. After that, runtime compares the exact
renderer-relevant snapshot tuple:

```text
(PaintScene content, logical surface size, RasterScale)
```

If that tuple is unchanged, a successful surface publication reuses the same
immutable `PaintPublication` value and `PaintRevision`; internal storage may be
shared. Semantic-only, hit-only, focus-only, diagnostic-only, and other
non-renderer changes therefore do not fabricate paint updates. They may still
advance their own accepted authorities such as displayed hit generation where
those contracts require it.

If the renderer-relevant tuple changes, runtime allocates exactly one checked next
`PaintRevision`, creates a new `PaintPublication`, and records the immediately
previous accepted paint revision as `base_revision`. Damage in that new value is
relative to exactly that base. Revision allocation occurs in the staged
publication plan/final-preflight boundary: exhaustion when a new paint revision
is required fails before commit under the existing non-wrapping terminal/atomic
counter discipline. An unchanged renderer tuple consumes no paint revision.

A renderer consumer tracks the exact `(SurfaceId, PaintRevision)` it has
successfully realized:

- if an observed paint publication has that same revision, there is no new paint
  update;
- if its current revision equals the publication's `base_revision`, it may apply
  the publication's damage incrementally and then adopt the new revision;
- if it has no matching base — including first observation, skipped revisions,
  stale state, another surface, or recovery after renderer loss — it ignores the
  incremental-damage optimization and realizes the complete immutable
  `PaintScene` using the publication's complete extent/scale metadata, then
  adopts the current revision.

RunenUI does not retain renderer acknowledgements or invent a second handshake to
make this work. The publication is always a complete snapshot; revision/base
facts only say when its damage delta is safe as an optimization. While the
matching live resource-resolver domain exists, every logical resource referenced
by that complete snapshot remains resolvable independently of any disposable
backend realization cache, so full realization does not depend on hidden prior
renderer state.

This renderer revision is deliberately distinct from
`SurfaceInputContext::hit_test_generation()`. Hit generations identify retained
input snapshots and may advance on successful surface publications even when
paint is unchanged. `PaintRevision` identifies renderer-relevant snapshot change
only. Neither substitutes for the other.

### Core owns contribution vocabulary; runtime owns composed scenes

The dependency direction remains `runenui_runtime -> runenui_core`.

`runenui_core` owns public host- and renderer-neutral contribution vocabulary:
focused paint contribution types, focused hit contribution types, and the
existing widget-owned contribution seam. M6 does not justify a broad
`Element`/`Widget` source reorganization or a registry of concrete controls.

`runenui_runtime` owns scene composition, mounted-region target injection, exact
displayed-generation mounted-target membership, layout-to-surface placement,
deterministic global order, retained displayed scenes, dirty phase planning,
publication transaction state, `PaintRevision` allocation, validated
`RasterScale`, publication-relative renderer metadata, and immutable public
snapshots. Widgets cannot author/forge `MountedNodeId`, `SurfaceId`,
`SurfaceInputContext`, displayed-snapshot membership, `PaintRevision`, paint base
revisions, scene-order identity, live input publication generations,
surface-publication extent, raster-scale authority, or damage history.

This follows the accepted M5 precedent: add focused production vocabulary at
its ownership seam rather than split unrelated responsibilities.

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
finite **clip-to-surface affine transform**. This permits later ancestor clip
chains without requiring all clips to share the item's local space. Paint and
hit consumers evaluate only transforms contained in the immutable product.

All scene geometry remains logical. `RasterScale` never alters this coordinate
contract.

### Paint clip composition is exact

Paint clips are conjunctive logical coverage constraints, not an ordered command
stack. An item with zero clips is unrestricted by clipping. With one or more
clips, a surface-logical sample contributes paint only when it is inside **every**
clip after mapping through each clip's own inverse `clip-to-surface` transform.
Multiple clips therefore form their geometric intersection; clip order cannot
change the result and no consumer may choose union, first/last-wins, or implicit
replacement semantics.

Paint primitive coverage is evaluated through the item's exact
`local-to-surface` transform. A non-invertible item transform produces no paint
coverage and is diagnosed rather than falling back to untransformed geometry. A
non-invertible clip transform excludes coverage for that item and is diagnosed;
it never makes the clip disappear. Rectangle and rounded-rectangle clips use the
same logical shape rules frozen below. Raster edge sampling and anti-aliasing may
remain backend-specific, but they cannot change these logical inclusion,
intersection, or singular-transform semantics.

### Rounded-rectangle geometry has one normalization rule

M6 rounded-rectangle hit/clip geometry uses four finite non-negative **circular**
corner radii: top-left, top-right, bottom-right, and bottom-left. Elliptical
per-axis radii are not part of M6. The outer rectangle uses the existing
`LogicalRect` half-open edge contract: left/top are inclusive and right/bottom
are exclusive.

Before any rounded-rectangle clip or hit test, runtime/consumers normalize all
four radii by one common factor. Starting with `f = 1`, take the minimum of `1`
and each applicable ratio below whose denominator is positive:

```text
width  / (top_left + top_right)
width  / (bottom_left + bottom_right)
height / (top_left + bottom_left)
height / (top_right + bottom_right)
```

Each effective radius is `radius * f`. This preserves corner proportions while
ensuring adjacent radii fit the rectangle. A zero-sized outer rectangle is empty
under the inherited half-open rectangle contract. Within a normalized corner,
arc containment includes the circular boundary (`distance² <= radius²`) after
the outer half-open rectangle check. Clips and hit regions use the same
normalized-shape semantics; no backend or storage representation may choose a
different oversized-radius policy.

### Paint contributions are owner-local immutable fragments

The production paint hook replaces `WidgetPaintProof` with immutable
`PaintContribution` evaluated from mounted widget state during paint work. Its
read-only context contains the owner's final local logical size and resolved
computed style. It receives no runtime arena, surface origin, semantic tree,
backend/GPU/host object, resource cache, prior publication, paint revision, or
raster-scale authority.

A contribution is an ordered list of self-contained items. Each contributed
item owns a primitive, owner-local transform, zero or more clips, opacity, and a
snapshot-local signed layer. Runtime composes it into the surface-space contract
above. Self-contained items avoid malformed push/pop command nesting.

M6's minimum primitive vocabulary is:

- filled logical rectangle;
- stroked logical rectangle with finite non-negative logical width;
- image resource reference in a logical rectangle;
- shaped-text-run resource reference at logical placement with an ordinary
  literal core `Color` foreground.

Production shaping and image decoding are not primitive semantics. Text/image
primitives reference resources supplied by another owner.

Image and shaped-run placement nevertheless have exact scene semantics. An image
primitive maps the complete normalized resource domain `(0, 0)..(1, 1)`
affinely onto its destination logical rectangle: normalized `(0, 0)` maps to the
rectangle origin and `(1, 1)` maps to its far corner. M6 has no implicit
contain/cover, crop, repeat/tile, nine-slice, or intrinsic-size fit mode; later
vocabulary must name such behavior explicitly rather than making it a renderer
default.

A shaped-text-run primitive maps the resource's own logical `(0, 0)` origin to
its finite authored logical placement point. Glyph positions and coverage
geometry already contained in that shaped resource remain relative to that
origin. The primitive's literal `Color` is the run's scene-owned monochrome
foreground and is independent of shaped-resource identity, so changing only
resolved foreground color changes paint scene content without requiring a new
shaped-run `ResourceRef`. The primitive performs no hidden baseline inference,
fit, scaling, reshaping, line breaking, or reflow. Item transforms may still
transform the result under the ordinary scene transform contract. Production
shaping and the contents of shaped-run resources remain M8/provider work; M6
already fixes the logical `ResourceRef` identity and live-domain lifetime
contract consumed by those later providers. Intrinsic multi-color glyph payloads
are not part of the minimum M6 shaped-run contract and require a later explicit
resource/primitive extension rather than silently changing foreground semantics.

All transforms must be finite. Opacity must be finite and in closed `[0, 1]`;
checked construction rejects values outside the contract rather than clamping or
silently normalizing them. Layer values are ordering facts only, never identity.

### Literal color, opacity, and basic rectangle stroke semantics are exact

M6 fill, stroke, and shaped-text-run literal colors reuse the existing core
`Color`; no duplicate scene color type is introduced for the same sRGB8 case. In
renderer-facing paint, its red/green/blue bytes are **unpremultiplied sRGB-encoded
8-bit channels** and its alpha byte is **linear coverage** in `[0, 255]`.
Consumers normalize alpha to `[0, 1]`, decode RGB to linear-light sRGB for
compositing, may premultiply internally, and composite scene items with ordinary
source-over semantics in the accepted scene order. The item's validated opacity
multiplies source alpha (and therefore premultiplied source color) before
source-over. M6 defines no alternate blend modes.

This color rule applies to literal scene colors. Resource payload color metadata
remains owned by the resource/provider contract for resource kinds that define
such metadata; it cannot override the literal foreground of the minimum M6
monochrome shaped-run primitive. A consumer cannot reinterpret a literal `Color`
according to backend defaults. Physical display color management, raster
sampling, and anti-aliasing implementation are backend concerns and are not
pixel-exact M6 scene semantics, but they must not change the scene's literal sRGB
color meaning or source-over ordering.

A filled rectangle with zero logical width or height produces no paint coverage.
A stroked logical rectangle's width is measured in the primitive's local logical
space before the item transform. Width `0` produces no stroke coverage; it is
never a device-dependent hairline. A stroked rectangle whose own logical width
or height is zero also produces no coverage.

For a non-empty rectangle and positive stroke width `w`, the exact logical stroke
is the axis-aligned **mitered ring** formed by the rectangle expanded by `w / 2`
on all four sides minus the rectangle inset by `w / 2` on all four sides. If the
inset collapses either dimension to zero or below, the inset is empty and the
stroke is the complete expanded rectangle. This fixes centered alignment and
square/mitered corners without introducing backend-selected round/bevel joins.
Raster edge sampling and anti-aliasing remain backend concerns, but consumers
must not silently choose inside-only, outside-only, rounded, beveled, or
hairline stroke geometry.

Global paint order is the stable ascending tuple:

```text
(layer, mounted logical preorder, contribution-local order)
```

Hash-map/storage/backend iteration cannot redefine it. Later M7 stacking policy
may select layer values but consumes this ordering contract.

### Hit contributions are independent from paint and semantics

Hit testing is not inferred from paint primitives and paint is not inferred from
hit testing. A widget may contribute zero or more owner-local `HitRegion` values
through a distinct hit hook. Its context contains the owner's final local logical
size and no surface or mounted-target authority.

Each contributed region contains:

- a logical rectangle or rounded-rectangle shape;
- an owner-local finite affine transform;
- zero or more explicit clips;
- a snapshot-local signed layer;
- one `PointerPolicy`.

`PointerPolicy` has exactly two outcomes:

- `Target` — the topmost containing eligible region resolves to its
  runtime-injected mounted owner;
- `Block` — the topmost containing eligible region terminates physical hit
  testing with no mounted target.

**Pass-through is represented canonically by not contributing a region.** There
is no `PassThrough` enum value because such a region would be observationally
identical to absence for target selection and would create two representations
of the same behavior.

The default downstream hit contribution is empty. Layout participation alone
never implies pointer targetability. Controls requiring direct physical pointer
targeting opt in. An ancestor without its own targetable region still
participates in the accepted capture/bubble route when a descendant is targeted.

Pointer participation is **not derived from semantic state**. Semantic
`disabled`, `hidden`, and `inert` do not automatically remove, block, or retarget
a physical hit region. An owning widget/runtime control policy may independently
change its hit contribution/policy and invalidate hit testing when its interaction
contract requires that behavior. This preserves routed behavior such as a
disabled control remaining physically targetable while later canonical
command/default eligibility suppresses activation.

M6 resolves the roadmap's physical hit visibility/inertness vocabulary without
adding redundant per-region booleans. In the final published hit scene, physical
visibility is region presence. The only observable point-selection outcomes are
`Target`, `Block`, or no containing region/pass-through. A physical policy that
hides or makes an owner (or, under a later explicit interaction contract, a
subtree) inert must lower that policy before publication by suppressing the
affected hit regions for pass-through behavior or by publishing `Block` where
physical occlusion is intended. Semantic `hidden`/`inert` state never performs
that lowering implicitly, and consumers never reinterpret semantic state as a
fourth hit outcome.

### Hit order is exact

Runtime composes hit regions using stable ascending:

```text
(layer, mounted logical preorder, contribution-local order)
```

The public `HitTestScene` stores that exact order. **Topmost** means reverse
traversal of it. Storage iteration, backend order, and current-tree traversal
cannot change the result after publication. Layer remains snapshot-local ordering
policy, not identity.

### The hit scene is the canonical displayed input snapshot

The public immutable `HitTestScene` associated with a successful
`SurfacePublication` is the exact scene retained by `SurfacePublicationState`
for displayed point targeting **and** checked resolved-target membership. Runtime
must not copy it into another rectangle/membership snapshot or re-run
`SurfaceFrame` hit testing.

The hit scene owns two distinct runtime-composed facts:

- ordered pointer-hit regions, used only to resolve a logical point;
- exact mounted-target membership for every mounted node published in that
  generation, used only to prove that a checked resolved target belonged to the
  exact named historical snapshot before inherited current mounted-target
  validation/routing proceeds.

These facts must not be inferred from each other. A mounted node may be a valid
member of the named snapshot while contributing no hit region. Conversely, the
presence of a hit region does not create a separate target lifetime or bypass
mounted identity validation. Region absence therefore means pointer
pass-through only; it does **not** remove historical snapshot membership or make
resolved surface ingress fail `TargetNotInSnapshot`.

The retained scene is the single storage owner of its runtime-issued
`SurfaceInputContext`; `SurfacePublication::input_context()` exposes that same
value through the hit scene rather than maintaining a second independently
stored context field. The retained generation ring stores cheap immutable handles
to these exact scenes.

`SurfaceInputContext::hit_test_generation()` remains the sole public displayed
input-scene generation. M6 adds no second hit-scene generation/target namespace.
Current and retained contexts resolve only against the scene they name;
retired/missing/foreign/mismatched contexts retain accepted M4 behavior.
Historical point resolution uses the named scene's regions. Historical checked
resolved-target ingress uses the named scene's exact mounted-target membership;
it never asks the current tree whether the target happened to exist in that old
snapshot.

`MountedNodeId` remains the canonical route target injected by runtime into
`Target` regions and retained in exact snapshot membership. Scene order,
resource IDs, authored IDs, semantic IDs, and primitive identities never
substitute for mounted target identity.

A successful surface publication may issue a fresh displayed hit generation even
when region and membership content are unchanged, preserving M4 publication
semantics; immutable region/membership storage may still be shared across
wrappers.

Paint has no independently targetable **scene generation**. Renderer update
identity is the surface-scoped `PaintRevision` owned by `PaintPublication`, not a
second input-like generation or target namespace.

### Directional focus geometry remains layout-owned

M6 removes the proof-era accidental coupling between pointer hit data and
current directional-focus geometry. Explicit pointer participation and focus
participation are independent facts.

The runtime's existing focus selection continues to use the latest **successfully
published** logical geometry for mounted focus candidates under the accepted M4
focus eligibility/scope/navigation rules. That geometry is a runtime-private
read-only projection of the retained layout phase (for example
`MountedNodeId + LogicalRect` in mounted logical order), not `HitTestScene`
regions and not paint geometry. A focusable widget with zero hit regions remains
fully eligible for keyboard/controller/programmatic focus according to M4.
Conversely, a pointer-targetable region does not become focusable merely because
it appears in `HitTestScene`.

No new public `FocusScene`, focus generation, or second layout authority is
introduced. #59's retained layout phase remains the storage owner; focus
selection may derive or cheaply expose the private current projection from that
same retained immutable product. Directional focus uses the current accepted
published layout, not a historical `SurfaceInputContext` generation.

The private proof `HitTestSnapshot` may be removed only after all of its old
consumer roles are migrated in the same accepted cutover sequence:

- point/current-and-retained surface resolution -> canonical `HitTestScene`
  regions;
- checked resolved-target historical membership -> canonical `HitTestScene`
  membership;
- current focus-navigation geometry -> retained layout-phase projection.

The full inherited M4 surface-context and directional-focus corpora and their
trace behavior must pass unchanged after this split. M6 does not redefine
scoring, scope, restoration, eligibility, resolved-target validation,
logical-focus-scroll policy, or historical no-retargeting behavior.

### Hit testing uses one exact algorithm

Runtime input and public deterministic consumers use the same `HitTestScene`
point-resolution semantics. For each region from topmost to bottommost:

1. map the surface-logical point through the inverse of the region's exact
   local-to-surface transform;
2. test the region shape in that local space, using the normalized rounded-
   rectangle rule above where applicable;
3. map the surface-logical point through the inverse of each clip's exact
   clip-to-surface transform and require it to be inside every clip shape using
   the same rectangle/rounded-rectangle containment rules;
4. when contained and unclipped, apply `PointerPolicy`.

A non-invertible region transform makes that region non-hittable with deterministic
diagnostics; it never falls back to untransformed geometry. A non-invertible
clip transform deterministically excludes the region for that clip and is
diagnosed; it never makes the clip disappear.

On a containing eligible region:

- `Target` returns the runtime-injected mounted owner;
- `Block` returns blocked/no-target and stops.

If no contributed region contains the point, physical hit testing returns no
target. That is the sole pass-through representation. This point algorithm never
uses snapshot membership as synthetic geometry.

This replaces both `SurfaceFrame::hit_test` authority and the private copied
rectangle resolver. Checked resolved-target validation uses the same retained
`HitTestScene` wrapper's membership rather than a second snapshot. A surviving
debug/layout snapshot must have a separate truthful purpose and no hit or
historical-membership authority.

### Hit invalidation is explicit

M6 adds public `WidgetInvalidation::HIT_TEST` and includes it in `ALL`.

- `HIT_TEST` invalidates widget hit contribution and dirties hit composition;
- `LAYOUT` implies hit work because context/surface placement may change;
- structural change rebuilds hit topology/order and exact displayed-snapshot
  membership;
- state changes to shape, existence, layer, clips, transform, or pointer policy
  independent of layout require explicit `HIT_TEST`;
- ordinary hit-contribution changes do not alter mounted snapshot membership;
- `PAINT` alone never changes hit testing;
- `SEMANTICS` alone never changes hit testing;
- `INTERACTION` does not silently imply widget hit invalidation. Runtime-owned
  interaction policy may schedule it only where a later accepted control
  contract explicitly says so.

### Paint invalidation is explicit

`PAINT` invalidates widget paint contribution. Layout and relevant resolved-style
changes schedule paint work because context/placement may change. Widget state
that changes visual output requires `PAINT`. Semantic-only work does not dirty
paint by implication.

The mounted `WidgetPaintProof` cache is removed during M6 cutover. Runtime may
cache/share immutable resolved paint fragments, but no proof paint capability
remains competing production authority.

### Retained publication uses immutable phase products

Issue #59 is the first implementation prerequisite after M6A0.

The one runtime publication authority stores retained phase products via cheap
immutable handles or equivalent persistent representation. Non-structural
planning starts from shared handles and allocates/replaces only dirty products.

It must guarantee:

- clean/focus-only/semantic-only publication does not deep-clone every retained
  renderer/layout/hit/paint/diagnostic vector;
- unchanged phase products are demonstrably reused;
- replacement ownership is explicit per dirty phase;
- deterministic output is storage-representation independent;
- the public publication derives from aligned phase products, never a second
  mutable cache;
- rejected/terminally failed plans preserve M5 zero-partial-commit semantics;
- semantic publication remains an independent sibling coordinated by the same
  final commit.

The internal smart-pointer/container type is not public protocol. Retained layout
storage used by focus navigation is part of this one phase-product authority; it
must not be recreated as a second focus cache merely to remove `HitTestSnapshot`.
The retained displayed-input phase similarly owns both its region storage and
exact generation membership; no second membership cache is introduced.

### Resource references are logical, self-disambiguating, live-domain-stable values

M6 resource references carry an explicit neutral kind, such as image or shaped
text run, plus one opaque provider-issued identity value. That opaque value
includes the issuing namespace as part of its equality/identity contract, so two
providers issuing the same local key do not collide.

A reference is not resource bytes, a provider object, GPU handle, font object,
native image, mounted/semantic identity, or scene-item identity. It is not a
scene-local index. Once a live resolver/provider domain issues a `ResourceRef`
that may enter an accepted M6 scene, reference equality is stable by value for
that resolver-domain/runtime lifetime and the reference continues to denote the
same immutable logical renderer content for that lifetime. Logical content
replacement therefore requires a new `ResourceRef` value. A live-domain ref does
not expire merely because a renderer drops a GPU/image/glyph realization or the
provider evicts a disposable realization cache; the resolver/provider must retain
or reconstruct the same logical content so a current complete paint snapshot can
be realized again without hidden prior renderer state.

For the minimum M6 shaped-text-run resource kind, that immutable logical content
is the shaped glyph/coverage geometry and its non-paint metadata. The primitive's
literal foreground `Color` is deliberately **not** part of the shaped resource
identity. A resolved foreground-only style change therefore reuses the same
shaped `ResourceRef` and changes ordinary `PaintScene` content instead of
requiring reshaping or provider mutation.

Consumers receive the **whole `ResourceRef`** and resolve it through one neutral
live resolver boundary; they do not split a local key from the value and
guess/select a provider. The resolver/provider owns bytes and backend realization
lifetime. M6 does not standardize storage layout, decoding, shaping, upload, or
backend realization caches, but it does standardize the logical identity
lifetime needed by complete snapshots: a ref does not silently retire while its
issuing resolver/runtime domain remains live. A later explicit lease/retirement
protocol may extend that rule; silent live-domain expiry is not M6 behavior.

Deterministic M6 proofs may use fixture resolvers/resources. M8/M10 later provide
production text/resource producers and realization behind this same reference
boundary. Unknown/foreign-domain or kind-mismatched refs are deterministic
consumer/admission errors, never reinterpretation as another primitive or
concrete widget kind. `Expired` is not a valid state for a ref issued by the
matching still-live M6 resolver domain.

### Raster scale has one neutral input authority

M6 introduces public runtime `RasterScale`, a validated finite strictly-positive
logical-to-raster scale value. `RasterScale::ONE` is the deterministic/headless
default. Invalid zero, negative, NaN, or infinite values are rejected by checked
construction and never enter publication state.

The existing public `SurfaceBuildContext` is the sole neutral M6 input boundary
for raster scale. M6C extends it with a `RasterScale` value/default and exposes
read-only access; the exact builder method name is API detail. The runtime copies
that accepted value into `PaintPublication`. Widgets, paint contributions,
`PaintScene`, and renderer consumers cannot mutate or override it.

This is not premature native-host integration. M6 deterministic callers can
publish the same surface at scale `1.0` and `2.0` through ordinary neutral
surface-build input. M10 later reads native/window DPI or scale policy and supplies
that result through the same `SurfaceBuildContext` boundary; native types never
enter the M6 scene or contribution vocabulary.

### Paint publication metadata and damage

Every `PaintPublication` is a complete renderer snapshot for one exact
`SurfaceId`. In addition to revision/base revision and `PaintScene`, it carries
metadata that is not part of the scene's history-independent content identity.

The publication contains the exact validated logical surface size. It is the
renderer consumer's target logical canvas extent and defines the full-surface
logical rectangle from origin `(0, 0)` to that size. A renderer must not consult
layout reports or mounted storage to discover the target extent. Surface size is
publication metadata because the same reusable scene content may be hosted by a
different accepted logical extent.

The publication also contains the exact accepted `RasterScale`. Scale is used
only for renderer realization; all layout/paint/hit/pointer geometry stays
logical. A scale change is renderer-relevant publication state but never changes
logical coordinate meaning.

Damage is a deterministic logical delta **from `base_revision` to the current
`PaintRevision`**. The first paint publication has no base and full-surface
damage. A logical surface-size or `RasterScale` change also requires full-surface
damage. For other renderer-state changes, damage must never under-report changed
renderer-relevant output; conservative full-surface damage is always permitted.
A newly allocated paint revision cannot use empty damage unless the accepted
renderer comparison proves the new tuple is visually unchanged under a future
explicit optimization; M6's required baseline may simply use conservative damage.
An unchanged renderer tuple creates no new paint revision/publication at all.

Because damage is predecessor-relative, it is publication metadata rather than a
field that changes `PaintScene` content equality. #59 may therefore reuse an
unchanged scene across distinct changed paint publications while computing fresh
revision/extent/scale/damage facts.

Scene requirements are derived from canonical `PaintScene` content. Consumer
capabilities are external input. Capability checking reports unsupported
requirements deterministically; it never makes core/runtime emit backend-specific
alternate scenes, silently lower primitives, or select a concrete renderer.

### Public observation uses ordinary immutable products

`runenui_testing` remains convenience authority only. It retains the latest
ordinary `SurfacePublication` and exposes/asserts public paint/hit products. It
does not fabricate paint revisions/base revisions, raster scale, scene IDs,
mounted targets, input generations, regions, snapshot membership, publication
state, damage history, or a second point/resolved-target algorithm.

The genuine downstream custom-widget package must migrate from
`WidgetPaintProof` through public contribution APIs. M6 requires two independent
deterministic scene consumers; at least one is a genuine external/custom
renderer consumer with no concrete widget-kind knowledge.

### Clean pre-1.0 migration

M6 preserves no proof-era renderer/hit authority through aliases. Slices remove
or truthfully narrow:

- `WidgetPaintProof` and its mounted capability cache;
- renderer-facing paint claims on `SurfaceFrame`/`SurfaceNode`;
- `SurfaceFrame::hit_test` / `hit_test_id` as hit authority;
- the private copied `HitTestSnapshot`/rectangle resolver once point targeting
  and exact historical resolved-target membership have moved to `HitTestScene`
  **and** current focus geometry has moved to the retained layout-phase
  projection;
- debug/test helpers independently reproducing renderer/hit or historical
  snapshot-membership semantics;
- stale docs/support claims naming proof products as production scene inputs.

`SurfaceLayoutReport`, style reports, mounted inspection, and diagnostics may
remain where they retain separate truthful ownership. No compatibility module,
deprecated wrapper, duplicate alias, or hidden parallel path is required before
1.0.

## Implementation and acceptance sequence

M6A0 freezes architecture/conformance only and owns no scene behavior.

After its architecture/conformance PR is owner-accepted, guarded-squash-merged,
and content identity is verified, perform one bounded M6A0 current-contract
reconciliation. The A0 package itself already registers ADR 0007 and the M6
matrix in the retained-document inventory. The reconciliation records the actual
accepted A0 squash, accepted discoverability/status, M6's conformance baseline,
umbrella/pickup state, and next exact base in roadmap/status/support/work-tracking
and other affected current-contract owners. It must itself be owner-accepted,
merged, and accepted-main validated. **No M6 implementation branch may start
before this reconciliation completes.**

Then the minimum implementation order is:

### M6A — persistent retained-publication substrate (#59)

Replace whole-`SurfaceCache` cloning with immutable shared phase products while
preserving M5 transaction/failure behavior. Prove narrow publications reuse
unchanged products. Preserve a single retained layout phase that can serve both
public layout reporting and the runtime-private current focus-geometry projection;
do not add a second focus cache. Preserve one retained displayed-input phase that
can serve both point-region lookup and exact historical mounted-target membership;
do not add a second membership cache. Do not add parallel cache or scene behavior
unless strictly required to prove the storage boundary.

### M6B — canonical paint/hit scene kernel and displayed-hit cutover

Introduce focused core contribution vocabulary, explicit hit invalidation,
public immutable `PaintPublication`/`PaintScene`/`HitTestScene`, runtime
placement/target injection, exact displayed-generation mounted-target membership,
surface-scoped non-wrapping paint revisions, basic rectangle primitive/region
composition, exact deterministic order, and the canonical retained hit-scene
ring. Migrate point resolution and checked historical resolved-target validation
to `HitTestScene`, and directional-focus geometry to the retained layout phase,
before removing the old mixed private snapshot. Migrate the built-in/downstream
vertical proof and remove `WidgetPaintProof`, duplicate private hit snapshots,
and old hit authority where replacements become live. Identity transforms, no
clips, opacity `1`, layer `0`, `Target`, `RasterScale::ONE`, and conservative full
damage suffice for this first kernel; M6C extends the same products rather than
creating another path.

### M6C — transforms, clips, resources, metadata, damage, and capabilities

Complete transformed/rounded/intersection-clipped composition and hit semantics,
exact image/shaped-run logical placement and shaped-run scene-owned literal
foreground color, immutable self-disambiguating live-domain-stable resource
references, exact paint revision/base consumer semantics, neutral
`SurfaceBuildContext` raster-scale input, logical extent/scale metadata, sound
incremental damage, `Block` policy, and consumer capability checking on the same
M6B products.

### M6D — independent consumers, migration, and milestone closure

Prove two independent deterministic consumers including one genuine downstream
renderer without widget-kind knowledge; complete public testing assertions;
remove remaining obsolete proof claims; run integrated M4/M5 inheritance plus M6
conformance, including unchanged surface-context and directional-focus corpora;
reconcile current authority and close M6.

A later critical audit may split a slice only when it reduces a real acceptance
boundary without duplicate authority. M6B cannot precede accepted M6A because
that would knowingly build real scenes around the cache #59 must replace.

## Rejected alternatives

### Keep `SurfaceFrame` and wrap it as `PaintScene`
Rejected: it contains mounted/widget/debug/style facts and proof paint.

### Keep a private hit snapshot beside a public hit scene
Rejected: two displayed-input representations can diverge. The public retained
scene owns both exact point-hit regions and exact historical mounted-target
membership. The independent focus-geometry consumer moves to the retained layout
phase rather than justifying a second hit or membership representation.

### Use `HitTestScene` as directional-focus geometry
Rejected: explicit pointer participation is not focus participation. A keyboard-
focusable widget may deliberately contribute no hit region, while a hit region
may belong to a non-focusable owner. M4 focus geometry therefore remains a
current retained-layout projection and does not acquire hit-scene identity or
retention semantics.

### Derive resolved-target snapshot membership from hit regions
Rejected: accepted M4 checked resolved-target ingress validates whether an exact
`MountedNodeId` belonged to the named historical snapshot independently of point
hit participation. A no-hit mounted node may still be a valid historical resolved
target. `HitTestScene` therefore retains exact generation membership as a
separate fact inside the same displayed-input product.

### Add an independent scene target/generation namespace
Rejected: `MountedNodeId` and `SurfaceInputContext` already own the necessary
route-target and displayed-generation lifetimes.

### Reuse `SurfaceInputContext` or hit generation as renderer revision
Rejected: input snapshot identity and renderer-change identity have different
advance rules and consumers. Coupling them would fabricate renderer updates for
semantic/hit-only surface publications and make renderer recovery depend on
input-retention semantics.

### Add `PaintSceneGeneration` instead of a paint publication revision
Rejected: `PaintScene` is reusable history-independent content. The versioned
thing is the renderer-facing surface snapshot containing scene plus extent/scale
and damage lineage, so revision belongs to `PaintPublication`.

### Let a renderer or widget set raster scale
Rejected: raster scale is a per-publication surface input, not widget paint state
or renderer feedback. `SurfaceBuildContext` already owns the neutral input seam;
M10 later supplies native scale through that same seam.

### Add an explicit pass-through hit policy
Rejected: not contributing a region already provides exactly that targeting
behavior. A public no-op variant would create duplicate representation.

### Add independent per-region visibility or inertness booleans
Rejected: the final physical hit scene already has one canonical representation
for each observable point-selection outcome: `Target`, `Block`, or omitted
region/pass-through. A separate visible/inert bit would create overlapping states
unless it merely re-encoded one of those outcomes. Semantic hidden/inert remains
an independent meaning/accessibility fact; explicit control/runtime policy lowers
physical hidden/inert behavior into the canonical hit representation before
publication.

### Derive hit testing from paint primitives
Rejected: visual coverage and interaction policy differ.

### Make every layout rectangle targetable by default
Rejected: layout participation does not imply pointer participation.

### Derive physical hit policy from semantic disabled/hidden/inert state
Rejected: semantics are an independent meaning/accessibility product, not
physical input authority. An owning control may intentionally align both products
but must contribute/invalidate each through its own authority.

### Let paint clips use consumer-selected composition semantics
Rejected: self-contained clips are public logical coverage constraints. Zero
clips mean no restriction, multiple clips intersect, and singular clip/item
transforms fail closed for coverage. Union, last-wins, stack-like replacement, or
singular-transform fallback would let independent consumers disagree on the same
scene.

### Leave image or shaped-run placement renderer-defined
Rejected: a resource reference does not authorize a consumer to invent fit or
origin semantics. Images map the complete normalized resource domain to the
named destination rectangle; shaped runs translate their own local origin to the
named logical placement. Decode, shaping, raster sampling, and realization remain
provider/backend work.

### Put resolved foreground color into shaped-resource identity
Rejected: resolved foreground is already a paint-relevant style fact and the
paint contribution receives resolved style but no resource/provider mutation
authority. Encoding foreground into shaped-resource identity would force a
paint-only style change to mint a new shaping resource or couple the shaping
provider to M7 paint style. The minimum M6 shaped-run primitive therefore owns an
ordinary literal `Color` foreground independently from reusable shaped geometry.

### Leave literal color encoding or stroke alignment backend-defined
Rejected: two independent renderer consumers could otherwise interpret the same
scene differently. M6 fixes literal `Color` as unpremultiplied sRGB8 plus linear
alpha, source-over compositing, item-opacity multiplication, centered positive
rectangle strokes as one mitered expanded-minus-inset ring, zero-area rectangle
no-coverage, and zero-width stroke no-op semantics. Backend rasterization remains
free only where it does not reinterpret those logical facts.

### Let oversized rounded radii use backend-specific clamping
Rejected: rounded clips and hit regions are public deterministic geometry. M6
normalizes all four circular radii by one proportional factor before containment,
so consumers cannot disagree on oversized corners.

### Put predecessor-relative damage inside `PaintScene` content
Rejected: identical scene content can have different damage after different
predecessors. That would make scene identity history-dependent and frustrate
persistent scene reuse.

### Store scene requirements as independent publication authority
Rejected: requirements are exactly derivable from `PaintScene`. A cached
materialization may exist internally but cannot become a separately mutable or
versioned product.

### Let a resource reference resolve mutable logical content
Rejected: paint-scene equality and revision reuse would become unsound if the same
logical reference could silently change renderer-relevant content. Content
replacement receives a new `ResourceRef`; only backend realization caches may
change behind a stable ref.

### Allow a live-domain resource reference to expire while current scenes still name it
Rejected: `PaintPublication` is a complete snapshot and must support full
realization after skipped revisions or renderer cache/device loss. Silent logical
expiry would make current scene content depend on hidden prior renderer state.
M6 therefore keeps issued logical refs immutable and resolvable for the live
resolver/runtime-domain lifetime; only disposable backend realizations may be
evicted. Any future bounded retirement/lease protocol requires an explicit
extension with its own lifetime/consumer rules.

### Add a giant renderer/widget trait or split the whole widget module first
Rejected: #10 remains a broad non-blocking concentration audit; M6 has focused
paint/hit seams.

### Generate backend-specific scenes from capability negotiation
Rejected: capabilities validate consumers of one canonical scene.

### Require minimal damage immediately
Rejected: sound conservative damage establishes the contract; precision is an
optimization.

### Add a separate M6 delivery charter immediately
Rejected as duplicate authority. ADR 0007 owns durable architecture, the M6
matrix owns observable acceptance, the roadmap owns milestone order, and GitHub
issues own volatile execution state.

## Consequences

Positive consequences:

- renderer/hit consumers receive explicit products rather than widget/debug
  proofs;
- scene coordinates/order are self-contained and deterministic;
- literal fill/stroke/shaped-run color/compositing, exact rectangle stroke
  geometry, paint clip intersection, resource placement, and rounded hit/clip
  geometry have one cross-consumer interpretation;
- shaped glyph geometry remains reusable across resolved foreground-only paint
  changes instead of making shaping/resource identity style-owned;
- point-hit participation, exact historical mounted-target membership, and
  directional-focus geometry remain distinct facts without duplicate live
  authorities;
- paint content identity remains reusable and history-independent;
- renderer consumers receive an explicit surface-scoped revision/base chain and
  can safely use damage after contiguous updates or recover from missed updates
  by realizing the complete snapshot;
- complete paint snapshots remain realizable after backend-cache/device loss
  because logical resource refs cannot silently expire inside the live resolver
  domain;
- semantic/hit-only surface publications do not fabricate renderer updates;
- renderer consumers receive explicit logical canvas extent and validated scale
  without reading layout authority or owning scale mutation;
- accepted M4 displayed-input, resolved-target, and focus-navigation authority
  remain intact and independent;
- runtime no longer needs two displayed-input representations;
- custom widgets gain explicit paint/hit contribution without registration;
- semantic, paint, hit, layout, diagnostics, and paint-publication metadata have
  distinct ownership;
- #59 is resolved before real scenes multiply retained-cache copying;
- resource/scale/damage/capability boundaries exist without a concrete backend.

Costs and constraints:

- M6 deliberately breaks proof paint/hit APIs before 1.0;
- downstream widgets must adopt explicit hit participation/new paint contribution;
- runtime gains one checked paint revision counter, consumed only by actual
  renderer-relevant snapshot changes;
- runtime gains one validated neutral raster-scale value in surface build input;
- runtime must retain exact displayed-generation mounted-target membership inside
  the same hit-scene product used for point regions;
- runtime focus selection must read retained layout geometry after the old mixed
  snapshot is removed;
- hit invalidation gains one public bit and proof burden;
- color conversion/compositing, shaped-run foreground independence,
  rectangle-stroke geometry, clip intersection, resource placement,
  rounded-radius normalization, coordinate/order, and transformed hit semantics
  require deterministic tests;
- fixture resource resolvers are required until M8/M10 production producers and
  must preserve issued logical refs for their live resolver-domain lifetime even
  if backend realization caches are dropped;
- retained immutable products must preserve simple staged atomicity.

## Acceptance

The normative observable requirements are recorded in
[`../architecture/m6-conformance-matrix.md`](../architecture/m6-conformance-matrix.md).
The M6A0 architecture/conformance merge does not by itself authorize M6A. M6
implementation remains blocked until the bounded A0 current-contract
reconciliation records the accepted squash and itself passes owner acceptance,
merge, content-identity, and accepted-main validation.
