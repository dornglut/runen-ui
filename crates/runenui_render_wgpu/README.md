# runenui_render_wgpu

`runenui_render_wgpu` is the reusable renderer edge for RunenUI's accepted M7 reference production spine.

It consumes ordinary public `runenui_core` and `runenui_runtime` paint/publication contracts. The package owns renderer-side publication lineage, resource-provider interaction, backend realization, rendering, readback, and renderer observations. Native event-loop and accessibility integration remain outside this crate.

The current implementation fails closed: before target creation or GPU
submission it accepts the bounded M7A literal, image, and shaped-run subset.
All accepted items use finite affine `local_to_surface` transforms; literal,
image, and shaped-run items may also carry the existing conjunctive clip
semantics. FillRects preserve literal color
alpha and validated item opacity. A transform whose canonical
`LogicalTransform::inverse()` is unavailable contributes no paint coverage rather
than falling back to the source rectangle. For an invertible transform, finite
f32 scene components are widened only inside the renderer to f64 for affine edge
construction and raster scaling. The resulting convex polygon is clipped to the
exact continuous raster canvas defined by `logical_size * RasterScale` before
conversion to the GPU f32 vertex ABI and triangulation. The integer texture
extent is the ceil-rounded storage/readback extent only; fractional-scale padding
never becomes logical paint coverage. This preserves a visible canvas
intersection even when an irrelevant remote forward `LogicalPoint` mapping would
overflow f32, without replacing transformed geometry with an axis-aligned
bounding box. Unknown primitives remain unsupported. Canonical
`SceneRequirements`/`SceneCapabilities`
continue to check resource kinds only; the narrower implementation-subset
validation and its detailed reasons remain renderer-internal. The public render
error reports an unsupported scene without making temporary implementation
progress part of the lasting API.

An offscreen target retains its texture, extent, format, and successful
publication lineage as one realization. `AlreadyCurrent` and `ExactBaseMatch`
are available only against that live target. Explicit loss, first creation, or
extent/format rebuild starts with empty lineage and forces `FullResync`. A
post-submission failure conservatively drops the target; validation failures do
not mutate it. `discard_offscreen_target` makes this lifetime boundary explicit.

Offscreen publication targets initialize to transparent black. Initialization
is target policy, not authored `PaintPublication` paint. Successful target
lineage is committed only after actual GPU submission and CPU readback complete.
The renderer returns raw GPU-derived RGBA8 sRGB bytes and an immutable
`PublicationObservation` containing publication/update/damage facts, logical and
physical extent, scale, backend/format, resource lookup/cache outcomes, and
stage results for render, readback, and present. `ResourceRenderer::last_observation`
also exposes the most recent failed publication attempt: preflight failures mark
the affected resource as `Failed` and leave render/readback as
`NotAttempted`; a post-submit map failure marks render as `Succeeded` and
readback as `Failed`. Offscreen rendering has no present stage. A deterministic
real-adapter readback failure cannot be injected without production test hooks
or fake backend behavior, so M7A proves the ordinary preflight and validation
failure boundaries and keeps the post-submit failure mapping explicit without
polluting production state for tests. PNG encoding and golden comparison belong
to proof tooling. The M7A corpus checks a checked-in PNG with the same real-wgpu
resource path; the comparator is exact and does not substitute a software or
noop renderer.

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

### M7A resources

Image payloads are caller-owned, non-zero, unpremultiplied RGBA8 sRGB sources.
The complete image maps to its declared logical destination. The renderer owns
only the disposable sampled-texture realization and never decodes PNG data.
The checked-in `provider_image.png` corpus is decoded by the test provider with
`image`'s PNG-only, defaults-disabled path, explicitly normalized to straight
RGBA8 bytes, and then passed through the same public `ImagePayload` seam.

Shaped text is retained by the runtime as one immutable logical resource. The
renderer resolves that exact resource from `PaintPublication`, extracts each
unique already-shaped glyph outline with Skrifa, generates per-glyph MSDF fields
with `bymsdfgen-core`, packs them into deterministic resource-local atlas pages,
and owns only the disposable atlas/device realization. A private representation
quality class selects among the current renderer tiers; it is not part of
`ResourceRef`, text shaping, or runtime/publication contracts. The shader samples
filterable `Rgba8Unorm` RGB MSDF data, reconstructs coverage using the field range
and projected texel footprint, and applies scene-owned foreground color and
opacity through the same linear source-over target path as literal paint.
The caller-owned `ResourceProvider` remains limited to external resources such as
images. Color and bitmap formats, SVG, faux bold, and invalid fonts/outlines
produce explicit diagnostics; whitespace and other valid non-painting glyphs
produce no atlas field or draw quad; supported outline glyphs never fall back to
an alpha-raster path. Atlas and cache state can be discarded and reconstructed
from the retained logical resource. The fixture in `tests/fixtures` uses the
bundled redistributable Cantarell font and the production runtime text system for
shaping, font binding, and retention.

The real-GPU scale proof renders identical 64x48 logical two-rectangle geometry
at scales 1.0 and 2.0, producing 64x48 and 128x96 targets with corresponding
background, rectangle-only, and overlap probes. Scale changes target
realization, never logical geometry. An adapter-independent fractional-scale
regression additionally proves that the production affine path clips at the
exact continuous raster canvas while the texture independently rounds up for
storage. PNG round-trip proof uses the 2.0 output. An independent test-only scalar
oracle checks selected translucent interior pixels without traversing scenes or
rasterizing geometry. Opaque, clear, and zero-opacity probes remain exact;
translucent probes allow at most one byte per channel for f64 oracle versus GPU
f32 blend and UNORM-storage rounding.

The affine proof uses an invertible non-axis-aligned shear/translation and a
singular transform in the same ordinary publication. Exact interior probes show
the transformed parallelogram is rasterized as authored, while a point inside its
axis-aligned bounding box but outside the parallelogram stays transparent. Probes
at the untransformed source rectangle and the singular rectangle also stay
transparent, proving neither transform path falls back to source geometry. A
separate extreme-affine regression proves that an invertible transform retains
its visible canvas intersection when an ordinary forward f32 `LogicalPoint`
mapping of a remote corner is unrepresentable. Adapter-independent tests exercise
the same production polygon construction/clipping helper, so adapterless hosts
still validate non-axis-aligned geometry, singular noncoverage, extreme-coordinate
retention, and fractional-scale canvas bounds; they do not substitute for the
real-GPU pixel corpus.

The authored rectangles in the existing PNG proof remain opaque. Translucent
target readback contains accumulated composited RGB, while ordinary PNG alpha is
unassociated; no translucent PNG visual or golden-comparison claim is made
without test tooling that explicitly decodes target sRGB, unpremultiplies in
linear space when alpha is nonzero, and re-encodes straight RGB to sRGB.

The package must not become UI behavior authority. In particular it must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Logical resource lookup remains caller-owned and keyed by the complete opaque `ResourceRef`; renderer caches are disposable realization state.
