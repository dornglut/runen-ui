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
> authority package containing it is explicitly accepted by the repository
> owner and merged. Its accepted status describes the intended state of that
> accepted squash; an unmerged branch never overrides accepted `main`.

## Context

M5 completed renderer-independent semantics and deterministic public testing.
M6 must now replace the remaining M2/M3 renderer and hit-testing proofs with a
backend-neutral scene protocol without weakening the mounted, input, semantic,
or publication contracts already accepted in M3–M5.

The accepted baseline deliberately does not already contain that protocol:

- `Widget::paint` returns `WidgetPaintProof`, a category/description proof value
  that ADR 0003 explicitly says is not a primitive scene;
- `SurfaceFrame` aligns mounted identity, logical bounds, widget type, proof
  paint, diagnostics, and computed style and exposes reverse-order rectangle
  hit testing;
- runtime pointer ingress does not use that public `SurfaceFrame::hit_test`
  method. `SurfacePublicationState` copies `(MountedNodeId, LogicalRect)` from
  the frame into a second private `HitTestSnapshot` ring and resolves displayed
  input there;
- ADR 0005 and the accepted M4 `SURFACE-*` rows already make
  `SurfaceInputContext` authoritative for one logical surface, coordinate
  revision, exact displayed hit-test generation, bounded historical retention,
  deterministic retirement, and no retargeting through current geometry;
- `WidgetInvalidation` has no hit-test-specific bit even though runtime
  `DirtyPhases` has `HIT_TEST`; ordinary hit work is currently implied by
  layout;
- non-structural publication planning clones the complete `SurfaceCache` before
  replacing dirty phase facts. Issue #59 records the resulting O(surface)
  copying on narrow publications;
- semantic publication is already an independently typed sibling product and
  must not be folded back into renderer or hit-test authority;
- the public testing harness retains only an ordinary immutable
  `SurfacePublication`, while the genuine downstream external-widget package
  directly implements the proof-level paint hook.

M6 therefore needs a scene protocol and a clean migration, not another wrapper
around the current frame/cache proof.

## Inherited authority this ADR does not supersede

This ADR refines renderer/hit publication only. It does not redefine:

- ADR 0004 mounted lifetime, `MountedNodeId`, reconciliation, lifecycle, or
  mounted-tree authority;
- ADR 0005 canonical event routing, `SurfaceId`, `SurfaceInputContext`, exact
  displayed-generation targeting, retained historical targeting, target
  validation, or no-retargeting behavior;
- ADR 0006 queue, effects, wake/redraw, trace, and transaction causality;
- the M4 conformance matrix, especially `SURFACE-*`;
- the M5 semantic contribution, semantic identity, semantic publication,
  semantic action, or testing contracts;
- M5's accepted staged surface transaction:
  `admit -> read-only/staged plan -> candidate-dependent final preflight -> commit`;
- layout ownership, which remains runtime-owned and intentionally proof-level
  until M7;
- production text shaping/editing/resource production, which remains M8 work;
- native host, accessibility, and renderer backends, which remain later work.

Where M6 depends on one of those contracts, its conformance matrix references the
existing accepted observation instead of duplicating it.

## Decision

### One publication authority, distinct sibling products

`runenui_runtime` continues to own one live surface-publication authority. A
successful publication atomically aligns distinct immutable products:

- `PaintScene` — renderer-neutral visual primitives and renderer metadata;
- `HitTestScene` — input-facing shapes, clips/transforms, pointer policy, order,
  and runtime-injected mounted targets;
- layout result/report;
- semantic publication;
- semantic diagnostics and ordinary diagnostics;
- the exact displayed `SurfaceInputContext` owned by the hit scene.

These products may share internal immutable phase data but are not one mixed
node model. Paint consumers do not receive semantic roles, `WidgetTypeId`,
concrete control kinds, mutable mounted state, or backend handles. Semantic
consumers do not receive paint primitives. Layout and diagnostics remain
inspectable without becoming renderer input.

`SurfacePublication` is the public alignment boundary. It exposes the sibling
products read-only. Extracting one product never transfers live runtime
authority.

### Core owns contribution vocabulary; runtime owns composed scenes

The dependency direction remains `runenui_runtime -> runenui_core`.

`runenui_core` owns the public host- and renderer-neutral contribution
vocabulary used by downstream widgets. The intended focused ownership is:

- paint contribution types in a paint-focused core module;
- hit-test contribution types in a hit-focused core module;
- the existing `Widget<Action>` protocol retains widget-owned contribution
  hooks;
- no broad `Element`/`Widget` source reorganization is required merely because
  proof capabilities are replaced.

`runenui_runtime` owns scene composition, mounted-target injection, surface
coordinates, deterministic global order, retained displayed scenes, dirty phase
planning, publication transaction state, and public immutable scene snapshots.
A widget cannot author or forge `MountedNodeId`, `SurfaceId`,
`SurfaceInputContext`, scene order positions, or live publication generations.

This follows the accepted M5 precedent: move production vocabulary to a focused
module while keeping the widget contribution seam, rather than creating a giant
future-complete widget trait or a second registry.

### Paint contributions are owner-local immutable fragments

The production paint hook replaces `WidgetPaintProof` with an immutable
`PaintContribution` evaluated from mounted widget state during paint work.
Paint contribution receives a read-only publication context containing only
facts required to author owner-local visual output, including the owner's final
logical size and resolved computed style. It receives no runtime arena,
absolute surface coordinate authority, semantic tree, backend, GPU, host, or
resource-cache handle.

A paint contribution is an ordered list of self-contained items. Each item owns
its primitive plus its owner-local transform, zero or more clips, opacity, and a
snapshot-local signed layer value. Self-contained items are preferred over
unbalanced push/pop command stacks: malformed nesting is impossible and one
item can be validated independently.

M6's minimum primitive vocabulary is:

- filled logical rectangle;
- stroked logical rectangle with finite non-negative logical width;
- image resource reference in a logical rectangle;
- shaped-text-run resource reference at logical placement.

A primitive may be extended only for a demonstrated renderer-neutral consumer.
Production shaping and image decoding are not primitive semantics. Text and
image primitives reference resources produced/resolved by later or external
resource owners.

Transforms are finite 2D affine logical transforms. Clips are explicit logical
rectangles or rounded rectangles. Opacity is finite and clamped by validated
construction to the closed `[0, 1]` interval. Layer values are snapshot-local
ordering facts, never identity.

Global paint order is deterministic. The runtime orders items by layer, mounted
logical preorder, and contribution-local order, with stable ordering for equal
keys. No hash-map/storage iteration or backend sorting may redefine the scene.
Later M7 stacking policy may author layer values, but it consumes this protocol
rather than replacing its deterministic ordering rule.

### Hit-test contributions are independent from paint

Hit testing is not inferred from paint primitives and paint is not inferred from
hit testing. A widget may contribute zero or more owner-local `HitRegion`
values through a distinct hit-test contribution hook.

The hit-test contribution context contains the owner's final local logical size
and no absolute surface or mounted-target authority. Each region contains:

- a rectangle or rounded-rectangle logical shape;
- a finite owner-local affine transform;
- zero or more explicit clips;
- one snapshot-local layer value;
- one `PointerPolicy`.

`PointerPolicy` has three semantic outcomes:

- `Target` — the first topmost containing region resolves to the region's
  runtime-injected mounted owner;
- `Block` — the first topmost containing region terminates physical hit testing
  with no mounted target;
- `PassThrough` — the region is ignored for targeting and lower regions remain
  eligible.

Invisible regions are absent from the composed hit scene. Input-inert behavior
is represented by `Block` or absence according to the widget/control contract;
it is not derived from semantic state and therefore does not make semantics an
input dependency.

The default downstream widget hit contribution is empty. Generic layout boxes
are not automatically interactive merely because they occupy geometry. Built-in
and downstream controls that require direct pointer targeting opt in explicitly.
An ancestor with no targetable region still participates in capture/bubble when
a descendant is the mounted target through the existing mounted route.

### The hit scene becomes the canonical displayed input snapshot

M6 removes the final duplicate hit authority.

The public immutable `HitTestScene` associated with a successful
`SurfacePublication` is the exact scene retained by `SurfacePublicationState`
for pointer targeting. Runtime must not copy it into a separate rectangle-only
snapshot or re-run `SurfaceFrame` hit testing.

A retained scene stores the exact runtime-issued `SurfaceInputContext` for that
displayed publication plus immutable region data. `SurfacePublication::input_context`
therefore names the same context as its hit scene. The retained generation ring
stores cheap immutable handles to these exact scenes.

`SurfaceInputContext::hit_test_generation()` remains the one public displayed
hit-scene generation. M6 does **not** add a second hit-scene generation or a
second target namespace. Current and retained contexts continue to resolve only
against the exact scene named by that context; retired/missing/foreign/mismatched
contexts retain the accepted M4 behavior.

`MountedNodeId` remains the canonical route target injected by runtime into
`Target` regions. Scene item position, resource key, authored ID, semantic ID,
or primitive identity never substitutes for mounted target identity.

A successful surface publication may issue a fresh displayed hit generation
even when region content is unchanged, preserving accepted M4 publication
semantics. The immutable region storage may nevertheless be shared across those
scene wrappers.

Paint has no independently targetable generation because no accepted consumer
requires one. Paint snapshot identity is the enclosing `SurfacePublication` and
its immutable value. Adding another counter solely for symmetry is rejected.

### Hit testing uses one exact algorithm

`HitTestScene` owns the deterministic public/runtime hit algorithm. Runtime input
and public deterministic consumers use the same scene semantics.

Regions are considered from topmost to bottommost according to the scene's
stable order. A point is mapped through the exact finite transform contract,
then tested against every active clip and the region shape. Non-invertible
transforms make that region non-hittable and produce deterministic diagnostic
coverage; they never fall back to untransformed geometry.

On the first containing region:

- `Target` returns the runtime-injected mounted owner;
- `Block` returns a blocked/no-target result and stops;
- `PassThrough` continues to lower regions.

This replaces both proof-level `SurfaceFrame::hit_test` authority and the private
copied rectangle resolver. A debug/layout snapshot may remain only if it has a
separate truthful purpose and no longer claims hit authority.

### Hit invalidation is explicit

M6 adds public `WidgetInvalidation::HIT_TEST` and includes it in `ALL`.

The exact dependency rules are:

- `HIT_TEST` invalidates the widget's hit contribution and dirties hit-scene
  composition;
- `LAYOUT` implies hit-test work because contribution context and surface
  placement may change;
- structural changes rebuild hit topology/order;
- a widget whose state changes hit shape or pointer policy independently of
  layout must request `HIT_TEST` explicitly;
- `PAINT` alone never changes hit testing;
- `SEMANTICS` alone never changes hit testing;
- `INTERACTION` does not silently imply widget hit invalidation. Runtime-owned
  interaction policy may schedule hit work only where that policy is explicitly
  documented by a later accepted control contract.

This keeps hit policy independently observable without overloading paint or
semantic invalidation.

### Paint invalidation remains explicit and gains resolved context dependencies

`PAINT` invalidates widget paint contribution. Layout and relevant resolved-style
changes schedule paint work because the paint contribution context or placement
changed. A widget whose own state changes visual output must request `PAINT`.
Semantic-only work cannot make paint dirty by implication.

The old mounted cache of `WidgetPaintProof` is removed during the M6 scene
cutover. Runtime may cache or share resolved paint fragments/scene phase products,
but there is no second proof paint capability retained as production authority.

### Retained publication uses immutable phase products

Issue #59 is adopted as the first implementation prerequisite after M6A0.

The one runtime surface-publication authority stores each retained phase product
through a cheap immutable handle or equivalent persistent representation.
Non-structural planning starts from a candidate of shared handles and allocates
or replaces only products whose dirty dependencies require new values.

The design must guarantee:

- clean/focus-only/semantic-only publication does not deep-clone every retained
  renderer/layout/hit/paint/diagnostic vector;
- unchanged phase products are demonstrably reused;
- replacement ownership is explicit per dirty phase;
- deterministic order is independent of storage representation;
- the composed public publication is derived from those aligned phase products,
  never a second mutable cache;
- rejected or terminally failed plans retain the accepted M5 zero-partial-commit
  semantics;
- semantic publication remains an independent sibling state coordinated by the
  same final surface commit.

The exact internal smart-pointer/container type is not public protocol. A new
cache abstraction layer is not justified if ordinary immutable shared values
satisfy these requirements.

### Resource references are logical, not backend handles

M6 introduces renderer-neutral paint resource references with an explicit
resource kind such as image or shaped text run. A resource reference is a
logical immutable key/reference supplied by a higher-level resource owner; it is
not a GPU descriptor, texture handle, font database object, native image, or
mounted identity.

The scene protocol specifies reference identity and required kind only. It does
not load, decode, shape, upload, evict, or cache resource bytes. Deterministic
M6 consumers may resolve fixture resources from ordinary maps/providers. M8/M10
later provide production text/resource producers and backend realization behind
this same reference boundary.

Missing or kind-mismatched resources are consumer/admission diagnostics; they do
not authorize a backend to reinterpret the primitive as another widget kind.

### Scale is renderer metadata, not logical-coordinate authority

M6 scene metadata may carry one validated positive finite raster scale. All
layout, paint geometry, clips, transforms, hit shapes, and pointer ingress remain
in RunenUI logical coordinates. Scale informs a renderer consumer and damage
realization only; it does not introduce physical-pixel/DPI/native-window types
into core/runtime input protocols and does not replace `SurfaceInputContext`
coordinate authority.

The deterministic headless default is `1.0`. Production host ownership of scale
changes belongs to M10.

### Damage is conservative and sound, not prematurely minimal

`PaintScene` carries deterministic logical damage facts for the current paint
publication. Damage must never under-report changed visual output. A full-surface
damage rectangle is valid whenever finer invalidation cannot be proven. Empty
damage is valid only when the paint scene and renderer-relevant metadata are
unchanged under the accepted comparison contract.

M6 does not require an optimal damage algorithm. M7/M10 may improve precision
without changing the soundness contract.

### Capabilities validate consumers; they do not rewrite the canonical scene

M6 defines renderer-neutral scene requirements/capabilities sufficient for a
consumer to declare which accepted primitive/resource features it can realize.
The runtime derives requirements from the canonical paint scene. Capability
checking reports unsupported requirements deterministically.

Capabilities do not cause core/runtime to generate backend-specific alternate
scenes, silently lower one primitive into an unrelated approximation, or select
a concrete renderer. A consumer either accepts the canonical requirements or
reports what it cannot realize.

### Public observation uses ordinary immutable products

`runenui_testing` continues to own convenience only. It retains the latest
ordinary `SurfacePublication` and exposes/asserts paint and hit scenes through
public runtime APIs. It does not fabricate scene IDs, mounted targets,
generations, regions, or publication state and does not reproduce hit testing in
parallel.

The genuine downstream custom-widget package must migrate from
`WidgetPaintProof` to the accepted contribution protocol through public APIs.
M6 also requires two independent deterministic scene consumers; at least one is
a genuine downstream/custom renderer consumer that renders/interprets paint
without knowing concrete widget kinds such as `Button`.

### Clean pre-1.0 migration

M6 does not preserve proof-era renderer/hit authority behind aliases.

The implementation slices own clean removal or truthful narrowing of:

- `WidgetPaintProof` and its mounted capability cache;
- renderer-facing paint claims on `SurfaceFrame`/`SurfaceNode`;
- `SurfaceFrame::hit_test` / `hit_test_id` as hit authority;
- the private copied `HitTestSnapshot` representation once `HitTestScene` is
  canonical;
- debug/test helpers that independently reproduce renderer or hit semantics;
- stale docs/support claims naming proof products as production scene inputs.

`SurfaceLayoutReport`, style reports, mounted inspection, and diagnostics may
remain where they retain independent truthful ownership. They are not removed
merely because the mixed proof frame is retired.

No compatibility module, deprecated wrapper, duplicate type alias, or hidden
parallel path is required before 1.0.

## Implementation sequence

M6A0 freezes architecture and conformance only. It owns no scene behavior.
After A0 is owner-accepted, merged, content-identity verified, and accepted-main
validated, the minimum implementation order is:

### M6A — persistent retained-publication substrate (#59)

Replace whole-`SurfaceCache` cloning with immutable shared phase products while
preserving accepted M5 transaction/failure behavior. Add performance/regression
proof that narrow publications reuse unchanged products. Do not add a parallel
cache or scene behavior in this slice unless required solely to prove the new
storage boundary.

### M6B — canonical paint/hit scene kernel and displayed-hit cutover

Introduce focused core contribution vocabulary, explicit hit invalidation,
public immutable `PaintScene`/`HitTestScene`, runtime target injection, basic
primitive/region composition, and the canonical retained hit-scene ring. Migrate
built-ins and genuine downstream widgets needed for the vertical proof. Remove
`WidgetPaintProof`, duplicate private hit snapshots, and proof-level hit
resolution authority in the same cutover where their replacement becomes live.

### M6C — transforms, clips, resources, metadata, damage, and capabilities

Complete the M6 scene vocabulary and exact composition/hit semantics, resource
reference boundary, raster-scale metadata, sound damage, and consumer capability
checking. Preserve deterministic ordering and neutral scene identity.

### M6D — independent consumers, migration, and milestone closure

Prove two independent deterministic consumers, including one genuine downstream
renderer with no concrete widget-kind knowledge; complete public testing scene
assertions; remove remaining obsolete renderer/hit proof claims; run integrated
M4/M5 inheritance and M6 conformance; reconcile current authority and close M6.

A later critical audit may split one of these slices only when doing so reduces a
real acceptance boundary without introducing duplicate authority. It may not
reorder M6B before the accepted M6A retained-publication substrate because that
would knowingly build new scene products around the cache architecture #59 is
required to replace.

## Rejected alternatives

### Keep `SurfaceFrame` and wrap it as `PaintScene`

Rejected. It contains mounted/widget/debug/style facts and proof paint rather
than renderer primitives, and would preserve the mixed M2/M3 authority M6 is
supposed to retire.

### Keep a private hit snapshot beside a public hit scene

Rejected. Two representations can diverge and would force conformance to prove
which one input actually uses. The public immutable scene retained by runtime is
the one hit authority.

### Add an independent scene target/generation namespace

Rejected. `MountedNodeId` and `SurfaceInputContext` already own the required
route-target and displayed-generation lifetimes. Another namespace would create
synchronization and stale-target ambiguity without a distinct consumer.

### Derive hit testing from paint primitives

Rejected. Visual coverage and interaction policy differ; transparent or
non-visual controls, overlays, pass-through regions, and future styling make the
coupling incorrect.

### Make every layout rectangle targetable by default

Rejected. Layout participation does not imply pointer targetability. Explicit
hit contribution keeps interaction intent local and prevents generic containers
or text from becoming accidental targets.

### Add a giant renderer/widget trait or split the entire widget module first

Rejected. #10 remains a broad concentration audit, but M6 has focused paint/hit
ownership seams. A whole-protocol refactor would mix unrelated lifecycle,
authoring, event, semantic, and layout responsibilities into this milestone.

### Generate backend-specific scenes from capability negotiation

Rejected. It makes the backend influence canonical framework semantics. M6
capabilities validate consumers of one scene instead.

### Require minimal damage from the first implementation

Rejected. Sound conservative damage is sufficient to establish the protocol;
precision is an optimization once real consumers exist.

### Add a separate M6 delivery charter immediately

Rejected as duplicate authority. ADR 0007 owns durable scene/publication
architecture, the M6 conformance matrix owns observable acceptance, the roadmap
owns milestone scope/order, and GitHub issues own volatile execution state. A
new charter is justified only if a later audit identifies a durable delivery
contract that none of those owners can express without duplication.

## Consequences

Positive consequences:

- renderer and hit consumers receive explicit products rather than widget/debug
  proofs;
- accepted M4 displayed-input identity remains intact;
- the runtime no longer needs two hit-test representations;
- custom widgets gain explicit paint and hit contribution without registration;
- semantic, paint, hit, layout, and diagnostics remain independently owned;
- #59 is resolved before real scenes multiply retained-cache copying;
- resource, scale, damage, and capability boundaries are defined without adding
  a concrete backend;
- M7–M10 can consume stable neutral products rather than reopen scene ownership.

Costs and constraints:

- M6 is a deliberate breaking pre-1.0 migration for proof paint/hit APIs;
- downstream custom widgets must adopt explicit hit participation and new paint
  contributions;
- hit invalidation gains one public bit and corresponding proof burden;
- scene composition and exact transformed hit testing require new deterministic
  validation;
- resource references are intentionally unresolved by M6 and need deterministic
  fixture consumers until M8/M10 production providers exist;
- retained immutable products must be designed so atomic planning stays simple,
  not hidden behind rollback mutation.

## Acceptance

The normative observable requirements are recorded in
[`../architecture/m6-conformance-matrix.md`](../architecture/m6-conformance-matrix.md).
M6 implementation remains blocked until this ADR and that matrix are accepted as
one M6A0 authority package, merged from the exact accepted M5 base, and the
resulting accepted main is independently validated under repository policy.
