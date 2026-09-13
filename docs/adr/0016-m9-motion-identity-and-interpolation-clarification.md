# ADR 0016: M9 deterministic motion identity and interpolation clarification

> **Category:** ADR
>
> **Status:** Accepted target amendment on exact-head owner acceptance
>
> **Decision date:** 2026-09-13
>
> **Milestone:** M9
>
> **Reviewed baseline:** `aa80800477dd62eb8f1da8da2f5af6e5dc3ebec8`
>
> **Owner:** #201
>
> **Acceptance:** this amendment becomes accepted only after the exact M9B0R
> package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance clarifies target
> architecture only; it does not claim M9B production implementation or promote
> any `M9MOTION-*` conformance row.

## Context

ADR 0010 establishes one runtime-owned deterministic motion model: style transitions
and declarative explicit timelines sample one accepted monotonic instant per staged
surface candidate, use typed derived overrides rather than authored-state mutation,
feed the existing style/layout/text/publication authorities, obey mandatory preference
policy, and commit atomically with the products they affect. ADR 0012 additionally
freezes motion-sensitive node-effect group lifetime.

The source-first audit after accepted M9A closure found that those ownership rules are
sound but do not yet uniquely determine several observable M9B behaviors. Production
code would otherwise have to choose conventions for duplicate timeline identities,
multiple explicit timelines targeting one property, completed declarative timeline
reconciliation, transition-policy precedence, the exact initial target set,
composite-value compatibility, scalar/color/rotation interpolation, layout-domain
membership, cubic-Bezier inversion, and reduced-motion strategy validity.

Those choices affect public lifecycle, samples, invalidation, diagnostics and trace.
They therefore belong to accepted RunenUI architecture rather than an implementation,
renderer, dependency, floating-point shortcut, or iteration-order convention.

## Relationship to accepted authority

This ADR narrowly amends ADR 0010 for deterministic motion only. It does not change:

- M3 mounted identity or generation lifecycle;
- M4 monotonic time, wake/redraw, timer, bounded scheduling or canonical trace
  ownership;
- M6 publication/revision/resource identity rules;
- M8 style precedence, inheritance, layout/text authority or explicit preference
  ownership;
- ADR 0012 composition-group ordering, node-effect scope or group-lifetime rules;
- accepted M9A visual geometry, brush, image, composition, shadow or renderer
  realization authority.

ADR 0010 remains authoritative for transition-versus-explicit-timeline precedence,
keyframe/repeat boundaries, interruption sampling, staged publication, retained
renderer retry, invalidation/cache discipline, discrete-property boundary behavior and
completion without an application action. This amendment supplies the missing
normalization and compatibility rules.

For reduced-motion defaults, this ADR explicitly supersedes ADR 0010's ambiguous
phrase `repeating/forever decorative timelines`. A timeline with an explicit finite
iteration count remains a **finite timeline**, including counts greater than one, and
therefore defaults to `SnapToEnd`. Only `Repeat::Forever` defaults to `HoldInitial`.
A finite timeline may still explicitly select `HoldInitial` as allowed below.

## Decision

### Explicit timelines have one unambiguous owner and target

One explicit timeline describes exactly one typed `MotionTarget`.

`AnimationId` is owner-local authored identity. Within one mounted owner's accepted
explicit-timeline description set:

- every `AnimationId` must be unique; and
- at most one explicit timeline may address a given `MotionTarget`.

A duplicate ID or two different IDs targeting the same property is invalid authored
motion. The staged candidate rejects with canonical diagnostics before motion or
publication mutation. There is no first-wins, last-wins, authored-order arbitration or
additive same-property timeline composition.

Exact mounted generation plus `AnimationId` identifies the authored declaration.
Exact target plus exact timeline specification determine whether that declaration is
compatible with retained runtime state.

### Declarative completion remains reconciled rather than restarting accidentally

An unchanged accepted explicit-timeline declaration preserves its lifecycle state
across transient tree rebuilds and later publications. That includes a terminal
**completed declaration state** after natural completion or reduced-motion
`SnapToEnd`.

Completed declaration state:

- owns no active sampled-property authority;
- requests no continuous redraw;
- exists only so the same still-authored finite timeline does not restart merely
  because its active sample record was cleaned up.

Accepted removal retires active, suppressed or completed declaration state. Re-adding
the declaration after an accepted absence starts a new timeline.

Keeping the same `AnimationId` while changing target or exact specification is a
replacement at one candidate instant. Runtime orders old cancellation/replacement
before new start. A replacement explicit timeline starts from its own authored
keyframe `0`; runtime never rewrites authored keyframes to manufacture continuity.
The cancelled old target then returns toward its current governed style target under
ADR 0010's ordinary transition rule when applicable.

### Transition policy is part of the existing style cascade, not a second one

Transition policy is a non-inherited, target-keyed style policy. It is not itself a
visual/layout target and explicit timelines do not participate in the style cascade.

For each motion target, one style layer may:

- make no transition-policy contribution;
- explicitly disable transition for that target; or
- supply one validated `TransitionSpec`.

Normal M8 style-layer precedence resolves each target independently. An explicit
disable value masks lower-precedence transition rules; absence merely makes no
contribution.

The transition policy resolved in the same staged candidate as a newly observed
governed-target change governs that change. A policy-only change does not start motion
when the governed target is unchanged and no transition is active.

For an active style transition, the exact winning transition spec remains part of
compatibility. Changing that spec replaces the active transition from the current
sample toward the same governed target at the candidate instant. Changing to explicit
disable cancels the transition and presents the governed target in that same accepted
candidate.

### The initial M9 motion-target set is explicit and closed

Initial style motion targets are exactly:

- foreground;
- background brush;
- padding;
- corner radius;
- typography;
- ordered ordinary shadows;
- node opacity; and
- node presentation transform.

`outline` is not an initial M9 motion target. Its ordinary style changes remain
immediate. Later target expansion requires accepted architecture rather than generic
numeric or trait inference.

Initial structural `LayoutStyle` targets are exactly:

- width and height;
- min-width and min-height;
- max-width and max-height;
- margin;
- gap;
- flex grow and flex shrink; and
- flex basis.

Grid track topology/lists, grid placement, container algorithm, flex
direction/wrap/alignment, flow-versus-absolute positioning/insets, overflow policy and
all other structural layout fields are not initial M9 motion targets.

### Optional style values remain explicit domains

For optional computed-style targets, absence is an incompatible/discrete endpoint.
M9B does not silently reinterpret missing foreground/background/padding/radius or
presentation as transparent, zero or identity.

In particular, `None <-> Some(PresentationTransform)` is discrete. Smooth transform
entry/exit requires explicit identity-transform endpoints with an explicitly authored
origin; runtime does not invent a pivot.

### Continuous scalar interpolation has one overflow-safe rule

M9 never derives animatability from `Lerp`, arithmetic traits, field reflection or
backend capabilities. Continuous numeric domains use one framework-owned scalar rule
unless a property-specific rule below says otherwise.

For finite `f32` endpoints `a` and `b` and eased progress `t` in `[0,1]`:

1. `t == 0` returns endpoint `a` exactly and `t == 1` returns endpoint `b` exactly;
2. otherwise promote `a`, `b`, and `t` to `f64`;
3. compute `left = (1 - t) * a`, then `right = t * b`, then `sample = left + right`
   in that order using ordinary non-fused arithmetic;
4. do not use `a + (b - a) * t`, `mul_add`, FMA contraction, or an intermediate
   `f32` difference/product as semantic authority;
5. convert the finite convex result once to `f32` and validate/normalize it through
   the owning RunenUI value type.

This rule avoids false overflow for opposite-sign finite endpoints and keeps exact
endpoint identity. A property whose accepted domain cannot represent a resulting
sample must fail staged planning rather than clamp through renderer/backend policy.

### Continuous interpolation is RunenUI-owned and type-specific

General motion color interpolation uses the same renderer-neutral color-math family as
accepted M9 gradients: straight-alpha sRGB8 endpoints decode to linear color,
interpolation occurs in premultiplied linear-sRGB with linear alpha, and the result is
deterministically converted back to straight-alpha sRGB8. Renderer color math is not
authority.

The initial continuous rules are:

- `Color`: the color rule above;
- `SceneOpacity`: the canonical scalar rule over the accepted unit interval;
- padding `EdgeInsets`: canonical scalar interpolation per logical-length component;
- corner `Radius`: canonical scalar interpolation per logical-length component;
- `PresentationTransform`: when both endpoints are present, canonical scalar
  interpolation of translation x/y, scale x/y, raw authored rotation radians and
  normalized origin x/y. Rotation uses the authored scalar directly: no modulo
  normalization, shortest-arc choice, matrix decomposition or backend transform
  interpolation;
- ordinary-shadow lists: continuous only when list lengths are equal. Pair shadows by
  authored index and apply canonical scalar interpolation to offset x/y, non-negative
  sigma and signed spread, plus the accepted motion color rule. Different lengths are
  discrete; there is no geometric/color matching or insertion/removal heuristic.

Typography is one discrete target in initial M9. Family fallback, size, weight, width,
style/oblique angle, variation axes, OpenType features and shaping identity switch
together at the accepted discrete boundary. Numeric representation inside typography
does not create font-size, weight, width or variation-axis interpolation authority.

### Brush compatibility preserves accepted gradient structure

Brush interpolation is continuous only under these exact compatibility rules:

- solid -> solid: interpolate color;
- linear -> linear: primitive-local start/end geometry must be exactly equal, stop
  count equal, and the exact authored stop-offset sequence—including duplicate
  hard-stop offsets/order—equal; interpolate stop colors only;
- radial -> radial: center/radius geometry must be exactly equal and exact stop
  count/offset sequence equal; interpolate stop colors only.

Every brush-kind, gradient-geometry, stop-count, stop-offset/order or hard-stop
structure change is discrete. M9 therefore does not synthesize gradient-geometry
morphing or risk an invalid intermediate linear-gradient geometry.

### Accepted numeric layout domains are exact

For the initial structural layout targets:

- `LayoutDimension`: `Length <-> Length` and `Percent <-> Percent` are continuous;
  every other endpoint pair is discrete;
- `LayoutBound`: `Length <-> Length` and `Percent <-> Percent` are continuous; every
  other pair is discrete;
- `FlexBasis`: `Length <-> Length` and `Percent <-> Percent` are continuous; every
  other pair is discrete;
- margin and gap logical lengths interpolate component-wise using the canonical scalar
  rule; and
- flex grow/shrink `LayoutFactor` values use the canonical scalar rule.

`Auto`, `Fill`, `MinContent`, `MaxContent`, `Content` and cross-domain numeric pairs do
not acquire implicit conversion/interpolation semantics. Sampled values still feed the
single accepted runtime/Taffy/text path and existing constraint semantics.

### Cubic-Bezier timing has one deterministic inversion algorithm

A cubic-Bezier timing curve has endpoints `(0,0)` and `(1,1)` plus accepted control
points `(x1,y1)` and `(x2,y2)` from ADR 0010.

Bezier coordinate evaluation itself is fixed. For scalar endpoints/control points
`p0..p3` and parameter `u`, evaluate in `f64` with the following explicitly ordered
de Casteljau steps, using ordinary `*`, `+`, and `-` only and no `mul_add`/FMA:

```text
v = 1 - u
q0 = v * p0 + u * p1
q1 = v * p1 + u * p2
q2 = v * p2 + u * p3
r0 = v * q0 + u * q1
r1 = v * q1 + u * q2
value = v * r0 + u * r1
```

Normalized progress `0` and `1` return exact easing endpoints. For progress `p` strictly
inside `(0,1)`, RunenUI computes the curve parameter deterministically in `f64`:

1. initialize `lo = 0` and `hi = 1`;
2. repeat exactly **32** times:
   - `mid = (lo + hi) / 2`;
   - evaluate the cubic x coordinate at `mid` using the fixed de Casteljau procedure;
   - if `x(mid) < p`, assign `lo = mid`; otherwise assign `hi = mid`;
3. set `u = (lo + hi) / 2`;
4. evaluate y at `u` with the same fixed procedure and clamp the result to `[0,1]`.

Named easing conveniences are only aliases for accepted control-point tuples. No
Newton iteration count, dependency solver, convergence tolerance, alternate polynomial
form, FMA contraction, platform timer or frame history may alter one logical easing
sample.

### Reduced-motion strategies are validated by source kind

`StylePreferences::reduced_motion` remains the sole reduced-motion input.

- Ordinary style transitions default to `SnapToEnd`, cannot use `HoldInitial`, and may
  explicitly use `PreserveEssential`.
- Any finite explicit timeline—including a finite repeat count greater than one—
  defaults to `SnapToEnd` and may explicitly use `HoldInitial` or
  `PreserveEssential`.
- `Repeat::Forever` cannot use `SnapToEnd`; its ordinary decorative default is
  `HoldInitial`. Essential forever motion must explicitly use `PreserveEssential`.
- `SnapToEnd` commits the finite source's terminal value plus terminal/completed
  lifecycle state atomically.
- `HoldInitial` retains a suppressed explicit-timeline declaration at keyframe `0`,
  owns no continuous redraw and accrues no hidden elapsed time. If reduced motion
  becomes false while the same declaration remains authored, it starts/restarts from
  that preference-change candidate instant.
- `PreserveEssential` is valid for either motion source and samples normally.

Invalid source/strategy combinations reject during description/spec validation rather
than receiving runtime fallback.

Mandatory non-reduced preference suppression, such as a currently governing
high-contrast override for the target property, masks the conflicting sampled value
and any motion-specific group requirement but does not pause or restart underlying
motion time. Lifecycle may progress and complete while suppressed. If suppression
later lifts while the motion remains active, the current same-clock sample becomes
visible. Only the reduced-motion rules above have hold/restart semantics.

### Existing atomic publication and trace authority remain unchanged

All normalization, collision checks, target/spec reconciliation, easing, preference
decisions and interpolation happen during staged surface planning against one candidate
instant. Invalid authored motion, checked-time failure, interpolation-domain failure,
trace-capacity/admission failure or downstream publication failure cannot partially
start/replace/complete motion.

Motion state, completed declaration state, sampled effective values, node-effect group
existence and dependent products commit through the existing runtime transaction
boundary. Renderer failure never advances motion; retained retry reuses the exact
already-sampled publication.

Canonical runtime diagnostics/trace record collisions, policy resolution,
start/replacement/cancellation/completion, completed-declaration retention, reduced
motion/suppression, easing/interpolation compatibility and cache/effect decisions.
There is no second animation log.

## Conformance impact

This amendment adds no M9 row and changes no row status.

It sharpens the existing M9B proof obligations, principally:

- `M9MOTION-02`: target-keyed non-inherited transition policy, explicit disable and
  active-spec replacement;
- `M9MOTION-03`: unique owner-local animation IDs, one explicit timeline per target,
  invalid collision rejection, completed declaration retention, remove/re-add and
  exact replacement lifecycle;
- `M9MOTION-04`: fixed deterministic cubic-Bezier inversion and coordinate evaluation
  in addition to existing keyframe/repeat/zero-duration rules;
- `M9MOTION-05`: the closed initial motion-target set plus exact optional, canonical
  scalar, color/brush/presentation/shadow/typography/layout compatibility rules;
- `M9MOTION-07`: source-valid reduced-motion strategies, explicit finite-repeat versus
  forever defaults, and the distinction between reduced-motion hold/restart and
  non-reduced mandatory preference suppression;
- `M9MOTION-09`: exact declaration/spec compatibility and atomic terminal/completed
  reconciliation; and
- `M9MOTION-10`: collision/compatibility/strategy/completed-state diagnostics through
  the canonical trace.

The matrix remains exactly:

```text
25 total unique rows
10 owner-accepted
0 implementation-complete
0 proof-complete
15 blocked
0 duplicate IDs
0 invalid statuses
0 invalid schemas
```

All `M9MOTION-*` and `M9INTEG-*` rows remain blocked until their normal implementation,
proof, owner acceptance, merge, accepted-main validation and later reconciliation.

## Consequences

- M9B can use one public typed target/value vocabulary without exposing generic numeric
  animation traits or renderer/dependency motion types.
- Authors who want smooth optional visual entry/exit must author explicit compatible
  endpoints; the framework does not invent transparent, zero or identity values.
- Continuous finite scalar interpolation cannot overflow merely because opposite-sign
  `f32` endpoints make an intermediate difference unrepresentable.
- Gradient motion is color animation over stable authored gradient geometry/topology,
  not vector-geometry morphing.
- Shadow list order remains identity for interpolation as well as painting.
- Typography remains truthful shaping/layout input rather than renderer text scaling.
- Structural layout animation is deliberately bounded to stable initial fields and
  same-domain numeric endpoints.
- Terminal explicit timelines remain reconciled without consuming redraw/time or
  accidentally restarting on ordinary transient rebuilds.
- Invalid competing authored motion fails before mutation rather than becoming
  iteration-order behavior.

## Rejected alternatives

### Let authored order choose among competing explicit timelines

Rejected because the same property would have multiple semantic authorities and
container/order refactors could change visible motion.

### Restart a finite declarative timeline whenever its live record is absent

Rejected because ordinary transient rebuild/publication would turn natural completion
into an accidental loop. Completion must remain reconcilable while the unchanged
specification remains authored.

### Infer animatability from numeric fields or a generic `Lerp`

Rejected because numeric representation does not imply one semantic interpolation
domain. Typography, layout modes, gradient topology and future value families require
explicit framework decisions.

### Use naive `f32` difference-based interpolation

Rejected because `a + (b - a) * t` can overflow for opposite-sign finite endpoints even
when the true convex sample is finite and inside the accepted domain.

### Treat missing optional style values as transparent/zero/identity

Rejected because that would invent color, spacing, radius or presentation-origin
semantics not authored by the application or accepted style authority.

### Interpolate gradient geometry whenever both endpoints are gradients

Rejected because it adds geometry-morph semantics, can create invalid intermediate
linear geometry and is unnecessary for initial M9. Stable geometry plus stop-color
motion is sufficient and deterministic.

### Use shortest-arc rotation automatically

Rejected because accepted presentation rotation preserves raw authored radians.
Shortest-arc/modulo normalization would silently replace authored path/direction with a
framework convention.

### Use dependency/default cubic-Bezier solvers or alternate arithmetic forms

Rejected because solver/tolerance/iteration/arithmetic differences would become
logical motion semantics. The bounded fixed algorithm is small and belongs to
RunenUI.

### Treat every finite repeat as `HoldInitial` by default

Rejected. Finite repeat has a terminal value and therefore uses the finite-timeline
`SnapToEnd` default. Only `Repeat::Forever` has no terminal value and defaults to
`HoldInitial`.

### Pause all motion hidden by mandatory preference policy

Rejected because only reduced motion owns hold/restart semantics. Ordinary mandatory
style preference precedence masks conflicting samples without creating a second hidden
clock policy.
