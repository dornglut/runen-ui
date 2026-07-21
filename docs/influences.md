# Influences

> **Category: Guide**

These projects inform specific design questions; they are not dependencies, compatibility claims, or evidence that RunenUI currently implements equivalent behavior.

- [Xilem and Masonry](https://docs.rs/xilem/latest/xilem/) inform the separation between declarative authoring and a retained widget runtime.
- [Iced](https://docs.rs/iced/latest/iced/) and the Elm architecture inform application-owned state, messages/actions, and explicit update.
- [Flutter layout constraints](https://docs.flutter.dev/ui/layout/constraints) inform explicit constraint-driven layout reasoning.
- [Taffy](https://docs.rs/taffy/latest/taffy/) is a future adopt-versus-build candidate for standard layout algorithms behind RunenUI-owned contracts.
- [AccessKit](https://docs.rs/accesskit/latest/accesskit/) informs renderer-independent semantic/accessibility output and platform adaptation.
- [Winit](https://docs.rs/winit/latest/winit/) is a potential low-level desktop adapter dependency, not the RunenUI host contract.
- [Parley](https://docs.rs/parley/latest/parley/) and Fontique are future text-stack candidates that require an ADR before adoption.
- [Vello](https://docs.rs/vello/latest/vello/) and WGPU-style backends inform renderer protocol questions; no production backend is currently selected.

All dependency and public-contract choices remain governed by the roadmap and required ADRs.
