# Styling Architecture

> **Category: Current architecture**

This document describes the style system that exists in the current headless framework foundation. Production theme/recipe/state-style expansion belongs to the roadmap and requires its own accepted contracts before replacing this model.

## Current ownership

`runenui_core` owns renderer- and host-neutral style values, typed token references, `StyleIntent`, `StyleTokens`, pure resolution, `ComputedStyle`, provenance, and deterministic diagnostics. `runenui_runtime` owns mounted authored-style observation, resolution orchestration, cache compatibility, invalidation, and publication of resolved style facts.

Renderers consume resolved visual facts. They do not resolve token names, authored style intent, or future theme policy. Layout consumes only resolved geometry-affecting style values.

## Current proof behavior

The current implementation provides:

- literal colors, padding, and corner radii;
- typed color, spacing, and radius token references;
- deterministic token definition and lookup with validated textual identity;
- `StyleIntent` to `ComputedStyle` resolution;
- per-field provenance and unresolved-token diagnostics;
- exact token-content compatibility for retained publication rather than trusting only a revision counter;
- mounted current-value reads so retained topology cannot freeze stale authored style;
- dependency-aware invalidation: padding changes affect layout/hit/semantic bounds/paint placement, while visual-only changes can remain paint-only.

Missing tokens are non-fatal but explicit: the unresolved field remains absent, provenance records the missing token, and diagnostics retain the failure. Consumers do not invent a hidden fallback.

`StyleTokens::revision()` is a diagnostic/change hint, not sole cache authority. Independent token sets with equal revisions cannot alias when their content differs.

## Authoring

Typed Rust expressions remain the current token authoring form:

```rust
button("Save")
    .background(color_token!("color.action.primary"))
    .padding(spacing_token!("space.2"))
    .radius(radius_token!("radius.control"))
```

`element!` uses the same typed expressions. See [ADR 0001](../adr/0001-typed-token-authoring.md).

## Current limitations

The current style proof does not provide production typography, borders, shadows, opacity, transforms, themes, recipes, variants, inheritance, external theme loading, interaction-state layers, high-contrast/reduced-motion policy, or renderer material systems.

Application state remains authoritative for durable product meaning such as validation or selection. Mounted interaction state exists independently, but a production recipe/state-layer model is not implied merely because hover, press, focus, and related runtime facts are available.

## Extraction rule

Primitive values, token references, pure resolution, computed style, provenance, and narrow diagnostics remain in `runenui_core`; mounted orchestration remains in `runenui_runtime`.

A dedicated style/theme crate requires a real independent policy or dependency boundary such as external theme loading, recipes/state layers, fallback/inheritance, serialized validation, multiple independent consumers, or optional dependencies that Cargo should enforce. Moving existing types or reacting to file size is insufficient.

The [roadmap](../roadmap.md) owns production styling sequencing. Future theme/recipe resolution rules require accepted architecture rather than being pre-decided here.
