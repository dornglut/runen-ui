mod group;
mod solid;

use std::collections::HashSet;

use runenui_core::{Color, LogicalSize, PaintPrimitive, ResourceKind, ResourceRef};
use runenui_runtime::{PaintPublication, RasterScale, SceneCapabilities};
use wgpu::util::DeviceExt;

use crate::{
    PublicationUpdateMode, PublicationUpdatePlan, ResourceProvider, ResourceResolveError,
    WgpuHasDisplayHandle,
    lineage::PublicationLineage,
    observation::{ResourceCacheOutcome, ResourceObservation, ResourceRealizationKind},
    scene_subset::{SceneValidationError, UnsupportedSceneSemantic},
};

use super::super::{
    OFFSCREEN_FORMAT, OffscreenExtent, OffscreenPublicationReadback, OffscreenReadback,
    OffscreenRenderError, RasterCanvasExtent, ReadbackLayout, RendererDiagnostics,
    RendererInitError, RendererOptions, encode_target_copy, publication_extents,
    scene_validation_error,
};
use super::{
    Renderer, clear_stencil_mask,
    clip::{self, ClipRenderer, PreparedClip},
    create_stencil_target, image, shaped,
};

/// Explicitly unsupported glyph source encountered during renderer-owned outline realization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedShapedGlyphKind {
    ColrV0,
    ColrV1,
    Bitmap,
    Svg,
    FauxBold,
}

/// Structured failure while realizing one retained paint publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationRenderError {
    /// Existing renderer/device/target/readback failure.
    Backend(OffscreenRenderError),
    /// Private fill/stroke geometry realization failed before target mutation.
    SolidGeometry { item_index: usize, detail: String },
    /// Private item-clip geometry realization failed before target mutation.
    ClipGeometry {
        item_index: usize,
        clip_index: usize,
        detail: String,
    },
    /// Private group-clip geometry realization failed before target mutation.
    GroupClipGeometry { clip_index: usize, detail: String },
    /// Private ordinary-shadow support/mask realization failed before target mutation.
    GroupShadowRealization { shadow_index: usize, detail: String },
    /// An accepted gradient stop list exceeds this renderer device's buffer limits.
    GradientStopBufferExceedsDeviceLimit {
        item_index: usize,
        required_bytes: u64,
        max_bytes: u64,
    },
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
    /// An otherwise valid glyph field exceeds this renderer device's texture limit.
    ShapedGlyphExtentExceedsDeviceLimit {
        item_index: usize,
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    },
    /// The publication did not retain the exact immutable logical shaped resource.
    ShapedTextResourceUnavailable { item_index: usize },
    /// The exact shaped glyph uses a source this outline-only realization does not support.
    UnsupportedShapedGlyph {
        item_index: usize,
        glyph_id: u32,
        kind: UnsupportedShapedGlyphKind,
    },
    /// The retained font bytes or face index cannot be read as a font.
    ShapedTextFontInvalid { item_index: usize },
    /// The exact retained outline could not be converted into a valid MSDF shape.
    ShapedTextOutlineInvalid { item_index: usize, glyph_id: u32 },
    /// This renderer was not constructed with a native surface target.
    SurfaceUnavailable,
    /// A retained native surface exists but has not been configured with a non-zero extent.
    SurfaceNotConfigured,
    /// Renderer-local surface target generations cannot advance without wrapping.
    SurfaceTargetGenerationExhausted,
    /// The presentation engine could not provide a frame before its timeout boundary.
    SurfaceTimeout,
    /// The native surface is currently occluded and should be retried later.
    SurfaceOccluded,
    /// The native surface configuration is outdated and must be configured again.
    SurfaceOutdated,
    /// The native surface was lost and the host must recreate the renderer/surface target.
    SurfaceLost,
    /// The acquired surface texture is usable but no longer matches the native surface optimally.
    SurfaceSuboptimal,
    /// Surface acquisition encountered a validation failure.
    SurfaceValidation,
}

impl core::fmt::Display for PublicationRenderError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Backend(error) => error.fmt(formatter),
            Self::SolidGeometry { item_index, detail } => write!(
                formatter,
                "renderer failed to realize paint geometry for scene item {item_index}: {detail}"
            ),
            Self::ClipGeometry {
                item_index,
                clip_index,
                detail,
            } => write!(
                formatter,
                "renderer failed to realize clip {clip_index} geometry for scene item {item_index}: {detail}"
            ),
            Self::GroupClipGeometry { clip_index, detail } => write!(
                formatter,
                "renderer failed to realize composition-group clip {clip_index}: {detail}"
            ),
            Self::GroupShadowRealization {
                shadow_index,
                detail,
            } => write!(
                formatter,
                "renderer failed to realize composition-group shadow {shadow_index}: {detail}"
            ),
            Self::GradientStopBufferExceedsDeviceLimit {
                item_index,
                required_bytes,
                max_bytes,
            } => write!(
                formatter,
                "renderer gradient stops for scene item {item_index} require {required_bytes} bytes, exceeding device storage-buffer limit {max_bytes}"
            ),
            Self::Resource { item_index, error } => write!(
                formatter,
                "renderer failed to resolve resource for scene item {item_index}: {error}"
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
            Self::ShapedGlyphExtentExceedsDeviceLimit {
                item_index,
                width,
                height,
                max_texture_dimension_2d,
            } => write!(
                formatter,
                "renderer shaped-text glyph for scene item {item_index} has field extent {width}x{height}, exceeding device 2D texture limit {max_texture_dimension_2d}"
            ),
            Self::ShapedTextResourceUnavailable { item_index } => write!(
                formatter,
                "renderer scene item {item_index} has no retained immutable shaped-text resource"
            ),
            Self::UnsupportedShapedGlyph {
                item_index,
                glyph_id,
                kind,
            } => write!(
                formatter,
                "renderer scene item {item_index} glyph {glyph_id} is unsupported for outline MSDF realization: {kind:?}"
            ),
            Self::ShapedTextFontInvalid { item_index } => write!(
                formatter,
                "renderer scene item {item_index} retained font bytes or face index are invalid"
            ),
            Self::ShapedTextOutlineInvalid {
                item_index,
                glyph_id,
            } => write!(
                formatter,
                "renderer scene item {item_index} glyph {glyph_id} has an invalid scalable outline"
            ),
            Self::SurfaceUnavailable => {
                formatter.write_str("renderer has no retained native surface target")
            }
            Self::SurfaceNotConfigured => {
                formatter.write_str("renderer native surface is not configured")
            }
            Self::SurfaceTargetGenerationExhausted => {
                formatter.write_str("renderer exhausted its native surface target generation space")
            }
            Self::SurfaceTimeout => formatter.write_str("native surface acquisition timed out"),
            Self::SurfaceOccluded => formatter.write_str("native surface is currently occluded"),
            Self::SurfaceOutdated => {
                formatter.write_str("native surface configuration is outdated")
            }
            Self::SurfaceLost => formatter.write_str(
                "native surface was lost and must be recreated from the host-owned window target",
            ),
            Self::SurfaceSuboptimal => formatter.write_str(
                "native surface texture is suboptimal and requires surface reconfiguration",
            ),
            Self::SurfaceValidation => {
                formatter.write_str("native surface acquisition failed validation")
            }
        }
    }
}

impl core::error::Error for PublicationRenderError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::Resource { error, .. } => Some(error),
            Self::SolidGeometry { .. }
            | Self::ClipGeometry { .. }
            | Self::GroupClipGeometry { .. }
            | Self::GroupShadowRealization { .. }
            | Self::GradientStopBufferExceedsDeviceLimit { .. }
            | Self::ImageExtentExceedsDeviceLimit { .. }
            | Self::ImageRowBytesOverflow { .. }
            | Self::ShapedGlyphExtentExceedsDeviceLimit { .. }
            | Self::ShapedTextResourceUnavailable { .. }
            | Self::UnsupportedShapedGlyph { .. }
            | Self::ShapedTextFontInvalid { .. }
            | Self::ShapedTextOutlineInvalid { .. }
            | Self::SurfaceUnavailable
            | Self::SurfaceNotConfigured
            | Self::SurfaceTargetGenerationExhausted
            | Self::SurfaceTimeout
            | Self::SurfaceOccluded
            | Self::SurfaceOutdated
            | Self::SurfaceLost
            | Self::SurfaceSuboptimal
            | Self::SurfaceValidation => None,
        }
    }
}

impl From<OffscreenRenderError> for PublicationRenderError {
    fn from(error: OffscreenRenderError) -> Self {
        Self::Backend(error)
    }
}

impl PublicationRenderError {
    const fn item_index(&self) -> Option<usize> {
        match self {
            Self::SolidGeometry { item_index, .. }
            | Self::ClipGeometry { item_index, .. }
            | Self::GradientStopBufferExceedsDeviceLimit { item_index, .. }
            | Self::Resource { item_index, .. }
            | Self::ImageExtentExceedsDeviceLimit { item_index, .. }
            | Self::ImageRowBytesOverflow { item_index, .. }
            | Self::ShapedGlyphExtentExceedsDeviceLimit { item_index, .. }
            | Self::ShapedTextResourceUnavailable { item_index }
            | Self::UnsupportedShapedGlyph { item_index, .. }
            | Self::ShapedTextFontInvalid { item_index }
            | Self::ShapedTextOutlineInvalid { item_index, .. } => Some(*item_index),
            Self::Backend(_)
            | Self::GroupClipGeometry { .. }
            | Self::GroupShadowRealization { .. }
            | Self::SurfaceUnavailable
            | Self::SurfaceNotConfigured
            | Self::SurfaceTargetGenerationExhausted
            | Self::SurfaceTimeout
            | Self::SurfaceOccluded
            | Self::SurfaceOutdated
            | Self::SurfaceLost
            | Self::SurfaceSuboptimal
            | Self::SurfaceValidation => None,
        }
    }
}

/// Canonical provider-aware renderer facade.
///
/// The wrapped renderer remains the single owner of instance/device/queue/targets
/// and publication lineage. Fill/stroke tessellation/coverage, external images, shaped
/// text, and composition-group intermediates are disposable realization details under
/// this one mixed-scene transaction.
#[derive(Debug)]
pub struct ResourceRenderer {
    literal: Renderer,
    clips: ClipRenderer,
    groups: group::GroupRenderer,
    solids: solid::SolidRenderer,
    images: image::ImageRenderer,
    shaped_runs: shaped::ShapedRunRenderer,
    surface_extent: Option<OffscreenExtent>,
    surface_target_generation: u64,
    surface_lineage: PublicationLineage,
}

impl ResourceRenderer {
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    pub async fn request(options: RendererOptions) -> Result<Self, RendererInitError> {
        Renderer::request(options).await.map(Self::from_literal)
    }

    /// Selects a native adapter using a caller-owned display connection.
    pub async fn request_with_display_handle(
        options: RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, RendererInitError> {
        Renderer::request_with_display_handle(options, display)
            .await
            .map(Self::from_literal)
    }

    /// Creates and retains a native surface before selecting a compatible adapter.
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
        let clips = ClipRenderer::new(&literal.base.device);
        let groups = group::GroupRenderer::new(&literal.base.device);
        let solids = solid::SolidRenderer::new();
        let images = image::ImageRenderer::new(&literal.base.device);
        let shaped_runs = shaped::ShapedRunRenderer::new(&literal.base.device);
        Self {
            literal,
            clips,
            groups,
            solids,
            images,
            shaped_runs,
            surface_extent: None,
            surface_target_generation: 0,
            surface_lineage: PublicationLineage::new(),
        }
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &RendererDiagnostics {
        self.literal.diagnostics()
    }

    #[must_use]
    pub const fn last_observation(&self) -> Option<&crate::PublicationObservation> {
        self.literal.last_observation()
    }

    #[must_use]
    pub const fn has_surface(&self) -> bool {
        self.literal.has_surface()
    }

    #[must_use]
    pub const fn configured_surface_extent(&self) -> Option<OffscreenExtent> {
        self.surface_extent
    }

    #[must_use]
    pub const fn surface_target_generation(&self) -> u64 {
        self.surface_target_generation
    }

    /// Configures the retained native surface for one non-zero physical extent.
    pub fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<OffscreenExtent, PublicationRenderError> {
        let extent = OffscreenExtent::new(width, height)?;
        self.literal.base.validate_extent(extent)?;
        let format = self
            .diagnostics()
            .surface_format()
            .ok_or(PublicationRenderError::SurfaceUnavailable)?;
        let next_generation = self
            .surface_target_generation
            .checked_add(1)
            .ok_or(PublicationRenderError::SurfaceTargetGenerationExhausted)?;
        let configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: extent.width(),
            height: extent.height(),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        self.literal
            .base
            .surface
            .as_ref()
            .ok_or(PublicationRenderError::SurfaceUnavailable)?
            .configure(&self.literal.base.device, &configuration);
        self.surface_extent = Some(extent);
        self.surface_target_generation = next_generation;
        self.surface_lineage.reset();
        Ok(extent)
    }

    #[must_use]
    pub fn discard_offscreen_target(&mut self) -> bool {
        self.literal.discard_offscreen_target()
    }

    #[must_use]
    pub fn discard_resource_cache(&mut self) -> bool {
        let images_discarded = self.images.discard_cache();
        let shaped_runs_discarded = self.shaped_runs.discard_cache();
        let discarded = images_discarded || shaped_runs_discarded;
        if discarded {
            if let Some(target) = self.literal.base.offscreen_target.as_mut() {
                target.lineage.reset();
            }
            self.surface_lineage.reset();
        }
        discarded
    }

    /// Renders one complete publication and reads actual GPU bytes.
    ///
    /// Fill/stroke geometry, images, shaped text, and atomic groups with ordinary
    /// shadows share one ordered target transaction. Scene/group validation, geometry
    /// and shadow-mask realization, device-limit checks, and resource preflight complete
    /// before retained-target mutation.
    #[allow(
        clippy::too_many_lines,
        reason = "the mixed render transaction intentionally keeps preflight, target mutation, ordered realization, readback, and lineage commit in one auditable sequence"
    )]
    pub fn render_offscreen_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
    ) -> Result<OffscreenPublicationReadback, PublicationRenderError> {
        let (canvas_extent, extent) = publication_extents(publication)?;
        self.literal.base.validate_extent(extent)?;
        let composition = group::prepare(
            publication.scene(),
            publication.raster_scale(),
            canvas_extent,
            extent,
            self.diagnostics().device_limits().max_buffer_size,
        )?;
        let scene = prepare_resource_scene(publication)?;
        self.preflight_gradient_stop_buffers(&scene)?;
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

        let mut observation = crate::PublicationObservation::new(publication, update_plan.mode());
        observation.set_target_facts(
            extent,
            self.literal
                .base
                .offscreen_target
                .as_ref()
                .filter(|target| target.matches(extent, OFFSCREEN_FORMAT))
                .map(|target| target.generation),
            self.diagnostics(),
        );
        self.literal.base.record_observation(observation);

        let has_groups = composition.has_groups();
        let has_solids = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::Solid(_)));
        let has_images = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::Image(_)));
        let has_shaped_runs = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::ShapedTextRun(_)));
        let needs_stencil =
            composition.needs_stencil() || scene.iter().any(ResourceSceneItem::needs_stencil);
        let live_images = live_image_resources(&scene);
        let live_shaped_runs = live_shaped_run_resources(&scene, publication);
        let initial_resource_observations = resource_observations_for_scene(
            &scene,
            publication,
            publication.raster_scale(),
            &self.images,
            &self.shaped_runs,
            None,
            None,
        );
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.set_resource_observations(initial_resource_observations);
        }

        if update_plan.mode() != PublicationUpdateMode::AlreadyCurrent {
            let (resolved_images, resolved_shaped_runs) = match self.preflight_resources(
                publication,
                &scene,
                publication.raster_scale(),
                provider,
            ) {
                Ok(resolved) => resolved,
                Err(error) => {
                    if let Some(observation) = self.literal.base.last_observation.as_mut() {
                        observation.set_resource_observations(resource_observations_for_scene(
                            &scene,
                            publication,
                            publication.raster_scale(),
                            &self.images,
                            &self.shaped_runs,
                            None,
                            error.item_index(),
                        ));
                    }
                    return Err(error);
                }
            };
            let resource_observations = resource_observations_for_scene(
                &scene,
                publication,
                publication.raster_scale(),
                &self.images,
                &self.shaped_runs,
                Some(&resolved_shaped_runs),
                None,
            );
            if let Some(observation) = self.literal.base.last_observation.as_mut() {
                observation.set_resource_observations(resource_observations);
            }
            if has_groups {
                self.groups
                    .ensure_pipelines(&self.literal.base.device, OFFSCREEN_FORMAT)?;
            }
            if has_solids {
                self.solids
                    .ensure_pipelines(&self.literal.base.device, OFFSCREEN_FORMAT)?;
            }
            if has_images {
                self.images
                    .ensure_pipelines(&self.literal.base.device, OFFSCREEN_FORMAT)?;
            }
            if has_shaped_runs {
                self.shaped_runs
                    .ensure_pipelines(&self.literal.base.device, OFFSCREEN_FORMAT)?;
            }
            self.images.realize(
                &self.literal.base.device,
                &self.literal.base.queue,
                resolved_images,
            );
            self.shaped_runs.realize(
                &self.literal.base.device,
                &self.literal.base.queue,
                resolved_shaped_runs,
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
                    label: Some("runenui mixed offscreen publication encoder"),
                });
        let target = self
            .literal
            .base
            .offscreen_target
            .as_ref()
            .unwrap_or_else(|| unreachable!("matching or newly created target is retained"));
        let target_generation = target.generation;
        let diagnostics = self.diagnostics().clone();
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.set_target_facts(extent, Some(target_generation), &diagnostics);
        }

        let stencil_target = (update_plan.mode() != PublicationUpdateMode::AlreadyCurrent
            && needs_stencil)
            .then(|| create_stencil_target(&self.literal.base.device, extent));
        if update_plan.mode() != PublicationUpdateMode::AlreadyCurrent {
            self.groups.encode_scene(
                &self.literal.base.device,
                &self.literal.base.queue,
                &self.solids,
                &self.clips,
                &self.images,
                &mut encoder,
                &target.view,
                stencil_target.as_ref().map(|(_, view)| view),
                target.format,
                extent,
                canvas_extent,
                publication.raster_scale(),
                publication,
                &scene,
                &composition,
                &self.shaped_runs,
            );
        }

        encode_target_copy(&mut encoder, &target.texture, &readback, extent, layout);
        let submission = self.literal.base.queue.submit([encoder.finish()]);
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.mark_render_succeeded();
        }
        let rgba8_srgb = match self
            .literal
            .base
            .map_readback(&readback, layout, submission)
        {
            Ok(pixels) => pixels,
            Err(error) => {
                if let Some(observation) = self.literal.base.last_observation.as_mut() {
                    observation.mark_readback_failed();
                }
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
        self.shaped_runs.retain(&live_shaped_runs);
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.mark_readback_succeeded();
        }
        let observation = self
            .literal
            .base
            .last_observation
            .clone()
            .unwrap_or_else(|| {
                unreachable!("publication observation was recorded before rendering")
            });
        Ok(OffscreenPublicationReadback {
            update_plan,
            target_generation,
            readback,
            observation,
        })
    }

    /// Renders one complete publication directly into the configured native surface.
    #[allow(
        clippy::too_many_lines,
        reason = "the native surface transaction keeps preflight, acquisition, shared mixed-scene encoding, submission, present, and lineage commit in one auditable sequence"
    )]
    pub fn render_surface_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
        before_present: impl FnOnce(),
    ) -> Result<crate::PublicationObservation, PublicationRenderError> {
        let extent = self
            .surface_extent
            .ok_or(PublicationRenderError::SurfaceNotConfigured)?;
        self.literal.base.validate_extent(extent)?;
        let canvas_extent =
            surface_canvas_extent(publication.logical_size(), publication.raster_scale());
        let composition = group::prepare(
            publication.scene(),
            publication.raster_scale(),
            canvas_extent,
            extent,
            self.diagnostics().device_limits().max_buffer_size,
        )?;
        let scene = prepare_resource_scene(publication)?;
        self.preflight_gradient_stop_buffers(&scene)?;
        let target_format = self
            .diagnostics()
            .surface_format()
            .ok_or(PublicationRenderError::SurfaceUnavailable)?;
        let update_mode = self.surface_lineage.classify(publication);
        let mut observation = crate::PublicationObservation::new(publication, update_mode);
        observation.set_target_facts_with_format(
            extent,
            Some(self.surface_target_generation),
            target_format,
            self.diagnostics(),
        );
        self.literal.base.record_observation(observation);

        let has_groups = composition.has_groups();
        let has_solids = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::Solid(_)));
        let has_images = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::Image(_)));
        let has_shaped_runs = scene
            .iter()
            .any(|item| matches!(item, ResourceSceneItem::ShapedTextRun(_)));
        let needs_stencil =
            composition.needs_stencil() || scene.iter().any(ResourceSceneItem::needs_stencil);
        let live_images = live_image_resources(&scene);
        let live_shaped_runs = live_shaped_run_resources(&scene, publication);
        let initial_resource_observations = resource_observations_for_scene(
            &scene,
            publication,
            publication.raster_scale(),
            &self.images,
            &self.shaped_runs,
            None,
            None,
        );
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.set_resource_observations(initial_resource_observations);
        }

        if update_mode != PublicationUpdateMode::AlreadyCurrent {
            let (resolved_images, resolved_shaped_runs) = match self.preflight_resources(
                publication,
                &scene,
                publication.raster_scale(),
                provider,
            ) {
                Ok(resolved) => resolved,
                Err(error) => {
                    if let Some(observation) = self.literal.base.last_observation.as_mut() {
                        observation.set_resource_observations(resource_observations_for_scene(
                            &scene,
                            publication,
                            publication.raster_scale(),
                            &self.images,
                            &self.shaped_runs,
                            None,
                            error.item_index(),
                        ));
                    }
                    return Err(error);
                }
            };
            let resource_observations = resource_observations_for_scene(
                &scene,
                publication,
                publication.raster_scale(),
                &self.images,
                &self.shaped_runs,
                Some(&resolved_shaped_runs),
                None,
            );
            if let Some(observation) = self.literal.base.last_observation.as_mut() {
                observation.set_resource_observations(resource_observations);
            }
            if has_groups {
                self.groups
                    .ensure_pipelines(&self.literal.base.device, target_format)?;
            }
            if has_solids {
                self.solids
                    .ensure_pipelines(&self.literal.base.device, target_format)?;
            }
            if has_images {
                self.images
                    .ensure_pipelines(&self.literal.base.device, target_format)?;
            }
            if has_shaped_runs {
                self.shaped_runs
                    .ensure_pipelines(&self.literal.base.device, target_format)?;
            }
            self.images.realize(
                &self.literal.base.device,
                &self.literal.base.queue,
                resolved_images,
            );
            self.shaped_runs.realize(
                &self.literal.base.device,
                &self.literal.base.queue,
                resolved_shaped_runs,
            );
        }

        let current = self
            .literal
            .base
            .surface
            .as_ref()
            .ok_or(PublicationRenderError::SurfaceUnavailable)?
            .get_current_texture();
        let surface_texture = match current {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                return Err(PublicationRenderError::SurfaceSuboptimal);
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                return Err(PublicationRenderError::SurfaceTimeout);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(PublicationRenderError::SurfaceOccluded);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                return Err(PublicationRenderError::SurfaceOutdated);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface_extent = None;
                self.surface_lineage.reset();
                return Err(PublicationRenderError::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(PublicationRenderError::SurfaceValidation);
            }
        };
        let color_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let stencil_target =
            needs_stencil.then(|| create_stencil_target(&self.literal.base.device, extent));
        let mut encoder =
            self.literal
                .base
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("runenui mixed native surface publication encoder"),
                });
        self.groups.encode_scene(
            &self.literal.base.device,
            &self.literal.base.queue,
            &self.solids,
            &self.clips,
            &self.images,
            &mut encoder,
            &color_view,
            stencil_target.as_ref().map(|(_, view)| view),
            target_format,
            extent,
            canvas_extent,
            publication.raster_scale(),
            publication,
            &scene,
            &composition,
            &self.shaped_runs,
        );
        self.literal.base.queue.submit([encoder.finish()]);
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.mark_render_succeeded();
        }
        before_present();
        self.literal.base.queue.present(surface_texture);
        self.surface_lineage.record_success(publication);
        self.images.retain(&live_images);
        self.shaped_runs.retain(&live_shaped_runs);
        if let Some(observation) = self.literal.base.last_observation.as_mut() {
            observation.mark_present_succeeded();
        }
        Ok(self
            .literal
            .base
            .last_observation
            .clone()
            .unwrap_or_else(|| unreachable!("surface publication observation was recorded")))
    }

    fn preflight_gradient_stop_buffers(
        &self,
        scene: &[ResourceSceneItem],
    ) -> Result<(), PublicationRenderError> {
        let limits = self.literal.diagnostics().device_limits();
        let max_bytes = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        for (item_index, item) in scene.iter().enumerate() {
            let ResourceSceneItem::Solid(item) = item else {
                continue;
            };
            let Some(required_bytes) = item.gradient_stop_buffer_size() else {
                continue;
            };
            if required_bytes > max_bytes {
                return Err(
                    PublicationRenderError::GradientStopBufferExceedsDeviceLimit {
                        item_index,
                        required_bytes,
                        max_bytes,
                    },
                );
            }
        }
        Ok(())
    }

    fn preflight_resources<P: ResourceProvider + ?Sized>(
        &self,
        publication: &PaintPublication,
        scene: &[ResourceSceneItem],
        raster_scale: RasterScale,
        provider: &P,
    ) -> Result<(Vec<image::ResolvedImage>, Vec<shaped::ResolvedShapedRun>), PublicationRenderError>
    {
        let images = self.preflight_images(scene, provider)?;
        let shaped_runs = self.preflight_shaped_runs(publication, scene, raster_scale)?;
        Ok((images, shaped_runs))
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
        let mut resolved = Vec::new();
        for item in scene {
            let ResourceSceneItem::Image(item) = item else {
                continue;
            };
            if let Some(actual) = self.images.extent(&item.image.resource) {
                image::extent_matches(item.image.intrinsic_size, actual)
                    .map_err(|failure| image_failure(item.image.item_index, failure))?;
                continue;
            }
            if let Some(pending) = resolved.iter().find(|resolved: &&image::ResolvedImage| {
                resolved.resource() == &item.image.resource
            }) {
                image::extent_matches(item.image.intrinsic_size, pending.extent())
                    .map_err(|failure| image_failure(item.image.item_index, failure))?;
                continue;
            }
            let resolved_image =
                image::resolve_image(provider, &item.image, max_texture_dimension_2d)
                    .map_err(|failure| image_failure(item.image.item_index, failure))?;
            resolved.push(resolved_image);
        }
        Ok(resolved)
    }

    fn preflight_shaped_runs(
        &self,
        publication: &PaintPublication,
        scene: &[ResourceSceneItem],
        raster_scale: RasterScale,
    ) -> Result<Vec<shaped::ResolvedShapedRun>, PublicationRenderError> {
        let max_texture_dimension_2d = self
            .literal
            .diagnostics()
            .device_limits()
            .max_texture_dimension_2d;
        let mut seen = HashSet::new();
        let mut resolved = Vec::new();
        for item in scene {
            let ResourceSceneItem::ShapedTextRun(item) = item else {
                continue;
            };
            let Some(resource) = publication
                .scene()
                .shaped_text_resource(&item.shaped_run.resource)
            else {
                return Err(PublicationRenderError::ShapedTextResourceUnavailable {
                    item_index: item.shaped_run.item_index,
                });
            };
            let quality = shaped::ShapedRunRenderer::quality(
                resource,
                raster_scale,
                item.shaped_run.local_to_surface,
            );
            let cache_key = (item.shaped_run.resource.clone(), quality);
            if self
                .shaped_runs
                .contains(resource, raster_scale, item.shaped_run.local_to_surface)
                || !seen.insert(cache_key)
            {
                continue;
            }
            match shaped::resolve_shaped_run(
                &item.shaped_run,
                resource,
                raster_scale,
                max_texture_dimension_2d,
            ) {
                Ok(shaped_run) => resolved.push(shaped_run),
                Err(shaped::ShapedRunResolveFailure::UnsupportedGlyph { glyph_id, kind }) => {
                    return Err(PublicationRenderError::UnsupportedShapedGlyph {
                        item_index: item.shaped_run.item_index,
                        glyph_id,
                        kind: match kind {
                            shaped::UnsupportedGlyphKind::ColrV0 => {
                                UnsupportedShapedGlyphKind::ColrV0
                            }
                            shaped::UnsupportedGlyphKind::ColrV1 => {
                                UnsupportedShapedGlyphKind::ColrV1
                            }
                            shaped::UnsupportedGlyphKind::Bitmap => {
                                UnsupportedShapedGlyphKind::Bitmap
                            }
                            shaped::UnsupportedGlyphKind::Svg => UnsupportedShapedGlyphKind::Svg,
                            shaped::UnsupportedGlyphKind::FauxBold => {
                                UnsupportedShapedGlyphKind::FauxBold
                            }
                        },
                    });
                }
                Err(shaped::ShapedRunResolveFailure::InvalidFont) => {
                    return Err(PublicationRenderError::ShapedTextFontInvalid {
                        item_index: item.shaped_run.item_index,
                    });
                }
                Err(shaped::ShapedRunResolveFailure::InvalidOutline { glyph_id }) => {
                    return Err(PublicationRenderError::ShapedTextOutlineInvalid {
                        item_index: item.shaped_run.item_index,
                        glyph_id,
                    });
                }
                Err(shaped::ShapedRunResolveFailure::GlyphExtentExceedsDeviceLimit {
                    width,
                    height,
                    max_texture_dimension_2d,
                    ..
                }) => {
                    return Err(
                        PublicationRenderError::ShapedGlyphExtentExceedsDeviceLimit {
                            item_index: item.shaped_run.item_index,
                            width,
                            height,
                            max_texture_dimension_2d,
                        },
                    );
                }
            }
        }
        Ok(resolved)
    }

    pub fn clear_offscreen(
        &self,
        extent: OffscreenExtent,
        color: Color,
    ) -> Result<OffscreenReadback, OffscreenRenderError> {
        self.literal.clear_offscreen(extent, color)
    }
}

fn image_failure(item_index: usize, failure: image::ImageResolveFailure) -> PublicationRenderError {
    match failure {
        image::ImageResolveFailure::Resource(error) => {
            PublicationRenderError::Resource { item_index, error }
        }
        image::ImageResolveFailure::IntrinsicExtentMismatch {
            expected_width,
            expected_height,
            actual_width,
            actual_height,
        } => PublicationRenderError::Resource {
            item_index,
            error: ResourceResolveError::ImageExtentMismatch {
                expected_width,
                expected_height,
                actual_width,
                actual_height,
            },
        },
        image::ImageResolveFailure::ExtentExceedsDeviceLimit {
            width,
            height,
            max_texture_dimension_2d,
        } => PublicationRenderError::ImageExtentExceedsDeviceLimit {
            item_index,
            width,
            height,
            max_texture_dimension_2d,
        },
        image::ImageResolveFailure::RowBytesOverflow { width } => {
            PublicationRenderError::ImageRowBytesOverflow { item_index, width }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ResourceSceneItem {
    Solid(solid::SupportedSolid),
    Image(ImageSceneItem),
    ShapedTextRun(ShapedTextRunSceneItem),
}

impl ResourceSceneItem {
    const fn needs_stencil(&self) -> bool {
        match self {
            Self::Solid(_) => true,
            Self::Image(item) => !item.clips.is_empty(),
            Self::ShapedTextRun(item) => !item.clips.is_empty(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ImageSceneItem {
    image: image::SupportedImage,
    clips: Vec<PreparedClip>,
}

#[derive(Clone, Debug, PartialEq)]
struct ShapedTextRunSceneItem {
    shaped_run: shaped::SupportedShapedRun,
    clips: Vec<PreparedClip>,
}

fn prepare_resource_scene(
    publication: &PaintPublication,
) -> Result<Vec<ResourceSceneItem>, PublicationRenderError> {
    let requirements = publication.scene().requirements();
    let capabilities = SceneCapabilities::new([ResourceKind::Image, ResourceKind::ShapedTextRun]);
    let unsupported_resource_kind =
        capabilities
            .check_requirements(&requirements)
            .err()
            .map(|error| SceneValidationError::UnsupportedResourceKind {
                resource_kind: error.resource_kind(),
            });
    let mut items = Vec::with_capacity(publication.scene().items().len());
    for (item_index, item) in publication.scene().items().iter().enumerate() {
        let prepared_clips = clip::prepare_clips(item.clips()).map_err(|failure| {
            PublicationRenderError::ClipGeometry {
                item_index,
                clip_index: failure.clip_index(),
                detail: failure.error().to_string(),
            }
        })?;

        match item.primitive() {
            PaintPrimitive::Fill { shape, brush } => {
                let solid = solid::SupportedSolid::fill(
                    shape,
                    brush.clone(),
                    item.opacity(),
                    item.local_to_surface(),
                    prepared_clips,
                )
                .map_err(|error| PublicationRenderError::SolidGeometry {
                    item_index,
                    detail: error.to_string(),
                })?;
                items.push(ResourceSceneItem::Solid(solid));
            }
            PaintPrimitive::Stroke {
                shape,
                brush,
                style,
            } => {
                let solid = solid::SupportedSolid::stroke(
                    shape,
                    *style,
                    brush.clone(),
                    item.opacity(),
                    item.local_to_surface(),
                    prepared_clips,
                )
                .map_err(|error| PublicationRenderError::SolidGeometry {
                    item_index,
                    detail: error.to_string(),
                })?;
                items.push(ResourceSceneItem::Solid(solid));
            }
            PaintPrimitive::Image(image_primitive) => {
                let Some(intrinsic_size) = image_primitive.resolved_intrinsic_size() else {
                    return Err(scene_failure(SceneValidationError::UnsupportedItem {
                        item_index,
                        semantic: UnsupportedSceneSemantic::Image,
                    }));
                };
                let patch_count = image_primitive
                    .resolved_patch_count()
                    .unwrap_or_else(|| unreachable!("resolved intrinsic metadata implies patches"));
                let patches = (0..patch_count)
                    .map(|patch_index| {
                        let (source, destination) = image_primitive
                            .resolved_patch(patch_index)
                            .unwrap_or_else(|| unreachable!("resolved patch count is exact"));
                        image::SupportedImagePatch {
                            source,
                            destination,
                        }
                    })
                    .collect();
                items.push(ResourceSceneItem::Image(ImageSceneItem {
                    image: image::SupportedImage {
                        item_index,
                        resource: image_primitive.resource_ref().clone(),
                        intrinsic_size,
                        patches,
                        opacity: item.opacity(),
                        local_to_surface: item.local_to_surface(),
                    },
                    clips: prepared_clips,
                }));
            }
            PaintPrimitive::ShapedTextRun(shaped_run) => {
                items.push(ResourceSceneItem::ShapedTextRun(ShapedTextRunSceneItem {
                    shaped_run: shaped::SupportedShapedRun {
                        item_index,
                        resource: shaped_run.resource_ref().clone(),
                        origin: shaped_run.origin(),
                        foreground: shaped_run.foreground(),
                        opacity: item.opacity(),
                        local_to_surface: item.local_to_surface(),
                    },
                    clips: prepared_clips,
                }));
            }
            _ => {
                return Err(scene_failure(SceneValidationError::UnsupportedItem {
                    item_index,
                    semantic: UnsupportedSceneSemantic::UnknownPrimitive,
                }));
            }
        }
    }
    if let Some(error) = unsupported_resource_kind {
        return Err(scene_failure(error));
    }
    Ok(items)
}

fn scene_failure(error: SceneValidationError) -> PublicationRenderError {
    PublicationRenderError::Backend(scene_validation_error(error))
}

fn live_image_resources(scene: &[ResourceSceneItem]) -> HashSet<ResourceRef> {
    scene
        .iter()
        .filter_map(|item| match item {
            ResourceSceneItem::Image(item) => Some(item.image.resource.clone()),
            ResourceSceneItem::Solid(_) | ResourceSceneItem::ShapedTextRun(_) => None,
        })
        .collect()
}

fn live_shaped_run_resources(
    scene: &[ResourceSceneItem],
    publication: &PaintPublication,
) -> HashSet<(ResourceRef, shaped::QualityTier)> {
    scene
        .iter()
        .filter_map(|item| match item {
            ResourceSceneItem::ShapedTextRun(item) => publication
                .scene()
                .shaped_text_resource(&item.shaped_run.resource)
                .map(|resource| {
                    (
                        item.shaped_run.resource.clone(),
                        shaped::ShapedRunRenderer::quality(
                            resource,
                            publication.raster_scale(),
                            item.shaped_run.local_to_surface,
                        ),
                    )
                }),
            ResourceSceneItem::Solid(_) | ResourceSceneItem::Image(_) => None,
        })
        .collect()
}

fn surface_canvas_extent(
    logical_size: LogicalSize,
    raster_scale: RasterScale,
) -> RasterCanvasExtent {
    let scale = f64::from(raster_scale.get());
    RasterCanvasExtent::new(
        f64::from(logical_size.width()) * scale,
        f64::from(logical_size.height()) * scale,
    )
}

fn resource_observations_for_scene(
    scene: &[ResourceSceneItem],
    publication: &PaintPublication,
    raster_scale: RasterScale,
    images: &image::ImageRenderer,
    shaped_runs: &shaped::ShapedRunRenderer,
    resolved_shaped_runs: Option<&[shaped::ResolvedShapedRun]>,
    failed_item_index: Option<usize>,
) -> Vec<ResourceObservation> {
    let empty_shaped = resolved_shaped_runs
        .unwrap_or_default()
        .iter()
        .filter(|resolved| resolved.is_empty())
        .map(shaped::ResolvedShapedRun::resource_key)
        .collect::<HashSet<_>>();
    scene
        .iter()
        .filter_map(|item| match item {
            ResourceSceneItem::Image(item) => Some(ResourceObservation::new(
                item.image.item_index,
                item.image.resource.clone(),
                ResourceRealizationKind::Image,
                if failed_item_index == Some(item.image.item_index) {
                    ResourceCacheOutcome::Failed
                } else if images.contains(&item.image.resource) {
                    ResourceCacheOutcome::Reused
                } else {
                    ResourceCacheOutcome::Realized
                },
            )),
            ResourceSceneItem::ShapedTextRun(item) => {
                let resource = publication
                    .scene()
                    .shaped_text_resource(&item.shaped_run.resource);
                let key = resource.map(|resource| {
                    (
                        item.shaped_run.resource.clone(),
                        shaped::ShapedRunRenderer::quality(
                            resource,
                            raster_scale,
                            item.shaped_run.local_to_surface,
                        ),
                    )
                });
                let cache_outcome = if failed_item_index == Some(item.shaped_run.item_index) {
                    ResourceCacheOutcome::Failed
                } else if resource.is_some_and(|resource| {
                    shaped_runs.contains(resource, raster_scale, item.shaped_run.local_to_surface)
                }) {
                    ResourceCacheOutcome::Reused
                } else if key.is_some_and(|key| empty_shaped.contains(&key)) {
                    ResourceCacheOutcome::EmptyCoverage
                } else {
                    ResourceCacheOutcome::Realized
                };
                Some(ResourceObservation::new(
                    item.shaped_run.item_index,
                    item.shaped_run.resource.clone(),
                    ResourceRealizationKind::ShapedText,
                    cache_outcome,
                ))
            }
            ResourceSceneItem::Solid(_) => None,
        })
        .collect()
}

#[allow(
    clippy::too_many_arguments,
    reason = "one mixed-scene item dispatch keeps exact target/canvas/scale, generic clip realization, and resource renderers explicit while runtime-published grouping owns traversal"
)]
fn encode_resource_item_to_target(
    device: &wgpu::Device,
    solid_renderer: &solid::SolidRenderer,
    clip_renderer: &ClipRenderer,
    image_renderer: &image::ImageRenderer,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: Option<&wgpu::TextureView>,
    target_format: wgpu::TextureFormat,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
    publication: &PaintPublication,
    item: &ResourceSceneItem,
    shaped_renderer: &shaped::ShapedRunRenderer,
) {
    match item {
        ResourceSceneItem::Solid(item) => solid_renderer.encode_item(
            device,
            clip_renderer,
            encoder,
            color_view,
            stencil_view.unwrap_or_else(|| unreachable!("fill/stroke item requires stencil")),
            target_format,
            extent,
            canvas_extent,
            raster_scale,
            item,
        ),
        ResourceSceneItem::Image(item) => encode_resource_image_item(
            device,
            clip_renderer,
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
        ResourceSceneItem::ShapedTextRun(item) => encode_resource_shaped_run_item(
            device,
            clip_renderer,
            shaped_renderer,
            encoder,
            color_view,
            stencil_view,
            target_format,
            extent,
            canvas_extent,
            raster_scale,
            publication,
            item,
        ),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "mixed-scene image dispatch keeps exact geometry, target format, generic clip realization, and cached resource identity explicit at the sampled-image draw boundary"
)]
fn encode_resource_image_item(
    device: &wgpu::Device,
    clip_renderer: &ClipRenderer,
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

    let stencil_view =
        stencil_view.unwrap_or_else(|| unreachable!("clipped image requires stencil target"));
    clear_stencil_mask(encoder, stencil_view);
    clip_renderer.apply_clips(
        device,
        encoder,
        stencil_view,
        extent,
        canvas_extent,
        raster_scale,
        &item.clips,
    );
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

#[allow(
    clippy::too_many_arguments,
    reason = "mixed-scene shaped-text dispatch keeps exact geometry, renderer quality realization, target format, generic clip realization, and sampled MSDF atlas authority explicit"
)]
fn encode_resource_shaped_run_item(
    device: &wgpu::Device,
    clip_renderer: &ClipRenderer,
    shaped_renderer: &shaped::ShapedRunRenderer,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: Option<&wgpu::TextureView>,
    target_format: wgpu::TextureFormat,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
    publication: &PaintPublication,
    item: &ShapedTextRunSceneItem,
) {
    let resource = publication
        .scene()
        .shaped_text_resource(&item.shaped_run.resource)
        .unwrap_or_else(|| unreachable!("shaped-text resource was preflighted"));
    let batches = shaped_renderer.vertex_batches(
        &item.shaped_run,
        resource,
        extent,
        canvas_extent,
        raster_scale,
    );
    if batches.is_empty() {
        return;
    }
    let stencil_view = if item.clips.is_empty() {
        None
    } else {
        let stencil_view = stencil_view
            .unwrap_or_else(|| unreachable!("clipped shaped-text requires stencil target"));
        clear_stencil_mask(encoder, stencil_view);
        clip_renderer.apply_clips(
            device,
            encoder,
            stencil_view,
            extent,
            canvas_extent,
            raster_scale,
            &item.clips,
        );
        Some(stencil_view)
    };
    let quality = shaped::ShapedRunRenderer::quality(
        resource,
        raster_scale,
        item.shaped_run.local_to_surface,
    );
    for (page_index, vertex_bytes) in batches {
        let vertex_count = u32::try_from(vertex_bytes.len() / 36).unwrap_or(u32::MAX);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui ordered mixed-scene shaped-text glyph vertices"),
            contents: &vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        shaped_renderer.draw(
            target_format,
            encoder,
            color_view,
            stencil_view,
            &item.shaped_run.resource,
            quality,
            page_index,
            &vertex_buffer,
            vertex_count,
        );
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        Element, ImageDescriptor, ImageIntrinsicSize, ImageMapping, ImagePaintDescriptor,
        IntoEffects, LogicalLength, LogicalRect, LogicalSize, NoHostProtocol, PaintContribution,
        PaintContributionContext, PaintContributionItem, PaintPrimitive, ResourceKind, ResourceRef,
        SceneOpacity, StyleEnvironment, UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput,
    };
    use runenui_runtime::{AppRuntime, LayoutConstraints, RasterScale, SurfaceBuildContext};

    use super::{
        OffscreenExtent, group, prepare_resource_scene, publication_extents, surface_canvas_extent,
    };

    #[derive(Debug)]
    struct ImagePaint;

    impl Widget<()> for ImagePaint {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
            WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
        }

        fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
            let resource = ResourceRef::new(ResourceKind::Image);
            let image = ImageDescriptor::new(
                resource,
                ImageIntrinsicSize::new(1, 1)
                    .unwrap_or_else(|| unreachable!("controlled intrinsic size is non-zero")),
            )
            .unwrap_or_else(|_| unreachable!("controlled resource has image kind"));
            let destination = LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
                .unwrap_or_else(|_| unreachable!("controlled destination is valid"));
            let descriptor = ImagePaintDescriptor::new(image, destination, ImageMapping::default())
                .unwrap_or_else(|_| unreachable!("controlled image mapping is valid"));
            PaintContribution::single(PaintContributionItem::image(descriptor))
        }
    }

    struct GroupedImageApp;

    impl UiApp for GroupedImageApp {
        type State = ();
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root((): &Self::State) -> impl View<Self::Action> {
            Element::new(ImagePaint).opacity(
                SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("controlled opacity is valid")),
            )
        }

        fn update(
            (): &mut Self::State,
            (): Self::Action,
        ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        }
    }

    #[test]
    fn resource_and_group_preflight_accept_shadow_free_grouped_image() {
        let mut runtime = AppRuntime::<GroupedImageApp>::mount(());
        let environment = StyleEnvironment::default();
        let publication = runtime
            .publish_surface(&SurfaceBuildContext::new(
                &environment,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("controlled publication is admitted"));
        assert_eq!(publication.paint_scene().groups().len(), 1);
        assert!(matches!(
            publication.paint_scene().items()[0].primitive(),
            PaintPrimitive::Image(_)
        ));

        let (canvas_extent, target_extent) = publication_extents(publication.paint_publication())
            .unwrap_or_else(|_| unreachable!("controlled publication extents are valid"));
        let prepared = group::prepare(
            publication.paint_scene(),
            publication.paint_publication().raster_scale(),
            canvas_extent,
            target_extent,
            u64::MAX,
        )
        .unwrap_or_else(|_| unreachable!("shadow-free group is supported"));
        assert!(prepared.has_groups());
        assert!(prepare_resource_scene(publication.paint_publication()).is_ok());
    }

    #[test]
    fn native_surface_extent_is_not_reconstructed_from_fractional_scale() {
        let logical_size = LogicalSize::try_new(0.8, 0.8)
            .unwrap_or_else(|_| unreachable!("fixture logical size is valid"));
        let raster_scale = RasterScale::new(1.25)
            .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
        let configured = OffscreenExtent::new(1, 1)
            .unwrap_or_else(|_| unreachable!("fixture target extent is valid"));

        let canvas_extent = surface_canvas_extent(logical_size, raster_scale);

        assert!(canvas_extent.width() > 1.0);
        assert!(canvas_extent.width() < 1.000_001);
        assert_eq!(configured.width(), 1);
        assert_eq!(configured.height(), 1);
    }
}
