# runenui_render_wgpu

`runenui_render_wgpu` is the reusable renderer edge for RunenUI's accepted M7 reference production spine.

It consumes ordinary public `runenui_core` and `runenui_runtime` paint/publication contracts. The package owns renderer-side publication lineage, resource-provider interaction, backend realization, rendering, readback, and renderer observations. Native event-loop and accessibility integration remain outside this crate.

The package must not become UI behavior authority. In particular it must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Logical resource lookup remains caller-owned and keyed by the complete opaque `ResourceRef`; renderer caches are disposable realization state.
