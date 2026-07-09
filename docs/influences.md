# Influences

RunenUI is informed by several existing UI systems. These references describe similarity in direction, not an intent to clone any one framework.

- [Xilem / Masonry](https://docs.rs/xilem/latest/xilem/) — similar in its Rust-native direction, retained runtime structure, and separation between authoring APIs and lower-level UI infrastructure.
- [Iced / Elm Architecture](https://docs.rs/iced/latest/iced/) — similar in its state, action, update, and element flow.
- [Dioxus](https://docs.rs/dioxus/latest/dioxus/prelude/macro.rsx.html) / [Leptos](https://book.leptos.dev/view/01_basic_component.html) — similar in using a Rust-native macro authoring surface for readable nested UI.
- [egui](https://github.com/emilk/egui) / Dear ImGui — similar in prioritizing low-friction authoring and practical integration into tools, editors, and custom engines.
- [Flutter](https://docs.flutter.dev/ui/layout/constraints) — similar in treating layout as a distinct system with explicit constraints and computed geometry.
- [AccessKit](https://docs.rs/accesskit/latest/accesskit/) — similar in treating accessibility as structured UI data.
- [Vello](https://docs.rs/vello/latest/vello/) / Skia-style rendering stacks — similar in separating UI structure from renderer-facing output.

RunenUI combines these ideas around its own vocabulary: `Element`, `Action`, `update`, `Runtime`, `LayoutBox`, `Primitive`, and `SurfaceFrame`.
