# RunenUI Gallery — initial work in progress

The Gallery will be one independent public-contract RunenUI application where
developers can navigate and interact with accepted M11 controls.

This initial application-model slice exposes **Basics** (Button, disabled
Button) and **Inputs** (Checkbox, TextField) with application-owned state and
ordinary public `UiApp`/M10/M11 contracts. The wrapping navigation/preview
layout and a read-only live state inspector use standard public authoring;
narrow-window publication tests are structural, not native resize evidence. A compiled, native interactive
window and the remaining controls are **not implemented yet**.

```sh
cargo run -p gallery
cargo test -p gallery
```

The current binary prints a deterministic headless layout representation, **not
a live native gallery**. It is not conformance authority, a mock of native
platform behavior, or a substitute for M11's 89 owner-accepted obligations.
Counter remains the minimal teaching example and reference_winit remains the
specialized native integration host.
