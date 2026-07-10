# Token Reference Target

This document defines the target shape for style token references in RunenUI before token APIs are added to `runenui_core`.

It is a design checkpoint. It does not create a theme engine, token map, computed style resolver, serializer, external document format, or dedicated style crate.

## Decision

RunenUI should represent token references as typed references to semantic token IDs, not as unstructured local style strings.

Token references should allow an authored element to say:

```text
use the foreground color named color.text.default
use the background color named color.surface.raised
use the radius named radius.control
use the spacing named spacing.control.padding
```

without resolving those names inside `runenui_core`.

The core model should preserve the distinction between:

```text
literal visual values
  and
unresolved token references
```

This lets tools, tests, live preview, and later theme resolution explain whether a style value came from a literal override or a theme token.

## Goals

- Keep token references host-neutral.
- Keep token references renderer-neutral.
- Keep token references inspectable.
- Keep token references typed by value family.
- Preserve local literal values as a valid authoring path.
- Avoid committing to theme loading or serialization too early.
- Avoid making arbitrary strings indistinguishable from typed style values.

## Non-goals

This target does not introduce:

- token resolution
- a token registry
- theme files
- theme inheritance
- selector matching
- component recipes
- computed style
- renderer-specific materials
- external syntax
- hot reload
- a `runenui_style` or `runenui_theme` crate

## Ownership

### `runenui_core`

Core should own only the typed reference vocabulary:

```text
TokenId
ColorToken
LengthToken
SpacingToken
RadiusToken
StyleValue<T>
```

or a similarly small equivalent.

Core should not own token maps, theme loading, fallback policy, missing-token diagnostics, variant resolution, or computed style production.

### `runenui_runtime`

Runtime may later orchestrate token resolution for a mounted tree once a theme source exists.

That future responsibility includes:

```text
mounted element tree
  -> authored style intent
  -> active theme/token source
  -> resolved computed style
  -> trace/debug provenance
```

Runtime still must not draw pixels or embed a concrete renderer.

### Future style/theme crate

A dedicated crate becomes justified only when token maps, theme data, recipes, or computed style resolution require independent API and tests.

Until then, token references should stay in `runenui_core` as primitive vocabulary.

## Typed token families

Token references should be typed by the kind of value they resolve to.

Recommended starting families:

```text
ColorToken
LengthToken
SpacingToken
RadiusToken
```

These match the current value vocabulary from the first style implementation slice:

```text
Color
Length
EdgeInsets / Spacing
Radius
```

Typed token families prevent accidentally passing a radius token where a color is expected.

## Token identity

The first implementation should use a small owned identifier type rather than raw public strings everywhere.

Target shape:

```rust
TokenId::new("color.text.default")
ColorToken::new("color.text.default")
RadiusToken::new("radius.control")
```

The exact constructor names are not fixed, but the public API should make the reference type explicit.

`TokenId` should initially be a lightweight owned string wrapper.

Do not intern token IDs yet. Interning can be added later if profiling or large document models justify it.

## Literal values versus token references

Style APIs need to support both literal values and unresolved token references.

Target conceptual model:

```rust
enum StyleValue<T, Token> {
    Literal(T),
    Token(Token),
}
```

The exact generic shape can vary, but the model must preserve provenance:

```text
literal Color::WHITE
  is not the same authored fact as
ColorToken::new("color.text.default")
```

This matters for inspectors, theme previews, diagnostics, and serialization.

## StyleIntent target

`StyleIntent` should eventually be able to store either literal values or token references.

Current local literal style intent:

```rust
button("Save")
    .background(Color::BLACK)
    .radius(Radius::all(Length::px(3.0)))
```

Future token-based style intent:

```rust
button("Save")
    .background_token(ColorToken::new("color.action.primary"))
    .radius_token(RadiusToken::new("radius.control"))
```

Alternative API names are acceptable, but token references must stay visibly different from literal values.

## Macro target

`element!` should keep literal style expressions explicit:

```rust
element! {
    button "Save"
        background = { Color::BLACK }
        radius = { Radius::all(Length::px(3.0)) }
}
```

Token references should use explicit token attribute names or explicit token constructors:

```rust
element! {
    button "Save"
        background_token = "color.action.primary"
        radius_token = "radius.control"
}
```

or:

```rust
element! {
    button "Save"
        background = { ColorToken::new("color.action.primary") }
}
```

Do not add token macro syntax before the core token-reference types exist.

## Serialization target

Token IDs should be serializable later, but the first implementation should not add `serde` or external format support.

Future external theme data may use names like:

```text
color.text.default
color.text.muted
color.surface.base
color.surface.raised
color.action.primary
spacing.control.padding
radius.control
```

The first implementation should not enforce a global naming schema beyond storing non-empty token IDs.

## Validation rules

The first implementation should probably enforce only:

```text
- token ID is non-empty
- token ID can be inspected as a string
- typed token wrappers preserve their ID
```

Do not add broad naming validation yet. Token naming conventions should be documented before they are enforced.

## First implementation slice after this design

After this document is accepted, the next code slice should add only token-reference primitives.

Recommended scope:

```text
- TokenId
- ColorToken
- LengthToken
- SpacingToken
- RadiusToken
- tests for construction, equality, and inspection
- root/prelude exports
```

Do not modify `Element`, `StyleIntent`, `element!`, runtime, or renderer in that first token-reference code slice.

After that, a separate slice can connect token references into `StyleIntent`.

## Open questions

These should not block the first token-reference primitive slice:

- Should token IDs eventually be interned?
- Should token namespaces be validated by convention?
- Should token references support fallback values?
- Should missing-token diagnostics live in runtime or a future style crate?
- Should external theme files use RON, TOML, JSON, or a custom source format?
- Should component recipes refer to tokens directly or through semantic slots?
