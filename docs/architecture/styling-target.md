# Styling Target Architecture

This document defines the intended end product for styling in RunenUI.

It is a target architecture, not permission to create all styling crates or systems immediately. The purpose is to keep the first token/style implementation slice aligned with the long-term model.

## Decision

RunenUI styling should be a host-neutral data pipeline:

```text
Element tree
  -> style intent
  -> theme and recipe resolution
  -> computed style
  -> layout inputs
  -> renderer-neutral visual output
  -> concrete renderer backend
```

The final styling system should not be a renderer API, a CSS clone, or a windowing concern.

For the token-reference target that follows this document, see [Token Reference Target](token-reference-target.md). For the current token authoring decision, see [Token Authoring Ergonomics](token-authoring-ergonomics.md).

## Goals

- Keep styling explicit, inspectable, and deterministic.
- Support app UI, game UI, editor UI, and tools UI.
- Support SDF and conventional raster/GPU renderers without making either backend the source of truth.
- Support typed Rust authoring first.
- Leave room for optional external source formats later.
- Make common controls easy to theme without hardcoding visual policy into controls.
- Make accessibility-relevant visual state visible to tools and tests.
- Allow live preview and inspector tooling to explain where a visual value came from.

## Non-goals

The styling target does not require these immediately:

- CSS-compatible syntax.
- DOM-style global cascade.
- browser layout compatibility.
- runtime stylesheet parsing.
- hot reload in the first implementation slice.
- a separate `runenui_style` or `runenui_theme` crate before the model has enough pressure.
- renderer-specific material/shader APIs in the style layer.

## Ownership model

### `runenui_core`

Owns the smallest stable style vocabulary:

- primitive value types
- optional token references
- element-local style intent
- semantic style slots only when they are independent of a control implementation

Core must not own theme loading, renderer materials, platform color APIs, stylesheet parsing, or control recipes.

### `runenui_runtime`

Eventually owns or orchestrates style resolution for a mounted tree:

- current theme selection
- interaction state inputs
- computed style production
- trace/debug visibility for style resolution
- invalidation when style-affecting state changes

Runtime must not draw pixels and must not embed a concrete renderer.

### future `runenui_style` or `runenui_theme`

A dedicated crate becomes justified only after style resolution needs independent API and tests.

It would own:

- token maps
- theme definitions
- component recipes
- variant resolution
- state-layer rules
- computed style types
- style diagnostics

Do not extract this crate while styling is only a few primitive types and direct element fields.

### future `runenui_render`

Renderer-neutral render output consumes computed visual values. It may carry paints, strokes, text runs, clips, transforms, and shape primitives.

It must not own theme resolution.

## Styling layers

The target system has five conceptual layers.

### 1. Primitive values

Primitive values are typed, renderer-neutral values:

```text
Color
Length
Spacing
Radius
Border
Opacity
FontSize
FontWeight
```

They are not theme-aware by themselves.

The first implementation should start with the minimum primitives required by real element fields. Do not add broad value types before a caller exists.

### 2. Token references

Token references point to named design values:

```text
color.surface.background
color.text.primary
space.2
radius.control
```

Token references should be typed where possible. A color token should not be accidentally used as a spacing token.

Target shape:

```rust
ColorValue::literal(Color::rgba(...))
ColorValue::token("color.text.primary")
ColorValue::Token(ColorToken::new("color.text.primary"))
```

or an equivalent typed design that preserves the same constraints.

Detailed token-reference decisions are tracked in [Token Reference Target](token-reference-target.md). Current authoring ergonomics are tracked in [Token Authoring Ergonomics](token-authoring-ergonomics.md).

### 3. Element-local style intent

Elements can carry style intent directly:

```text
padding
margin
gap
background
foreground
border
radius
font size
```

This intent may be concrete or token-backed.

Element-local style is not the whole system. It is the low-level escape hatch and the foundation for tests.

### 4. Component recipes and variants

Controls should not hardcode their final visuals. A button should expose semantic component information such as:

```text
component = button
variant = primary | secondary | subtle | danger
state = enabled | disabled | focused | hovered | pressed
```

A recipe maps that semantic component state to concrete style intent.

Target example:

```text
button.primary.enabled
button.primary.hovered
button.primary.pressed
button.primary.disabled
```

This keeps control behavior separate from visual policy.

### 5. Computed style

Computed style is the fully resolved output of style resolution:

```text
no unresolved tokens
no variant indirection
no recipe indirection
only values that layout/render/accessibility can consume
```

Computed style should be inspectable and testable.

Layout reads computed geometry-affecting values such as padding, gap, border width, font size, and sizing constraints.

Render reads computed visual values such as fills, strokes, radius, opacity, text color, and effects.

Accessibility tooling reads visual state that affects contrast, focus visibility, disabled state, and semantic emphasis.

## Resolution order

The target resolution order should be explicit and stable:

```text
default element style
  -> control recipe defaults
  -> component variant
  -> interaction state layer
  -> theme token resolution
  -> element-local overrides
  -> computed style
```

This may change after implementation pressure, but the system must always be able to explain the source of the winning value.

## State model

Styling state should separate app state from UI interaction state.

Application state belongs to the app:

```text
selected item
validation error
business status
```

UI interaction state belongs to the runtime:

```text
focused
hovered
pressed
disabled
active
```

Control semantics connect these two worlds without making visual styling own app logic.

## Authoring target

Normal app authors should be able to stay simple:

```rust
button("Save")
    .variant("primary")
    .on_press(AppAction::Save)
```

Direct style overrides should remain possible. Current token-backed authoring uses typed values rather than string shorthand:

```rust
button("Save")
    .background(ColorToken::new("color.action.primary"))
    .radius(RadiusToken::new("radius.control"))
```

The same model can be authored through `element!` expression attributes:

```rust
element! {
    button "Save"
        action=AppAction::Save
        background = { ColorToken::new("color.action.primary") }
        radius = { RadiusToken::new("radius.control") }
}
```

The exact API names are not fixed by this document. The target is the separation of concerns:

```text
structure and action binding
  separate from
semantic variant
  separate from
resolved visual values
```

Macro sugar may eventually expose the same model:

```rust
element! {
    button "Save" variant="primary" action=AppAction::Save
}
```

The builder/descriptor model remains the authority underneath macro syntax.

## Theme target

A theme should be data, not code that mutates controls.

A theme should eventually provide:

```text
typed token values
component recipes
variant rules
state-layer rules
default typography
spacing scale
radius scale
focus-ring policy
```

The theme is not allowed to own app behavior, input routing, layout algorithms, or renderer backends.

## Renderer target

Renderers consume computed visual output. They should not know whether a value came from a token, recipe, variant, or local override.

SDF backends may turn computed style into SDF shapes and shader parameters.

Raster/GPU backends may turn computed style into quads, paths, text runs, clips, and draw calls.

Both consume the same resolved UI facts.

## Accessibility target

Styling must preserve accessibility-visible facts:

- focus indicator policy
- disabled state
- contrast-sensitive foreground/background pairs
- semantic emphasis
- validation/error state
- selected/current state

The style system should make these inspectable rather than burying them in renderer code.

## Live preview and tools target

The styling system should eventually support inspection:

```text
selected node
  -> authored element style
  -> matched component recipe
  -> active state layer
  -> token resolution
  -> final computed style
```

This is why the style pipeline should be explicit and data-driven.

## First implementation slice after this design

After this document is accepted, the first code slice should not implement a theme engine.

It should add only the minimum core vocabulary needed for future style intent:

```text
Color
Length
Spacing or EdgeInsets
Radius
```

Optionally add typed token references only if the resulting API remains small.

The first code slice should include tests for value construction and preserve current counter behavior.

## Extraction criteria

Create a dedicated style/theme crate only after at least two of these are true:

- multiple controls need shared visual recipes
- token resolution exists and needs independent tests
- computed style becomes a public data product
- renderer output needs resolved visual values from more than one source
- live inspector/debug output needs style provenance
- theme data needs serialization or external loading

Until then, keep styling as modules in `runenui_core` and `runenui_runtime`.

## Open questions

These should not block the first token vocabulary slice:

- Should token IDs be interned, strongly typed newtypes, or simple strings initially?
- Should recipes be Rust-authored only at first?
- Should external theme files be RON, TOML, or a custom source format later?
- How much selector-like behavior is needed before it becomes harmful complexity?
- Which computed style facts belong in `SurfaceFrame` versus a separate render model?
