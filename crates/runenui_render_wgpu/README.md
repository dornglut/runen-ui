# `runenui_render_wgpu`

> **Category: Library reference**

`runenui_render_wgpu` is RunenUI's reusable concrete wgpu renderer edge over ordinary public paint publications. It owns disposable GPU realization and target state; it does not own widget behavior, semantic identity, mounted/runtime authority, text shaping, logical layout, logical visual/composition semantics, or a native event loop.

The public `Renderer` consumes the accepted `runenui_core`, `runenui_runtime`, and exact retained `runenui_text` shaped-resource contracts required by paint realization. Native hosts such as `reference_winit` and Counter keep window/event-loop policy outside this crate while using the same renderer for real native presentation.

## Ownership

The renderer owns:

- wgpu instance, adapter, device, queue, offscreen targets, and retained native surface state;
- renderer-local target generations and successful-publication lineage;
- validation and disposable realization of the currently supported paint-scene subset;
- renderer-private Lyon tessellation for accepted generic fill/stroke coverage without making Lyon identity/tolerance/defaults public semantics;
- disposable gradient buffers/shaders, generic clip stencil realization, isolated composition-group targets, and ordinary-shadow alpha-mask/GPU resources derived from runtime-published neutral facts;
- bounded renderer-private ordinary-shadow mask allocation/lifetime accounting, Euclidean spread/erosion work, finite Gaussian workspaces, and exact tight final mask crops;
- caller-provided external image realization/cache state keyed by complete opaque `ResourceRef` values;
- disposable per-glyph SDF/MSDF generation, quality selection, atlas pages, textures, pipelines, and shaders for exact retained shaped-text resources;
- native surface configuration/acquisition/render/present mechanics and offscreen GPU readback;
- immutable renderer observations covering publication/update, target, resource, render, readback, and present stages.

The renderer does **not** own native event loops, application lifecycle policy, AccessKit, semantic trees/actions, mounted/layout storage, style resolution, node decoration/group ordering, logical path containment/bounds, image fit policy, ordinary-shadow support semantics/painter order, font discovery, shaping, line breaking, logical text identity, or application resource identity.

## Construction and native presentation

`Renderer::request` constructs a headless renderer. `Renderer::request_with_display_handle` supplies an owned display connection without creating a surface. `Renderer::request_with_surface_target` creates and retains a native surface before selecting a compatible adapter while remaining independent of winit itself.

A native host then explicitly drives the retained target:

1. `configure_surface` establishes a non-zero physical extent and renderer-local target generation;
2. the runtime publishes one ordinary `PaintPublication` for the host's exact logical surface and raster scale;
3. `render_surface_publication` performs validation/resource/effect preflight, acquires the native surface texture, encodes the same accepted mixed scene used by the offscreen path, submits GPU work, invokes the caller-owned pre-present boundary, presents, and only then commits successful surface lineage;
4. timeout, occlusion, outdated/suboptimal configuration, or surface loss are returned as structured host-visible errors so the host can retry, reconfigure, or recreate the renderer without moving UI authority into this crate.

The host remains responsible for window/event-loop ownership, mapping physical size and scale into RunenUI logical/raster facts, redraw/publication acknowledgement, retry policy, and renderer recreation. The renderer remains winit-free.

Surface creation follows wgpu platform requirements, including main-thread creation where required. Native surface formats are selected only from the accepted sRGB formats advertised by the compatible adapter; unsupported or empty format sets fail structurally rather than changing the color contract.

## Supported scene and resource path

The implementation fails closed before target mutation or GPU submission when a publication cannot be represented by the current renderer subset. Supported production realization includes accepted generic `SceneShape` fills/strokes for rectangle, rounded rectangle, ellipse, and path geometry; solid, linear-gradient, and concentric-radial brushes; runtime-resolved image fit/crop/alignment/nine-slice patches; retained shaped-text runs; finite affine transforms; conjunctive generic clips; snapshot-local atomic composition groups; ordinary shadows; scene/group opacity; and ordered source-over composition. Unknown or unsupported primitives/effects remain explicit failures rather than being reinterpreted by the renderer.

Canonical `SceneRequirements` / `SceneCapabilities` remain renderer-neutral runtime contracts. Narrow renderer implementation checks and detailed rejection reasons stay renderer-local and do not become a second scene vocabulary.

External image payloads are caller-owned non-zero, tightly packed, unpremultiplied RGBA8 sRGB sources. The complete opaque `ResourceRef` is the provider/cache identity. Runtime-published intrinsic dimensions must match the payload extent exactly; the renderer never chooses fit policy, silently rescales mismatched metadata, or remints a resource identity. Production PNG decoding remains outside the renderer; fixture PNG decoding belongs only to test providers.

The caller-owned `ResourceProvider` resolves external resources such as images. Runtime-shaped text does not round-trip through that provider: the exact immutable shaped-resource binding is retained by `PaintPublication` and consumed directly by the renderer.

## Shaped text

RunenUI logical text authority remains outside the renderer. Runtime publication retains the exact scale-independent `ResourceRef -> ShapedTextResource` binding produced by the accepted text/layout path. The renderer consumes that already-shaped resource, resolves one renderer-private exact scalable outline interpretation with Skrifa, generates per-glyph MSDF fields with `bymsdfgen-core`, packs deterministic renderer-local atlas pages, and reconstructs coverage in the GPU shader.

That same exact outline interpretation may be projected into disposable neutral-support geometry when an ordinary shadow actually consumes shaped-text support. Shadow support never derives from text foreground alpha, MSDF samples, atlas coverage, raster scale, or cache/device state.

Raster scale and renderer quality affect only disposable realization. They do not change text content, line breaking, glyph selection, logical metrics, `ResourceRef` identity, runtime layout, or shadow-support semantics. Resource/atlas cache loss can therefore be reconstructed from the retained logical publication without a runtime republish, provider lookup, reshaping, re-line-breaking, or resource remint.

Supported outline glyphs never silently fall back to alpha-raster text. COLR, SVG, bitmap/intrinsic-color glyphs, faux-bold requirements, invalid font data, and invalid outlines produce explicit structured diagnostics unless a future separately accepted resource/paint contract represents them truthfully.

## Geometry, brushes, color, clipping, and groups

Accepted scene geometry and transforms remain RunenUI-owned logical facts. Logical path validation, fill containment, structural equality, deterministic bounds, point-degenerate behavior, stroke cap/join/tangent semantics, miter fallback, and zero-width/zero-extent rules are defined outside the renderer. Private Lyon tessellation is a disposable decomposition constrained by those semantics; dependency flattening/defaults do not become public logical authority.

Non-invertible item geometry contributes no paint coverage rather than falling back to source rectangles. Non-invertible/empty clips exclude coverage rather than disappearing. Renderer-internal raster construction may widen finite scene components for robust clipping/math, but the public logical geometry remains unchanged.

The exact continuous raster canvas is `logical_size * RasterScale`; integer texture extents are ceil-rounded storage/readback extents only. Fractional-scale padding does not become logical paint coverage.

Solid and gradient inputs retain RunenUI's accepted straight-alpha sRGB8 / premultiplied-linear interpolation contract. Gradient stop order, hard-stop side semantics, endpoint extension, linear/radial logical geometry, and brush selection are framework semantics; the renderer only realizes the already-accepted brush. One logical fill/stroke item remains one compositing source even when private tessellation triangles overlap internally.

Clipped fills, strokes, images, shaped runs, groups, and shadows use renderer-owned stencil/offscreen/mask resources while preserving runtime-authored transforms, exact M6-derived painter order, conjunctive clip semantics, group nesting/first-member contraction, authored shadow order, exact-once group opacity, and parent source-over.

## Ordinary shadows and neutral support

Ordinary shadows are realized from ADR 0015 neutral effect support, not accumulated RGBA alpha. Runtime-published support facts preserve accepted fill/stroke geometry, resolved image destination patches, exact retained shaped-text outline support, nested group/effect support, transforms/clips, and independent sibling-shadow derivation from one shared pre-shadow source.

Renderer realization applies the accepted Euclidean signed spread (disk dilation/erosion), then offset, then finite `3 * sigma` Gaussian support. Off-surface source/support is retained until downstream effect derivation and final-canvas cropping, so an effect that reaches the target is not lost merely because its source did not initially intersect it. Shadows paint in authored order behind composed child color; group clips constrain the completed group result/effects; group opacity is applied once before parent source-over.

Mask storage/work is private disposable state. Admission accounts deterministic requested live payloads and inherited recursive residency rather than a scalar bytes-per-pixel approximation; distance-transform and morphology/blur scratch are bounded/reused; transformed support geometry is streamed; final masks own exact tight `Vec<u8>` storage. Allocation-policy rejection is structured and occurs before resource loading, render observation/transaction start, target allocation/mutation, or final-canvas crop. Those limits are renderer implementation policy and do not become public shadow semantics or a scene-wide memory authority.

## Target lineage, retry, and cache loss

One retained offscreen target owns its texture, extent, format, renderer-local generation, and successful publication lineage. Target loss/recreation or extent/format changes reset that lineage and require full resynchronization. Pre-submission validation/resource/effect-realization failures do not mutate the retained target. Post-submission failures conservatively invalidate target state where correctness requires it.

Native surface reconfiguration similarly creates a new renderer-local target generation and resets successful surface-publication lineage while allowing disposable resource caches to remain reusable when valid.

`discard_offscreen_target` explicitly drops offscreen target realization. `discard_resource_cache` drops renderer-owned external-image and shaped-text realizations and invalidates successful target lineage so the next complete publication reconstructs resources through the ordinary production path. Generic geometry, clip, group, gradient, and shadow realization is likewise reconstructible from the complete neutral publication and retained/caller-owned resource bindings rather than a retained renderer scene tree.

Renderer observations are evidence of renderer-local work only; they do not replace runtime trace/publication authority.

## Evidence

Repository tests exercise the real wgpu path rather than a software expected renderer. Current evidence includes:

- real offscreen readback and checked-in PNG/golden coverage for accepted scene/resource behavior;
- generic shape/path fill/stroke, degenerate geometry, gradient, image mapping, generic clip, transform, opacity/source-over, fractional raster-scale, target-lineage, and reconstruction regressions;
- atomic/nested composition-group proof covering runtime-published order, group clips, exact-once opacity, resources, and rebuild;
- ordinary-shadow proof covering neutral alpha-independent support, transparent child/image/text cases, authored sibling painter order, Euclidean-vs-square spread, quarter-turn invariance, nested effect support, complete erosion, off-surface reach, bounded mask allocation, and deterministic zero-blur coverage;
- shaped-text SDF/MSDF realization, raster-scale changes, retained-publication retry, resource-cache re-realization, and explicit intrinsic-format diagnostics;
- the M8D responsive multiscript corpus, which regenerates a compact human-inspectable contact sheet when a real wgpu adapter is available while keeping automated logical/resource assertions authoritative.

Adapter-independent tests may exercise the same production geometry/effect helpers for deterministic edge cases; they supplement rather than replace real-wgpu evidence.

## Boundaries and current limitations

The package must not become UI behavior authority. It must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Renderer caches, masks, intermediate targets, tessellation and device resources remain disposable derived state.

M9A static visual/composition realization is accepted current behavior. Deterministic transition/timeline sampling and reduced-motion behavior remain runtime/framework M9B concerns, while M9C owns integrated visual-motion closure; this renderer must not invent either. Current supported text realization is outline SDF/MSDF; intrinsic COLR/SVG/bitmap rendering remains unsupported with explicit diagnostics. Broader device-loss/platform breadth belongs to later platform work, not to alternate visual/text/layout/semantic authority inside this renderer.

Exact public signatures and error variants are authoritative in source/Rustdoc. Conceptual cross-crate ownership is summarized in [`docs/architecture/public-api.md`](../../docs/architecture/public-api.md), and current accepted maturity is owned by [`docs/status.md`](../../docs/status.md).
