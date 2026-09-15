# Styling Architecture

> **Category: Current architecture**

[ADR 0009](../adr/0009-production-style-layout-text-foundation.md) owns the accepted M8 style architecture. M9A extends that same production cascade with accepted static visual properties under [ADR 0010](../adr/0010-visual-composition-and-animation.md) and its accepted visual clarifications, especially [ADR 0013](../adr/0013-m9-node-decoration-publication-clarification.md) for common node decoration and [ADR 0015](../adr/0015-m9-shadow-support-and-painter-order-clarification.md) for ordinary-shadow support/order. M9B extends the same authority with target-keyed transition policy and deterministic motion under ADR 0010 plus [ADR 0012](../adr/0012-m9-group-ordering-clarification.md), [ADR 0016](../adr/0016-m9-motion-identity-and-interpolation-clarification.md), and [ADR 0017](../adr/0017-m9-zero-duration-forever-timing-clarification.md). M9C closes integrated production proof without adding another style or animation authority. This document records current implementation: M8A establishes deterministic environment, cascade, interaction-state, preference, inheritance, provenance, and invalidation behavior; M8B–M8D integrate metric typography/layout/text; M9A adds brush-valued backgrounds, outlines, ordered ordinary shadows, opacity, and static presentation; M9B adds resolved transition policy plus runtime-owned deterministic motion; M9C proves canonical interaction-driven transitions, sampled presentation correlation, ordinary public manual-time integration, and real-wgpu consumption/reconstruction through those same authorities.

## Ownership

`runenui_core` owns renderer- and host-neutral style and authored motion vocabulary plus pure computation:

- validated typed token, recipe, variant, and owner-local animation identities;
- `StyleIntent` authored recipe/variant selection plus direct property overrides;
- partial `StyleProperties` and exact `StyleTokens` content, including target-keyed transition policy contributions;
- `StyleTheme`, `StyleRecipe`, `StyleEnvironment`, explicit preference facts, and mandatory preference policy values;
- canonical typed interaction facts consumed by resolution, without live interaction authority;
- `resolve_style_in_environment`, `ComputedStyle`, exact per-property winning-layer/value provenance, resolved transition-policy provenance, unresolved-token and missing recipe/variant diagnostics;
- RunenUI-owned transition/timeline/keyframe/timing/easing/repeat/reduced-motion descriptions and pure deterministic interpolation/sampling helpers;
- direct per-property downstream effect classification.

`runenui_runtime` owns all live style and motion orchestration:

- the complete `StyleEnvironment` supplied for one surface publication attempt;
- ephemeral projection of canonical pointer/focus interaction authority into style facts;
- shared staged widget activation used by both disabled styling and semantic/capability publication;
- retained style cache compatibility, style-resolution orchestration, inspection reports, and dependency-aware invalidation;
- live transition and explicit-timeline declaration state keyed to exact mounted generations, including completed declaration retention, replacement/cancellation/removal/unmount/shutdown behavior, reduced-motion/preference decisions, and one candidate monotonic sample instant;
- publication of common background/outline decoration, sampled presentation correlation, and node-level opacity/shadow composition from effective resolved values rather than widget-private reinterpretation.

Renderers consume already-resolved and already-sampled publication facts. They do not resolve recipes, variants, token names, interaction states, transition policy, preferences, inheritance, node-decoration policy, presentation policy, timeline lifecycle, easing/interpolation, reduced-motion strategy, or theme policy. Platform adapters may supply explicit preference inputs but do not become style or motion authority.

Application state remains authoritative for durable product meaning such as validation, selection, or domain status. Runtime interaction state supplies transient framework facts such as hover, focus, active, and disabled; applications do not maintain a second hidden interaction-style or animation state machine.

## Current property and motion vocabulary

The accepted style mechanism currently represents these property families truthfully:

- foreground color;
- brush-valued background (`solid`, linear gradient, or concentric radial gradient);
- padding;
- corner radius;
- metric typography;
- optional node outline;
- one complete ordered ordinary drop-shadow list;
- effective node opacity;
- node presentation transform;
- target-keyed non-inherited transition policy for the accepted M9 motion target set.

Each visual property may be literal or use its typed token family. Transition policy resolves independently per accepted target as absent, explicit disable, or one validated `TransitionSpec`; it is policy, not an additional sampled property value. Property breadth is independent from the production resolution mechanism; later milestones may add new typed properties without changing the ownership model. M8C layout vocabulary remains separately RunenUI-owned through `LayoutStyle`; M9 accepts only its closed structural-layout motion target subset and does not create a second style cascade or layout engine.

Explicit timelines are transient element authoring rather than style-cascade entries. One owner-local `AnimationId` identifies one declarative one-target timeline; duplicate IDs or multiple explicit timelines targeting one target reject before runtime mutation. Exact mounted generation plus declaration identity/specification drives runtime reconciliation, including terminal completed-state retention.

## Resolution model

The complete publication environment contains framework defaults, one theme with exact token content and typed recipes, explicit preference facts, and preference policy. A recipe contains a base property set, typed variant definitions, and framework interaction layers.

The current resolver implements ADR 0009's deterministic property-local cascade. Inherited foreground and typography, when present, seed the child. The accepted production precedence above those seeds is low to high:

```text
framework defaults
-> theme recipe base
-> variants in stable authored order
-> active interaction layers: hover -> focus -> active -> disabled
-> authored token/literal overrides
-> mandatory preference policy
```

Later layers replace only properties they define. Ordered variants therefore have authored-order meaning, while interaction states always use framework order independent of container/hash ordering. The complete shadow list is one property: a later winning layer replaces the lower-precedence list rather than appending to it.

Transition policy participates in that same layer order on a target-by-target, non-inherited basis. Absent policy allows lower-precedence policy to remain visible; explicit disable masks lower policy; an enabled policy contributes one exact validated transition specification. The policy resolved in the same candidate as a governed target change controls that change. Policy change alone does not start inactive motion; changing an active transition spec replaces from the current sample toward the same target, while explicit disable cancels to the governed target.

`StyleResolution` records both the exact layer that last attempted to define each property/policy and whether its value was inherited, literal, resolved from a typed token, explicitly disabled, or failed because a token was missing.

A missing higher-precedence token does not expose a lower-precedence value. The property remains unresolved, provenance records the missing token at the winning layer, and typed diagnostics retain the failure. Missing recipes and variants are also diagnosed explicitly; resolution does not silently rebind them or mutate authored intent.

## Preferences and reduced motion

`StylePreferences` makes high-contrast and reduced-motion facts explicit inputs to style computation/cache compatibility and motion planning rather than ambient platform reads.

High contrast may apply mandatory `StylePreferencePolicy` properties above authored overrides, with ordinary winning-layer/token provenance. A mandatory non-reduced preference override may suppress a conflicting motion sample/group requirement without pausing its underlying same-clock lifecycle; if suppression lifts while the source remains active, the then-current sample becomes visible.

Reduced motion is owned only by explicit `StylePreferences::reduced_motion` plus each source's validated RunenUI `ReducedMotionStrategy`:

- style transitions default to `SnapToEnd`, cannot use `HoldInitial`, and may explicitly use `PreserveEssential`;
- finite timelines default to `SnapToEnd` and may explicitly use `HoldInitial` or `PreserveEssential`;
- forever timelines cannot use `SnapToEnd`, default to `HoldInitial`, and must use `PreserveEssential` to continue as essential motion;
- `HoldInitial` retains the authored timeline at keyframe 0 with no continuous redraw or hidden elapsed time and restarts from the preference-change candidate instant when reduced motion is disabled;
- `SnapToEnd` commits the finite terminal value/lifecycle atomically in the staged candidate;
- `PreserveEssential` samples normally.

Invalid source/strategy combinations reject during authored specification validation. There is no hidden speed multiplier, renderer policy, or ambient platform animation setting.

## Inheritance

Inheritance is explicit and bounded. The accepted current resolver seeds only foreground and typography from the resolved parent.

Background, padding, radius, outline, shadows, opacity, presentation, and transition policy do not inherit. Node shadows/opacity may affect a composed mounted visual subtree through runtime-owned composition-group semantics, but that effect scope is not style inheritance. Layout geometry does not inherit through the style cascade. Any future inherited property family requires an explicit accepted extension rather than CSS-like accidental propagation.

## Runtime interaction authority

Style resolution consumes an ephemeral `SurfaceInteractionProjection`; that projection is derived state, never a second live interaction model.

- hover comes from canonical pointer physical-path membership;
- active comes from canonical pressed ownership while the press remains inside;
- focus comes from runtime `FocusState`;
- disabled comes from the same staged activation fact used by capability/semantic publication.

Multi-pointer hover/active behavior is membership-based. Retained interaction projection exists only to compare cache compatibility and effective membership changes; it does not become authoritative mounted state.

Disabled style evaluation participates in the staged surface transaction. Runtime does not call a second activation path from styling and does not mutate live capability caches before publication commit. Interaction-driven target changes can start ordinary M9 transitions through the same canonical projection/cascade path; applications do not need mirrored hover/focus/active state to drive them. Accepted M9C integration proof exercises representative hover, focus, and active changes through real pointer/command ingress, this exact interaction projection, the ordinary cascade, and the existing transition planner rather than application-maintained interaction state.

## Motion sampling, retention, and invalidation

One staged surface publication candidate observes one exact runtime monotonic instant. Runtime resolves canonical target style/layout facts and transition policy, reconciles explicit declarations and style target changes at that same instant, applies preference decisions, samples effective typed motion values, computes direct effects, then drives the existing layout/text/presentation/hit/paint/semantic stages from those effective values. Motion/declaration/group state commits only with the existing final publication transaction.

Transition interruption/reversal begins from the current same-candidate sample rather than a stale prior target. Explicit timelines have precedence over same-target style transitions, and explicit replacement starts from authored keyframe 0 rather than rewriting the timeline for continuity. Unchanged completed declarations remain terminal reconciliation state without sampled-property or redraw authority until accepted removal/re-add or exact replacement.

Accepted interpolation is closed and typed rather than generic numeric `Lerp`: colors use premultiplied linear-sRGB with linear alpha; compatible brushes interpolate colors while preserving exact gradient geometry/stop topology; equal-length shadow lists pair by authored index; present presentation transforms interpolate components using raw authored radians; typography and incompatible/absent domains are discrete; accepted structural layout fields interpolate only within their frozen same-domain numeric cases. Sampled structural-layout values feed the existing runtime/Taffy/text authority.

Style/motion cache compatibility includes exact environment, interaction, target-policy/specification, mounted/declaration, preference, start/end/start-instant, and accepted effective-value facts required by the motion contract. Time advance alone does not invalidate all stages or mint publication/resource identity.

Current direct property effects remain:

- foreground, background, radius, outline, shadows, opacity -> paint;
- padding, typography -> layout;
- presentation -> presentation geometry.

M9 applies those same classifiers to effective sampled changes. Paint-only motion avoids layout/text; presentation motion updates correlated paint/hit/focus/semantic geometry without relayout; accepted layout motion recomputes layout/dependents through the existing authority. Opacity/shadow motion recomposes node-effect group structure under the accepted M9 group-lifetime rules without turning renderer caches into grouping authority. Equal consecutive samples may preserve cache/publication revision while still requesting a future redraw if visible motion remains active.

Active visible motion uses the existing redraw/wake authority. Runtime does not enqueue one FIFO action per sample and does not invent a fixed framework frame rate. Recoverable or terminally failed surface planning does not partially commit new style cache, motion lifecycle, group state, or sampled products; retained renderer retry reuses the exact already-sampled publication.

M8D's accepted integration evidence exercises resolved style through the same production surface path as responsive layout/text measurement, exact retained shaped-resource paint, semantic bounds/content, and real-wgpu composition. M9A extends that path with runtime-owned common decoration/static composition; M9B extends it again with deterministic staged sampling and exact downstream invalidation; M9C proves the resulting path across canonical interaction transitions, sampled presentation paint/hit/focus/semantic/clip correlation, public deterministic manual-time observation, and real-wgpu retained retry/cache/device reconstruction. Those additions do not move style/motion resolution into layout, text, semantics, or renderer ownership.

## Authoring

Typed Rust expressions remain the authoring form. `StyleIntent` may select one recipe, append variants in authored order, and set direct literal/token overrides. Transition policy is contributed through the same typed style layers. Built-in builders and `element!` use the same typed style intent rather than parallel styling languages.

Explicit timelines are typed transient `Element` authoring, not widget-local timers or retained application state. They reconcile against runtime-owned mounted lifetimes and use the same deterministic publication clock/transaction.

See [ADR 0001](../adr/0001-typed-token-authoring.md) for token-expression authoring. ADR 0009 remains the canonical owner of the production style cascade; accepted M9 ADRs extend visual/composition and motion semantics without duplicating that cascade.

## Current limitations

M9 visual/composition, deterministic transition/timeline and reduced-motion behavior, and integrated canonical interaction/transform/public-manual-time/real-wgpu closure are accepted current behavior. External theme serialization/loading, arbitrary renderer material/shader/filter policy, and broader later property families remain outside the current style authority. The current theme is an explicit host-neutral value supplied in `StyleEnvironment`; no ambient global theme/provider authority exists.

Later property breadth must preserve the same explicit style/motion ownership, staged sampling, canonical interaction projection, and invalidation model rather than creating a parallel cascade or animation engine.

## Extraction rule

Host-neutral style values, environment/policy values, authored motion descriptions, pure resolution/sampling, computed style, provenance/diagnostics, and direct property-effect classification remain in `runenui_core`; mounted interaction/capability authority, live motion/declaration lifecycle, clocks/scheduling, orchestration, retention, invalidation, and publication of runtime-derived effective values/decoration/effect structure remain in `runenui_runtime`.

M8/M9 do not justify a `runenui_style` or `runenui_animation` crate. A dedicated crate requires a real independent ownership, dependency, optionality, serialized-source, external-loading, or multiple-consumer boundary that Cargo should enforce; file size or property growth alone is insufficient.
