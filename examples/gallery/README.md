# RunenUI Gallery — native host integration in progress

The Gallery will be one independent public-contract RunenUI application where
developers can navigate and interact with accepted M11 controls.

This initial application-model slice exposes **Basics** (Button, disabled
Button) and **Inputs** (Checkbox, TextField) with application-owned state and
ordinary public `UiApp`/M10/M11 contracts. The wrapping navigation/preview
layout and a read-only live state inspector use standard public authoring;
narrow-window publication tests are structural. A native winit/wgpu/AccessKit
executable now integrates the same public Gallery application using the
accepted native mapping, keyboard/pointer and M10 text/IME translation edges.
Hosted headless validation does **not** establish actual OS window or IME behavior;
the native walkthrough and the remaining controls are **not complete yet**.

```sh
cargo run -p gallery
cargo run -p gallery --bin gallery
cargo test -p gallery
```

The default run target is `gallery-native`, a real native window host; the
separate `gallery` binary prints a deterministic headless layout representation.
Neither hosted compile/real-wgpu evidence nor that debug dump substitutes for a
physical native keyboard/mouse/IME, focus and resize walkthrough or for M11's
remaining catalog obligations. If the environment has no graphical display,
the native binary may be unable to open a window.
Counter remains the minimal teaching example and reference_winit remains the
specialized native integration host.
