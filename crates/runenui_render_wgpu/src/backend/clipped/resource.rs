use std::collections::HashSet;

use runenui_core::{Color, PaintPrimitive, ResourceKind, ResourceRef};
use runenui_runtime::{PaintPublication, RasterScale, SceneCapabilities, SceneClip};
use wgpu::util::DeviceExt;

use crate::{
    PublicationUpdateMode, PublicationUpdatePlan, ResourceProvider, ResourceResolveError,
    WgpuHasDisplayHandle,
    scene_subset::{SceneValidationError, validate_literal_rect_item},
};

use super::super::{
    OFFSCREEN_FORMAT, OffscreenExtent, OffscreenPublicationReadback, OffscreenReadback,
    OffscreenRenderError, RasterCanvasExtent, ReadbackLayout, RendererDiagnostics,
    RendererInitError, RendererOptions, encode_target_copy, publication_extents,
    scene_validation_error,
};
use super::{
    ClipTargetPipelines, LiteralRectItem, Renderer, apply_clip_mask, clear_color_target,
    clear_stencil_mask, create_stencil_target, draw_clipped_fill, draw_unclipped_fill, image,
    prepare_clip_uniforms, stroke_mask,
};

/// Structured failure while realizing one provider-backed paint publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationRenderError {
    /// Existing renderer/device/target/readback failure.
    Backend(OffscreenRenderError),
    /// Caller-owned logical resource resolution failed before target mutation.
    Resource {
        item_index: usize,
        error: ResourceResolveError,
    },
    /// An otherwise valid image payload exceeds this renderer device's texture limit.
    ImageExtentExceedsDeviceLimit {
        item_index: usize,
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    },
    /// The tightly packed RGBA8 row byte count cannot be represented by wgpu's upload layout.
    ImageRowBytesOverflow { item_index: usize, width: u32 },
}

impl core::fmt::Display for PublicationRenderError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Backend(error) => error.fmt(formatter),
            Self::Resource { item_index, error } => write!(
                formatter,
                "renderer failed to resolve image resource for scene item {item_index}: {error}"
            ),
            Self::ImageExtentExceedsDeviceLimit {
                item_index,
                width,
                height,
                max_texture_dimension_2d,
            } => write!(
                formatter,
                "renderer image resource for scene item {item_index} has extent {width}x{height}, exceeding device 2D texture limit {max_texture_dimension_2d}"
            ),
            Self::ImageRowBytesOverflow { item_index, width } => write!(
                formatter,
                "renderer image resource for scene item {item_index} has width {width}, whose tightly packed RGBA8 row byte count overflows u32"
            ),
        }
    }
}

impl core::error::Error for PublicationRenderError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::Resource { error, .. } => Some(error),
            Self::ImageExtentExceedsDeviceLimit { .. } | Self::ImageRowBytesOverflow { .. } => None,
        }
    }
}

impl From<OffscreenRenderError> for PublicationRenderError {
    fn from(error: OffscreenRenderError) -> Self {
        Self::Backend(error)
    }
}

/// Canonical provider-aware renderer facade.
///
/// The already-proven literal renderer remains private implementation machinery
/// and continues to own the single wgpu instance/device/queue/target/lineage.
/// Image upload, bind groups, and sampled textures are disposable child caches
/// keyed only by the complete opaque `ResourceRef`.
#[derive(Debug)]
pub struct ResourceRenderer {
    literal: Renderer,
    images: image::ImageRenderer,
}

impl ResourceRenderer {
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request(options: RendererOptions) -> Result<Self, RendererInitError> {
        Renderer::request(options).await.map(Self::from_literal)
    }

    /// Selects a native adapter using a caller-owned display connection.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request_with_display_handle(
        options: RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, RendererInitError> {
        Renderer::request_with_display_handle(options, display)
            .await
            .map(Self::from_literal)
    }

    /// Creates and retains a native surface before selecting a compatible adapter.
    ///
    /// # Errors
    ///
    /// Returns structured surface-creation, compatible-adapter, target-format, or
    /// device diagnostics when construction fails.
    pub async fn request_with_surface_target(
        options: RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, RendererInitError> {
        Renderer::request_with_surface_target(options, display, window)
            .await
            .map(Self::from_literal)
    }

    fn from_literal(literal: Renderer) -> Self {
        let images = image::ImageRenderer::new(&literal.base.device);
        Self { literal, images }
    }

    /// Returns immutable instance, adapter, device, and target diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &RendererDiagnostics {
        self.literal.diagnostics()
    }

    /// Returns whether construction retained an actual native surface target.
    #[must_use]
    pub const fn has_surface(&self) -> bool {
        self.literal.has_surface()
    }

    /// Drops the retained offscreen target and every publication realization tied to it.
    #[must_use]
    pub fn discard_offscreen_target(&mut self) -> bool {
        self.literal.discard_offscreen_target()
    }

    /// Drops renderer-owned uploaded resource realizations without changing logical refs.
    ///
    /// A real cache loss also invalidates successful publication lineage so the
    /// next complete publication is reconstructed with a full resync.
    #[must_use]
    pub fn discard_resource_cache(&mut self) -> bool {
        let discarded = self.images.discard_cache();
        if discarded
            && let Some(target) = self.literal.base.offscreen_target.as_mut()
        {
            target.lineage.reset();
        }
        discarded
    }

    /// Renders one complete provider-backed publication and reads actual GPU bytes.
    ///
    /// Publications without images delegate to the already-proven literal renderer.
    /// Image-bearing publications preserve exact scene order across fills, centered
    /// strokes, and images; image payload resolution completes before retained-target
    /// mutation, and `ShapedTextRun` remains fail-closed until its own bounded slice.
    ///
    /// # Errors
    ///
    /// Returns deterministic scene, resource, image-limit, target, device, or
    /// readback failures. A missing/unavailable/malformed provider result never
    /// mutates the retained target.
    #[allow(
        clippy::too_many_lines,
        reason = "the provider-backed render transaction intentionally keeps complete validation and resource preflight before target mutation, then ordered realization/submission/readback, and only then lineage/cache commit in one auditable sequence"
    )]
    pub fn render_offscreen_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
    ) -> Result<OffscreenPublicationReadback, PublicationRenderError> {
        if !publication
            .scene()
            .items()
            .iter()
            .any(|item| matches!(item.primitive(), PaintPrimitive::Image(_)))
        {
            let result = self
                .literal
                .render_offscreen_publication(publication)
                .map_err(PublicationRenderError::Backend);
            if result.is_ok() {
                self.images.discard_cache();
            }
            return result;
        }

        let scene = validate_resource_scene_subset(publication).map_err(scene_validation_error)?;
        let (canvas_extent, extent) = publication_extents(publication)?;
        self.literal.base.validate_extent(extent)?;
        let layout = ReadbackLayout::new(extent)?;
        self.literal.base.validate_readback_buffer(layout)?;

        let retained_target_matches = self
            .literal
            .base
            .offscreen_target
            .as_ref()
            .is_some_and(|target| target.matches(extent, OFFSCREEN_FORMAT));
        let update_plan = if retained_target_matches {
            self.literal
                .base
                .offscreen_target
                .as_ref()
                .map_or_else(PublicationUpdatePlan::full_resync, |target| {
                    target.lineage.plan(publication)
                })
        } else {
            PublicationUpdatePlan::full_resync()
        };

        let has_literals = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::Literal(_)));
        let needs_stencil = scene.iter().any(ResourceSceneItem::needs_stencil);
        let live_images = live_image_resources(&scene);

        if update_plan.mode() != PublicationUpdateMode::AlreadyCurrent {
            let resolved = self.preflight_images(&scene, provider)?;
            if has_literals {
                self.literal
                    .base
                    .ensure_fill_rect_pipeline(OFFSCREEN_FORMAT)?;
            }
            if needs_stencil {
                self.literal.ensure_clip_pipelines(OFFSCREEN_FORMAT)?;
            }
            self.images
                .ensure_pipelines(&self.literal.base.device, OFFSCREEN_FORMAT)?;
            self.images.realize(
                &self.literal.base.device,
                &self.literal.base.queue,
                resolved,
            );

            if !retained_target_matches {
                let target = self.literal.base.create_offscreen_target(extent)?;
                self.literal.base.offscreen_target = Some(target);
            }
        }

        let readback = self.literal.base.create_readback_buffer(layout);
        let mut encoder =
            self.literal
                .base
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("runenui provider-backed offscreen publication encoder"),
                });
        let target = self
            .literal
            .base
            .offscreen_target
            .as_ref()
            .unwrap_or_else(|| unreachable!("matching or newly created target is retained"));
        let target_generation = target.generation;

        let stencil_target = (update_plan.mode() != PublicationUpdateMode::AlreadyCurrent
            && needs_stencil)
            .then(|| create_stencil_target(&self.literal.base.device, extent));
        if update_plan.mode() != PublicationUpdateMode::AlreadyCurrent {
            let ordinary_pipeline = has_literals.then(|| {
                self.literal
                    .base
                    .fill_rect_pipelines
                    .get(&target.format)
                    .unwrap_or_else(|| unreachable!("ordinary target pipeline is cached"))
            });
            let clip_pipelines = needs_stencil.then(|| {
                self.literal
                    .clip_pipelines
                    .get(&target.format)
                    .unwrap_or_else(|| unreachable!("literal mask pipelines are cached"))
            });
            encode_resource_scene_to_target(
                &self.literal.base.device,
                ordinary_pipeline,
                clip_pipelines,
                &self.images,
                &mut encoder,
                &target.view,
                stencil_target.as_ref().map(|(_, view)| view),
                target.format,
                extent,
                canvas_extent,
                publication.raster_scale(),
                &scene,
            );
        }

        encode_target_copy(&mut encoder, &target.texture, &readback, extent, layout);
        let submission = self.literal.base.queue.submit([encoder.finish()]);
        let rgba8_srgb = match self
            .literal
            .base
            .map_readback(&readback, layout, submission)
        {
            Ok(pixels) => pixels,
            Err(error) => {
                self.literal.base.offscreen_target = None;
                return Err(PublicationRenderError::Backend(error));
            }
        };
        let readback = OffscreenReadback {
            extent,
            format: OFFSCREEN_FORMAT,
            rgba8_srgb,
        };

        self.literal
            .base
            .offscreen_target
            .as_mut()
            .unwrap_or_else(|| unreachable!("successful readback retains its target"))
            .lineage
            .record_success(publication);
        self.images.retain(&live_images);
        Ok(OffscreenPublicationReadback {
            update_plan,
            target_generation,
            readback,
        })
    }

    fn preflight_images<P: ResourceProvider + ?Sized>(
        &self,
        scene: &[ResourceSceneItem],
        provider: &P,
    ) -> Result<Vec<image::ResolvedImage>, PublicationRenderError> {
        let max_texture_dimension_2d = self
            .literal
            .diagnostics()
            .device_limits()
            .max_texture_dimension_2d;
        let mut seen = HashSet::new();
        let mut resolved = Vec::new();
        for item in scene {
            let ResourceSceneItem::Image(item) = item else {
                continue;
            };
            if self.images.contains(&item.image.resource)
                || !seen.insert(item.image.resource.clone())
            {
                continue;
            }
            match image::resolve_image(provider, &item.image, max_texture_dimension_2d) {
                Ok(image) => resolved.push(image),
                Err(image::ImageResolveFailure::Resource(error)) => {
                    return Err(PublicationRenderError::Resource {
                        item_index: item.image.item_index,
                        error,
                    });
                }
                Err(image::ImageResolveFailure::ExtentExceedsDeviceLimit {
                    width,
                    height,
                    max_texture_dimension_2d,
                }) => {
                    return Err(PublicationRenderError::ImageExtentExceedsDeviceLimit {
                        item_index: item.image.item_index,
                        width,
                        height,
                        max_texture_dimension_2d,
                    });
                }
                Err(image::ImageResolveFailure::RowBytesOverflow { width }) => {
                    return Err(PublicationRenderError::ImageRowBytesOverflow {
                        item_index: item.image.item_index,
                        width,
                    });
                }
            }
        }
        Ok(resolved)
    }

    /// Executes one real wgpu render-pass clear and returns actual GPU bytes from GPU readback.
    ///
    /// # Errors
    ///
    /// Returns structured extent, device-wait, buffer-map, or mapped-range failures.
    pub fn clear_offscreen(
        &self,
        extent: OffscreenExtent,
        color: Color,
    ) -> Result<OffscreenReadback, OffscreenRenderError> {
        self.literal.clear_offscreen(extent, color)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ResourceSceneItem {
    Literal(LiteralRectItem),
    Image(ImageSceneItem),
}

impl ResourceSceneItem {
    const fn needs_stencil(&self) -> bool {
        match self {
            Self::Literal(item) => item.literal.stroke_inset.is_some() || !item.clips.is_empty(),
            Self::Image(item) => !item.clips.is_empty(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ImageSceneItem {
    image: image::SupportedImage,
    clips: Vec<SceneClip>,
}

fn validate_resource_scene_subset(
    publication: &PaintPublication,
) -> Result<Vec<ResourceSceneItem>, SceneValidationError> {
    let requirements = publication.scene().requirements();
    let capabilities = SceneCapabilities::new([ResourceKind::Image]);
    let unsupported_resource_kind =
        capabilities
            .check_requirements(&requirements)
            .err()
            .map(|error| SceneValidationError::UnsupportedResourceKind {
                resource_kind: error.resource_kind(),
            });
    let mut items = Vec::with_capacity(publication.scene().items().len());
    for (item_index, item) in publication.scene().items().iter().enumerate() {
        if let PaintPrimitive::Image(image_primitive) = item.primitive() {
            items.push(ResourceSceneItem::Image(ImageSceneItem {
                image: image::SupportedImage {
                    item_index,
                    resource: image_primitive.resource_ref().clone(),
                    destination: image_primitive.destination(),
                    opacity: item.opacity(),
                    local_to_surface: item.local_to_surface(),
                },
                clips: item.clips().to_vec(),
            }));
            continue;
        }
        if let Some(literal) = validate_literal_rect_item(item_index, item)? {
            items.push(ResourceSceneItem::Literal(LiteralRectItem {
                literal,
                clips: item.clips().to_vec(),
            }));
        }
    }
    if let Some(error) = unsupported_resource_kind {
        return Err(error);
    }
    Ok(items)
}

fn live_image_resources(scene: &[ResourceSceneItem]) -> HashSet<ResourceRef> {
    scene
        .iter()
        .filter_map(|item| match item {
            ResourceSceneItem::Image(item) => Some(item.image.resource.clone()),
            ResourceSceneItem::Literal(_) => None,
        })
        .collect()
}

#[allow(
    clippy::too_many_arguments,
    reason = "the ordered mixed-scene encoder keeps the single device, exact target/canvas/scale, optional stencil realization, and established literal/image pipeline authorities explicit"
)]
fn encode_resource_scene_to_target(
    device: &wgpu::Device,
    ordinary_pipeline: Option<&wgpu::RenderPipeline>,
    clip_pipelines: Option<&ClipTargetPipelines>,
    image_renderer: &image::ImageRenderer,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: Option<&wgpu::TextureView>,
    target_format: wgpu::TextureFormat,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
    scene: &[ResourceSceneItem],
) {
    clear_color_target(encoder, color_view);
    for item in scene {
        match item {
            ResourceSceneItem::Literal(item) => encode_resource_literal_item(
                device,
                ordinary_pipeline,
                clip_pipelines,
                encoder,
                color_view,
                stencil_view,
                extent,
                canvas_extent,
                raster_scale,
                item,
            ),
            ResourceSceneItem::Image(item) => encode_resource_image_item(
                device,
                clip_pipelines,
                image_renderer,
                encoder,
                color_view,
                stencil_view,
                target_format,
                extent,
                canvas_extent,
                raster_scale,
                item,
            ),
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "mixed-scene literal dispatch reuses the established exact rectangle/stroke/clip helpers while keeping its already-validated target realization inputs explicit"
)]
fn encode_resource_literal_item(
    device: &wgpu::Device,
    ordinary_pipeline: Option<&wgpu::RenderPipeline>,
    clip_pipelines: Option<&ClipTargetPipelines>,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: Option<&wgpu::TextureView>,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
    item: &LiteralRectItem,
) {
    let vertex_bytes = super::super::fill_rect_vertex_bytes(
        std::slice::from_ref(&item.literal.fill),
        extent,
        canvas_extent,
        raster_scale,
    );
    if vertex_bytes.is_empty() {
        return;
    }
    let vertex_count = u32::try_from(vertex_bytes.len() / 24).unwrap_or(u32::MAX);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui ordered mixed-scene literal vertices"),
        contents: &vertex_bytes,
        usage: wgpu::BufferUsages::VERTEX,
    });

    if item.literal.stroke_inset.is_none() && item.clips.is_empty() {
        draw_unclipped_fill(
            encoder,
            color_view,
            ordinary_pipeline
                .unwrap_or_else(|| unreachable!("literal pipeline exists for literal scene items")),
            &vertex_buffer,
            vertex_count,
        );
        return;
    }

    let Some(clip_uniforms) = prepare_clip_uniforms(&item.clips, raster_scale) else {
        return;
    };
    let stroke_uniform = if item.literal.stroke_inset.is_some() {
        let Some(uniform) =
            stroke_mask::StrokeMaskUniform::from_literal(&item.literal, raster_scale)
        else {
            return;
        };
        Some(uniform)
    } else {
        None
    };
    let stencil_view =
        stencil_view.unwrap_or_else(|| unreachable!("masked literal item requires stencil target"));
    let clip_pipelines = clip_pipelines
        .unwrap_or_else(|| unreachable!("masked literal item requires mask pipelines"));
    clear_stencil_mask(encoder, stencil_view);
    if let Some(uniform) = stroke_uniform {
        stroke_mask::apply_stroke_mask(
            device,
            encoder,
            stencil_view,
            &clip_pipelines.stroke_mask,
            uniform,
        );
    }
    for uniform in &clip_uniforms {
        apply_clip_mask(device, encoder, stencil_view, &clip_pipelines.mask, uniform);
    }
    draw_clipped_fill(
        encoder,
        color_view,
        stencil_view,
        &clip_pipelines.clipped_fill,
        &vertex_buffer,
        vertex_count,
    );
}

#[allow(
    clippy::too_many_arguments,
    reason = "mixed-scene image dispatch keeps exact geometry, target format, optional stencil realization, and cached resource identity explicit at the sampled-image draw boundary"
)]
fn encode_resource_image_item(
    device: &wgpu::Device,
    clip_pipelines: Option<&ClipTargetPipelines>,
    image_renderer: &image::ImageRenderer,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: Option<&wgpu::TextureView>,
    target_format: wgpu::TextureFormat,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
    item: &ImageSceneItem,
) {
    let vertex_bytes = image::vertex_bytes(&item.image, extent, canvas_extent, raster_scale);
    if vertex_bytes.is_empty() {
        return;
    }
    let vertex_count = u32::try_from(vertex_bytes.len() / 20).unwrap_or(u32::MAX);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui ordered mixed-scene image vertices"),
        contents: &vertex_bytes,
        usage: wgpu::BufferUsages::VERTEX,
    });

    if item.clips.is_empty() {
        image_renderer.draw(
            target_format,
            encoder,
            color_view,
            None,
            &item.image.resource,
            &vertex_buffer,
            vertex_count,
        );
        return;
    }

    let Some(clip_uniforms) = prepare_clip_uniforms(&item.clips, raster_scale) else {
        return;
    };
    let stencil_view =
        stencil_view.unwrap_or_else(|| unreachable!("clipped image requires stencil target"));
    let clip_pipelines =
        clip_pipelines.unwrap_or_else(|| unreachable!("clipped image requires mask pipelines"));
    clear_stencil_mask(encoder, stencil_view);
    for uniform in &clip_uniforms {
        apply_clip_mask(device, encoder, stencil_view, &clip_pipelines.mask, uniform);
    }
    image_renderer.draw(
        target_format,
        encoder,
        color_view,
        Some(stencil_view),
        &item.image.resource,
        &vertex_buffer,
        vertex_count,
    );
}
