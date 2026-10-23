# Influences

> **Category: Guide**

This list separates adopted implementation dependencies from external design references. Neither a dependency nor a reference implies API or behavior compatibility with RunenUI.

- [Xilem and Masonry](https://docs.rs/xilem/latest/xilem/) inform the separation between declarative authoring and a retained widget runtime.
- [Iced](https://docs.rs/iced/latest/iced/) and the Elm architecture inform application-owned state, messages/actions, and explicit update.
- [Flutter layout constraints](https://docs.flutter.dev/ui/layout/constraints) inform explicit constraint-driven layout reasoning.
- [Taffy](https://docs.rs/taffy/latest/taffy/) supplies replaceable low-level Block/Flex/Grid algorithms behind RunenUI-owned runtime contracts; its topology, styles, caches, and IDs are private derived state.
- [AccessKit](https://docs.rs/accesskit/latest/accesskit/) informs renderer-independent semantic/accessibility output and platform adaptation.
- [Winit](https://docs.rs/winit/latest/winit/) supplies native window/input mechanisms through `runenui_winit` and example hosts; it does not own RunenUI's host contract.
- [Parley](https://docs.rs/parley/latest/parley/) and Fontique are adopted internal shaping and font-discovery dependencies behind `runenui_text`.
- [WGPU](https://docs.rs/wgpu/latest/wgpu/) is used by the concrete `runenui_render_wgpu` backend. [Vello](https://docs.rs/vello/latest/vello/) remains a design reference, not an adopted renderer.

All dependency and public-contract choices remain governed by the roadmap and required ADRs.
