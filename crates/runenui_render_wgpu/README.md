# runenui_render_wgpu

`runenui_render_wgpu` is the reusable renderer edge for RunenUI's accepted M7 reference production spine.

It consumes ordinary public `runenui_core` and `runenui_runtime` paint/publication contracts. The package owns renderer-side publication lineage, resource-provider interaction, backend realization, rendering, readback, and renderer observations. Native event-loop and accessibility integration remain outside this crate.

The current implementation fails closed: before target creation or GPU
submission it accepts only opaque `FillRect` items with identity
`local_to_surface`, no clips, and unit item opacity. Canonical
`SceneRequirements`/`SceneCapabilities` continue to check resource kinds only;
the narrower implementation-subset validation and its detailed reasons remain
renderer-internal. The public render error reports an unsupported scene without
making temporary implementation progress part of the lasting API.

An offscreen target retains its texture, extent, format, and successful
publication lineage as one realization. `AlreadyCurrent` and `ExactBaseMatch`
are available only against that live target. Explicit loss, first creation, or
extent/format rebuild starts with empty lineage and forces `FullResync`. A
post-submission failure conservatively drops the target; validation failures do
not mutate it. `discard_offscreen_target` makes this lifetime boundary explicit.

Offscreen publication targets initialize to transparent black. Initialization
is target policy, not authored `PaintPublication` paint. Successful target
lineage is committed only after actual GPU submission and CPU readback complete.
The renderer returns raw GPU-derived RGBA8 sRGB bytes; PNG encoding and any
golden-file policy belong to proof tooling. Normal tests write no expected
golden.

`Renderer::request` is the headless constructor.
`Renderer::request_with_display_handle` accepts wgpu's owned, thread-safe
`WgpuHasDisplayHandle` abstraction without a winit or event-loop dependency.
`wgpu-types` is a direct exact-version dependency only because wgpu 30 does not
re-export that trait; `raw-window-handle` is not a direct dependency.

Device creation deliberately requests `Features::empty()` and
`Limits::downlevel_defaults().using_resolution(adapter.limits())`, disables
experimental features and tracing, and chooses performance memory hints. This
uses only portable core rendering, sampled-texture, buffer, and readback
facilities needed by current and planned M7A paint/resource work while retaining
the selected adapter's target-resolution limits. Diagnostics distinguish adapter
capabilities, requested device policy, and actual device capabilities.

FillRect pipelines are renderer-owned and cached by target `TextureFormat`.
The shared scene encoder supports both `Rgba8UnormSrgb` and
`Bgra8UnormSrgb` with a format-matching pipeline and rejects other formats. The
controlled offscreen readback target remains `Rgba8UnormSrgb`.

The real-GPU scale proof renders identical 64x48 logical two-rectangle geometry
at scales 1.0 and 2.0, producing 64x48 and 128x96 targets with corresponding
background, rectangle-only, and overlap probes. Scale changes target
realization, never logical geometry. PNG round-trip proof uses the 2.0 output.

The package must not become UI behavior authority. In particular it must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Logical resource lookup remains caller-owned and keyed by the complete opaque `ResourceRef`; renderer caches are disposable realization state.
