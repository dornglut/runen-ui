# runenui_render_wgpu

`runenui_render_wgpu` is the reusable renderer edge for RunenUI's accepted M7 reference production spine.

It consumes ordinary public `runenui_core` and `runenui_runtime` paint/publication contracts. The package owns renderer-side publication lineage, resource-provider interaction, backend realization, rendering, readback, and renderer observations. Native event-loop and accessibility integration remain outside this crate.

The current implementation fails closed: before target creation or GPU
submission it accepts only `FillRect` items with finite affine
`local_to_surface` transforms and no clips. FillRects preserve literal color
alpha and validated item opacity. A transform whose canonical
`LogicalTransform::inverse()` is unavailable contributes no paint coverage rather
than falling back to the source rectangle. For an invertible transform, finite
f32 scene components are widened only inside the renderer to f64 for affine edge
construction and raster scaling; the resulting convex polygon is clipped to the
finite target before conversion to the GPU f32 vertex ABI and triangulation. This
preserves a visible target intersection even when an irrelevant remote forward
`LogicalPoint` mapping would overflow f32, without replacing transformed geometry
with an axis-aligned bounding box. Stroke, image, shaped-text, unknown primitive,
and all clip semantics remain unsupported. Canonical
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
Display-handle construction alone remains headless: it does not claim that the
selected adapter can present to a particular native target.

`Renderer::request_with_surface_target` accepts a boxed
`WgpuHasDisplayHandle` plus an owned `wgpu::WindowHandle + 'static`. It constructs
the `Instance` with `InstanceDescriptor::new_with_display_handle`, then constructs
the safe window-only target with `SurfaceTarget::from_window_without_display`.
wgpu 30 requires the instance display when presentation through GLES is intended,
especially on Wayland; passing it at instance creation also ensures that the
window-only target uses the same display connection. wgpu retains the window
handle source inside the resulting `Surface<'static>`; the renderer then retains
the `Surface`, `Instance`, compatible `Adapter`, `Device`, and `Queue`. Adapter
selection on this path uses `compatible_surface: Some(&surface)`. Structurally, a
later winit host can produce an owned display handle from its event loop, pass an
owned/`Arc` window while retaining its own clone, and keep event-loop ownership
outside this crate. This crate does not provide a concrete winit compile proof.
Surface creation must follow wgpu's platform rule (notably the macOS main-thread
requirement). This M7A seam deliberately does not configure, acquire, render to,
or present a swapchain, so it records no successful surface publication lineage.

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
Literal unpremultiplied sRGB8 RGB is decoded exactly once to straight linear RGB;
source alpha is `(color alpha / 255) * item opacity`. The shader outputs that
straight linear source, and wgpu's non-premultiplied `BlendState::ALPHA_BLENDING`
performs ordered source-over before the sRGB target applies its storage transfer.
The equations are `C_out = C_src * A_src + C_dst * (1 - A_src)` and
`A_out = A_src + A_dst * (1 - A_src)` in linear space.
The accumulated target therefore contains composited RGB: translucent paint over
transparent black does not retain the original straight source RGB bytes.
Native-surface construction queries exact adapter-specific `SurfaceCapabilities`
and honors wgpu's advertised preference order while accepting only
`Rgba8UnormSrgb` or `Bgra8UnormSrgb`. Any other/empty advertised format set fails
structurally rather than changing the accepted sRGB color contract. Resize,
present mode, alpha mode, frame latency, configuration, acquisition, and
presentation remain host/present lifecycle work for M7B. Future surface drawing
is required to call the same target-neutral scene encoder used by the offscreen
path.

The real-GPU scale proof renders identical 64x48 logical two-rectangle geometry
at scales 1.0 and 2.0, producing 64x48 and 128x96 targets with corresponding
background, rectangle-only, and overlap probes. Scale changes target
realization, never logical geometry. PNG round-trip proof uses the 2.0 output.
An independent test-only scalar oracle checks selected translucent interior
pixels without traversing scenes or rasterizing geometry. Opaque, clear, and
zero-opacity probes remain exact; translucent probes allow at most one byte per
channel for f64 oracle versus GPU f32 blend and UNORM-storage rounding.

The affine proof uses an invertible non-axis-aligned shear/translation and a
singular transform in the same ordinary publication. Exact interior probes show
the transformed parallelogram is rasterized as authored, while a point inside its
axis-aligned bounding box but outside the parallelogram stays transparent. Probes
at the untransformed source rectangle and the singular rectangle also stay
transparent, proving neither transform path falls back to source geometry. A
separate extreme-affine regression proves that an invertible transform retains
its visible target intersection when an ordinary forward f32 `LogicalPoint`
mapping of a remote corner is unrepresentable.

The authored rectangles in the existing PNG proof remain opaque. Translucent
target readback contains accumulated composited RGB, while ordinary PNG alpha is
unassociated; no translucent PNG visual or golden-comparison claim is made
without test tooling that explicitly decodes target sRGB, unpremultiplies in
linear space when alpha is nonzero, and re-encodes straight RGB to sRGB.

The package must not become UI behavior authority. In particular it must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Logical resource lookup remains caller-owned and keyed by the complete opaque `ResourceRef`; renderer caches are disposable realization state.
