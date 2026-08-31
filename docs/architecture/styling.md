# Styling Architecture

> **Category: Current architecture**

[ADR 0009](../adr/0009-production-style-layout-text-foundation.md) owns the accepted M8 architectural decisions. This document records the accepted current implementation of that architecture: M8A establishes deterministic environment, cascade, interaction-state, preference, inheritance, provenance, and invalidation behavior over the property families that are currently implemented truthfully. Production typography/text and responsive layout breadth remain M8B/M8C work.

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
- retained style cache compatibility, style-resolution orchestration, inspection reports, and dependency-aware invalidation.

Renderers consume resolved visual facts. They do not resolve recipes, variants, token names, interaction states, preferences, inheritance, or theme policy. Platform adapters may supply explicit preference inputs but do not become style authority.

Application state remains authoritative for durable product meaning such as validation, selection, or domain status. Runtime interaction state supplies transient framework facts such as hover, focus, active, and disabled; applications do not maintain a second hidden interaction-style state machine.

## Current property vocabulary

M8A deliberately preserves only property families that the implementation can represent truthfully today:

- foreground color;
- background color;
- padding;
- corner radius.

Each property may be literal or use its typed token family. Property breadth is independent from the production resolution mechanism; later slices may add new typed properties without changing the ownership model.

## Resolution model

The complete publication environment contains framework defaults, one theme with exact token content and typed recipes, explicit preference facts, and preference policy. A recipe contains a base property set, typed variant definitions, and framework interaction layers.

The current resolver implements ADR 0009's deterministic property-local cascade. A parent foreground, when present, is a bounded inherited seed. The accepted production precedence above that seed is low to high:

```text
framework defaults
-> theme recipe base
-> variants in stable authored order
-> active interaction layers: hover -> focus -> active -> disabled
-> authored token/literal overrides
-> mandatory preference policy
```

Later layers replace only properties they define. Ordered variants therefore have authored-order meaning, while interaction states always use framework order independent of container/hash ordering.

`StyleResolution` records both the exact layer that last attempted to define each property and whether its value was inherited, literal, resolved from a typed token, or failed because a token was missing.

A missing higher-precedence token does not expose a lower-precedence value. The property remains unresolved, provenance records the missing token at the winning layer, and typed diagnostics retain the failure. Missing recipes and variants are also diagnosed explicitly; resolution does not silently rebind them or mutate authored intent.

## Preferences

`StylePreferences` makes high-contrast and reduced-motion facts explicit inputs to style computation/cache compatibility rather than ambient platform reads.

High contrast may apply mandatory `StylePreferencePolicy` properties above authored overrides, with ordinary winning-layer/token provenance.

M8A has no animation property family. Reduced motion is therefore an explicit preference and cache/invalidation fact but currently applies no style-property override. M9 may add motion properties/policy without changing preference ownership.

## Inheritance

Inheritance is explicit and bounded. Current M8A inheritance seeds only foreground from the resolved parent.

Background, padding, and radius do not inherit. Geometry cannot acquire CSS-like inheritance accidentally. Future typography inheritance requires an explicit accepted extension when M8B introduces production typography.

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

- foreground, background, radius -> paint;
- padding -> layout.

These are direct effects only. Runtime owns dependency propagation: a layout-affecting style change also makes every dependent hit, paint-placement, and semantic-geometry fact stale. Paint-only style changes do not force layout work when retained facts remain compatible.

Preference/environment or interaction changes first invalidate style resolution as required; exact computed-property differences then determine downstream work. Recoverable or terminally failed surface planning does not commit a partial new retained style cache.

## Authoring

Typed Rust expressions remain the authoring form. `StyleIntent` may select one recipe, append variants in authored order, and set direct literal/token overrides. Built-in builders and `element!` use the same typed style intent rather than parallel styling languages.

See [ADR 0001](../adr/0001-typed-token-authoring.md) for token-expression authoring. ADR 0009 remains the canonical owner of the M8 architectural decisions summarized by this current-architecture document.

## Current limitations

The accepted M8A mechanism does not yet provide production typography, borders, shadows, opacity, transforms, external theme serialization/loading, broad renderer material systems, or animation/motion properties. The current theme is an explicit host-neutral value supplied in `StyleEnvironment`; no ambient global theme/provider authority exists.

M8B owns production logical text/typography and SDF/MSDF realization. M8C owns production responsive layout and the style/layout property breadth required by that work. M9 owns motion/animation behavior.

## Extraction rule

Host-neutral style values, environment/policy values, pure resolution, computed style, provenance/diagnostics, and direct property-effect classification remain in `runenui_core`; mounted interaction/capability authority, orchestration, retention, and invalidation remain in `runenui_runtime`.

M8A does not justify a `runenui_style` crate. A dedicated crate requires a real independent ownership, dependency, optionality, serialized-source, external-loading, or multiple-consumer boundary that Cargo should enforce; file size or property growth alone is insufficient.
