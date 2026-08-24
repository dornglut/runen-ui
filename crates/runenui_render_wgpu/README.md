# runenui_render_wgpu

`runenui_render_wgpu` is the reusable renderer edge for RunenUI's accepted M7 reference production spine.

It consumes ordinary public `runenui_core` and `runenui_runtime` paint/publication contracts. The package owns renderer-side publication lineage, resource-provider interaction, backend realization, rendering, readback, and renderer observations. Native event-loop and accessibility integration remain outside this crate.

The current FillRect checkpoint fails closed: before target creation or GPU
submission it accepts only opaque `FillRect` items with identity
`local_to_surface`, no clips, and unit item opacity. Canonical
`SceneRequirements`/`SceneCapabilities` continue to check resource kinds only;
the narrower implementation-subset validation is renderer-owned and temporary.

Offscreen publication targets initialize to transparent black. Initialization is
target policy, not authored `PaintPublication` paint. Successful publication
lineage is committed only after actual GPU submission and CPU readback complete.
The renderer returns raw GPU-derived RGBA8 sRGB bytes; PNG encoding and any
golden-file policy belong to proof tooling. Normal tests write no expected golden.

The package must not become UI behavior authority. In particular it must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Logical resource lookup remains caller-owned and keyed by the complete opaque `ResourceRef`; renderer caches are disposable realization state.
