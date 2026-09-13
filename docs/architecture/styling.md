# Styling Architecture

> **Category: Current architecture**

[ADR 0009](../adr/0009-production-style-layout-text-foundation.md) owns the accepted M8 style architecture. M9A extends that same production cascade with accepted static visual properties under [ADR 0010](../adr/0010-visual-composition-and-animation.md) and its accepted clarifications, especially [ADR 0013](../adr/0013-m9-node-decoration-publication-clarification.md) for common node decoration and [ADR 0015](../adr/0015-m9-shadow-support-and-painter-order-clarification.md) for ordinary-shadow support/order. This document records current implementation: M8A establishes deterministic environment, cascade, interaction-state, preference, inheritance, provenance, and invalidation behavior; M8B–M8D integrate metric typography/layout/text; M9A adds brush-valued backgrounds, outlines, ordered ordinary shadows, opacity, and static presentation without creating a second style authority. Deterministic transitions/timelines and reduced-motion motion behavior remain M9B work.

## Ownership

`runenui_core` owns renderer- and host-neutral style vocabulary and pure style computation:

- validated typed token, recipe, and variant identities;
- `StyleIntent` authored recipe/variant selection plus direct property overrides;
- partial `StyleProperties` and exact `StyleTokens` content;
- `StyleTheme`, `StyleRecipe`, `StyleEnvironment`, explicit preference facts, and mandatory preference policy values;
- canonical typed interaction facts consumed by resolution, without live interaction authority;
- `resolve_style_in_environment`, `ComputedStyle`, exact per-property winning-layer/value provenance, unresolved-token and missing recipe/variant diagnostics;
- direct per-property downstream effect classification.

`runenui_runtime` owns all live style orchestration:

- the complete `StyleEnvironment` supplied for one surface publication attempt;
- ephemeral projection of canonical pointer/focus interaction authority into style facts;
- shared staged widget activation used by both disabled styling and semantic/capability publication;
- retained style cache compatibility, style-resolution orchestration, inspection reports, and dependency-aware invalidation;
- publication of common background/outline decoration, static presentation correlation, and node-level opacity/shadow composition from the resolved style rather than widget-private reinterpretation.

Renderers consume resolved/publication visual facts. They do not resolve recipes, variants, token names, interaction states, preferences, inheritance, node-decoration policy, presentation policy, or theme policy. Platform adapters may supply explicit preference inputs but do not become style authority.

Application state remains authoritative for durable product meaning such as validation, selection, or domain status. Runtime interaction state supplies transient framework facts such as hover, focus, active, and disabled; applications do not maintain a second hidden interaction-style state machine.

## Current property vocabulary

The accepted style mechanism currently represents these property families truthfully:

- foreground color;
- brush-valued background (`solid`, linear gradient, or concentric radial gradient);
- padding;
- corner radius;
- metric typography;
- optional node outline;
- one complete ordered ordinary drop-shadow list;
- effective node opacity;
- static node presentation transform.

Each property may be literal or use its typed token family. Property breadth is independent from the production resolution mechanism; later milestones may add new typed properties without changing the ownership model. M8C layout vocabulary remains separately RunenUI-owned through `LayoutStyle`; M9A visual/composition values do not create a second style cascade.

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

`StyleResolution` records both the exact layer that last attempted to define each property and whether its value was inherited, literal, resolved from a typed token, or failed because a token was missing.

A missing higher-precedence token does not expose a lower-precedence value. The property remains unresolved, provenance records the missing token at the winning layer, and typed diagnostics retain the failure. Missing recipes and variants are also diagnosed explicitly; resolution does not silently rebind them or mutate authored intent.

## Preferences

`StylePreferences` makes high-contrast and reduced-motion facts explicit inputs to style computation/cache compatibility rather than ambient platform reads.

High contrast may apply mandatory `StylePreferencePolicy` properties above authored overrides, with ordinary winning-layer/token provenance.

M9A adds static presentation/composition only; it does not add a transition/timeline sampling property family. Reduced motion therefore remains an explicit preference and cache/invalidation fact without fabricated static overrides. M9B owns deterministic motion/timeline behavior and the exact reduced-motion policy applied to that motion while preserving the same preference ownership.

## Inheritance

Inheritance is explicit and bounded. The accepted current resolver seeds only foreground and typography from the resolved parent.

Background, padding, radius, outline, shadows, opacity, and presentation do not inherit. Node shadows/opacity may affect a composed mounted visual subtree through runtime-owned composition-group semantics, but that effect scope is not style inheritance. Layout geometry does not inherit through the style cascade. Any future inherited property family requires an explicit accepted extension rather than CSS-like accidental propagation.

## Runtime interaction authority

Style resolution consumes an ephemeral `SurfaceInteractionProjection`; that projection is derived state, never a second live interaction model.

- hover comes from canonical pointer physical-path membership;
- active comes from canonical pressed ownership while the press remains inside;
- focus comes from runtime `FocusState`;
- disabled comes from the same staged activation fact used by capability/semantic publication.

Multi-pointer hover/active behavior is membership-based. Retained interaction projection exists only to compare cache compatibility and effective membership changes; it does not become authoritative mounted state.

Disabled style evaluation participates in the staged surface transaction. Runtime does not call a second activation path from styling and does not mutate live capability caches before publication commit.

## Retention and invalidation

Style cache compatibility includes exact style-environment content and the effective interaction projection. A token revision or other hint is not sufficient authority when content differs.

Current direct property effects are:

- foreground, background, radius, outline, shadows, opacity -> paint;
- padding, typography -> layout;
- presentation -> presentation geometry.

These are direct effects only. Runtime owns dependency propagation: a layout change also makes every dependent text/layout, presentation, hit, paint-placement, and semantic-geometry fact stale as required. Presentation changes update correlated paint/hit/focus/semantic publication geometry without mutating retained layout authority. Paint-only style changes do not force layout work when retained facts remain compatible.

Preference/environment or interaction changes first invalidate style resolution as required; exact computed-property differences then determine downstream work. Recoverable or terminally failed surface planning does not commit a partial new retained style cache.

M8D's accepted integration evidence exercises resolved style through the same production surface path as responsive layout/text measurement, exact retained shaped-resource paint, semantic bounds/content, and real-wgpu composition. M9A extends that path: runtime synthesizes common background/outline decoration from the resolved final owner-local box/radius; static presentation is correlated through paint/hit/focus/semantic geometry; and shadows/opacity are represented through runtime-owned immutable composition groups/effect bounds. Those additions do not move style resolution into layout, text, semantics, or renderer ownership.

## Authoring

Typed Rust expressions remain the authoring form. `StyleIntent` may select one recipe, append variants in authored order, and set direct literal/token overrides. Built-in builders and `element!` use the same typed style intent rather than parallel styling languages.

See [ADR 0001](../adr/0001-typed-token-authoring.md) for token-expression authoring. ADR 0009 remains the canonical owner of the production style cascade; accepted M9 ADRs extend static visual/composition semantics without duplicating that cascade.

## Current limitations

The accepted style mechanism does not yet provide deterministic transitions/timelines, animation sampling, or reduced-motion transformation of active motion; those belong to M9B. External theme serialization/loading, arbitrary renderer material/shader/filter policy, and broader later property families also remain outside the current style authority. The current theme is an explicit host-neutral value supplied in `StyleEnvironment`; no ambient global theme/provider authority exists.

M9A's accepted static visual breadth is current behavior; it must not be described as future merely because M9 motion/integration remains incomplete. M9B owns motion/timeline policy and M9C owns integrated visual-motion closure. Later property breadth must preserve the same explicit style ownership and invalidation model rather than creating a parallel cascade.

## Extraction rule

Host-neutral style values, environment/policy values, pure resolution, computed style, provenance/diagnostics, and direct property-effect classification remain in `runenui_core`; mounted interaction/capability authority, orchestration, retention, invalidation, and publication of runtime-derived decoration/effect structure remain in `runenui_runtime`.

M8/M9A do not justify a `runenui_style` crate. A dedicated crate requires a real independent ownership, dependency, optionality, serialized-source, external-loading, or multiple-consumer boundary that Cargo should enforce; file size or property growth alone is insufficient.
