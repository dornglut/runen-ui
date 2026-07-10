# Token Authoring Ergonomics

This document records the current authoring decision for token-backed style values in RunenUI.

It is an ergonomics checkpoint, not a theme-resolution design. The goal is to keep token authoring explicit and type-safe while avoiding premature shorthand syntax.

## Decision

Use typed Rust expressions as the default token authoring form for now.

```rust
button("Save")
    .background(ColorToken::new("color.action.primary"))
    .padding(SpacingToken::new("space.2"))
    .radius(RadiusToken::new("radius.control"))
```

The same model works through `element!` because macro attributes already accept Rust expressions:

```rust
element! {
    button "Save"
        action=AppAction::Save
        background = { ColorToken::new("color.action.primary") }
        padding = { SpacingToken::new("space.2") }
        radius = { RadiusToken::new("radius.control") }
}
```

Authors can also be explicit about the literal-or-token union when that helps inspection or examples:

```rust
button("Save")
    .background(ColorValue::token("color.action.primary"))
```

## Why no shorthand yet

Do not add `background_token = "..."`, `padding_token = "..."`, or similar shorthand yet.

Reasons:

- The existing typed expression path is already supported by builder calls and `element!`.
- Shorthand would expand the macro grammar before there is enough usage pressure.
- Literal and token values already share the same `ColorValue`, `SpacingValue`, and `RadiusValue` path.
- Type-specific token constructors keep incorrect token families visible at the call site.
- Token resolution does not exist yet, so shorthand would be surface sugar without a complete downstream story.

## Accepted authoring forms

### Literal builder values

```rust
button("Save")
    .background(Color::BLACK)
    .padding(EdgeInsets::all(Length::px(6.0)))
    .radius(Radius::all(Length::px(3.0)))
```

### Token builder values

```rust
button("Save")
    .background(ColorToken::new("color.action.primary"))
    .padding(SpacingToken::new("space.2"))
    .radius(RadiusToken::new("radius.control"))
```

### Explicit union builder values

```rust
button("Save")
    .background(ColorValue::token("color.action.primary"))
    .padding(SpacingValue::token("space.2"))
    .radius(RadiusValue::token("radius.control"))
```

### Literal macro values

```rust
element! {
    button "Save"
        background = { Color::BLACK }
        padding = { EdgeInsets::all(Length::px(6.0)) }
        radius = { Radius::all(Length::px(3.0)) }
}
```

### Token macro values

```rust
element! {
    button "Save"
        background = { ColorToken::new("color.action.primary") }
        padding = { SpacingToken::new("space.2") }
        radius = { RadiusToken::new("radius.control") }
}
```

## Rejected first shorthand

These forms are not accepted yet:

```rust
element! {
    button "Save"
        background_token = "color.action.primary"
        padding_token = "space.2"
        radius_token = "radius.control"
}
```

They may become useful later, but only after at least one of these is true:

- token-backed styling is common enough that repeated constructors are noisy,
- an external source syntax needs concise token syntax,
- inspector output proves that shorthand still maps cleanly to typed token values,
- token resolution diagnostics need source-location-like distinction between literal and shorthand token inputs.

## Public API boundary

The public API remains:

```text
StyleIntent
  -> ColorValue / SpacingValue / RadiusValue
      -> Literal(...)
      -> Token(...Token)
```

The macro does not become a separate styling language. It stays sugar over the builder/descriptor API.

## Next implementation implication

The next styling work should move toward computed style instead of adding more authoring syntax:

```text
StyleIntent
  -> literal-only computed style proof
  -> token map
  -> token-backed computed style resolution
```

Token shorthand should remain deferred until implementation pressure proves it is needed.
