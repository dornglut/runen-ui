mod stroke_mask;

use std::collections::HashMap;

use runenui_core::{Color, PaintPrimitive, SceneShape};
use runenui_runtime::{PaintPublication, RasterScale, SceneClip};
use wgpu::util::DeviceExt;

use crate::{
    PublicationUpdateMode, PublicationUpdatePlan, WgpuHasDisplayHandle,
    scene_subset::{
        SceneValidationError, SupportedLiteralRect, publication_resource_error,
        validate_literal_rect_item,
    },
};

const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;
const STENCIL_ALLOWED: u32 = 1;
const CLIP_MASK_SHADER: &str = r"
struct ClipUniform {
    transform_a: vec4<f32>,
    transform_b: vec4<f32>,
    rect: vec4<f32>,
    radii: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> clip: ClipUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

fn outside_circle(point: vec2<f32>, center: vec2<f32>, radius: f32) -> bool {
    if radius <= 0.0 {
        return false;
    }
    let delta = point - center;
    return dot(delta, delta) > radius * radius;
}

fn contains_clip(local: vec2<f32>) -> bool {
    let left = clip.rect.x;
    let top = clip.rect.y;
    let right = clip.rect.z;
    let bottom = clip.rect.w;
    if !(local.x >= left && local.x < right && local.y >= top && local.y < bottom) {
        return false;
    }
    if clip.transform_b.w < 0.5 {
        return true;
    }

    let top_left = clip.radii.x;
    if local.x < left + top_left && local.y < top + top_left
        && outside_circle(local, vec2<f32>(left + top_left, top + top_left), top_left)
    {
        return false;
    }

    let top_right = clip.radii.y;
    if local.x >= right - top_right && local.y < top + top_right
        && outside_circle(local, vec2<f32>(right - top_right, top + top_right), top_right)
    {
        return false;
    }

    let bottom_right = clip.radii.z;
    if local.x >= right - bottom_right && local.y >= bottom - bottom_right
        && outside_circle(
            local,
            vec2<f32>(right - bottom_right, bottom - bottom_right),
            bottom_right,
        )
    {
        return false;
    }

    let bottom_left = clip.radii.w;
    if local.x < left + bottom_left && local.y >= bottom - bottom_left
        && outside_circle(
            local,
            vec2<f32>(left + bottom_left, bottom - bottom_left),
            bottom_left,
        )
    {
        return false;
    }

    return true;
}

@fragment
fn fs_main(input: VertexOutput) {
    let surface = input.position.xy / clip.transform_b.z;
    let local = vec2<f32>(
        clip.transform_a.x * surface.x + clip.transform_a.z * surface.y + clip.transform_b.x,
        clip.transform_a.y * surface.x + clip.transform_a.w * surface.y + clip.transform_b.y,
    );
    if contains_clip(local) {
        discard;
    }
}
";

#[derive(Debug)]
struct ClipTargetPipelines {
    clipped_fill: wgpu::RenderPipeline,
    mask: ClipMaskPipeline,
    stroke_mask: stroke_mask::StrokeMaskPipeline,
}

#[derive(Debug)]
struct ClipMaskPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ClipMaskPipeline {
    fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui clip-mask bind-group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("runenui clip-mask pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("runenui clip-mask shader"),
            source: wgpu::ShaderSource::Wgsl(CLIP_MASK_SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("runenui clip-mask pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(mask_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// Canonical renderer facade with explicit M6 clip and centered-stroke realization.
///
/// The wrapped backend remains the single owner of the instance, adapter,
/// device, queue, native surface, retained color target, publication lineage,
/// and readback machinery. Mask-specific pipelines and the ephemeral stencil
/// attachment are renderer-local realization only.
#[derive(Debug)]
pub struct Renderer {
    base: super::Renderer,
    clip_pipelines: HashMap<wgpu::TextureFormat, ClipTargetPipelines>,
}

impl Renderer {
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request(
        options: super::RendererOptions,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request(options).await.map(Self::from_base)
    }

    /// Selects a native adapter using a caller-owned display connection.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request_with_display_handle(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request_with_display_handle(options, display)
            .await
            .map(Self::from_base)
    }

    /// Creates and retains a native surface before selecting a compatible adapter.
    ///
    /// # Errors
    ///
    /// Returns structured surface-creation, compatible-adapter, target-format, or
    /// device diagnostics when construction fails.
    pub async fn request_with_surface_target(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request_with_surface_target(options, display, window)
            .await
            .map(Self::from_base)
    }

    fn from_base(base: super::Renderer) -> Self {
        Self {
            base,
            clip_pipelines: HashMap::new(),
        }
    }

    /// Returns immutable instance, adapter, device, and target diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &super::RendererDiagnostics {
        self.base.diagnostics()
    }

    /// Returns whether construction retained an actual native surface target.
    #[must_use]
    pub const fn has_surface(&self) -> bool {
        self.base.has_surface()
    }

    /// Drops the retained offscreen target and every publication realization tied to it.
    #[must_use]
    pub fn discard_offscreen_target(&mut self) -> bool {
        self.base.discard_offscreen_target()
    }

    /// Renders the exact currently supported scene subset and reads actual GPU bytes.
    ///
    /// A publication containing only unclipped `FillRect` items delegates to the
    /// already-validated base path. Explicit clips and centered `StrokeRect`
    /// items use the stencil path while preserving the same color geometry,
    /// affine transform, source-over, target, lineage, and readback authority.
    /// A non-collapsed stroke draws its accepted expanded rectangle while the
    /// exact accepted inset is cleared from the stencil mask; a collapsed inset
    /// therefore naturally becomes the complete expanded rectangle. Zero-width,
    /// zero-area, or checked derived-rectangle overflow contributes no coverage.
    ///
    /// # Errors
    ///
    /// Returns a deterministic scene, extent, target-format, device, or readback
    /// failure. Validation still completes before retained-target mutation.
    pub fn render_offscreen_publication(
        &mut self,
        publication: &PaintPublication,
    ) -> Result<super::OffscreenPublicationReadback, super::OffscreenRenderError> {
        if publication.scene().items().iter().all(|item| {
            item.clips().is_empty() && matches!(item.primitive(), PaintPrimitive::FillRect { .. })
        }) {
            return self.base.render_offscreen_publication(publication);
        }

        let literal_rects =
            validate_clipped_scene_subset(publication).map_err(super::scene_validation_error)?;
        let (canvas_extent, extent) = super::publication_extents(publication)?;
        self.base.validate_extent(extent)?;
        let layout = super::ReadbackLayout::new(extent)?;
        self.base.validate_readback_buffer(layout)?;
        self.base
            .ensure_fill_rect_pipeline(super::OFFSCREEN_FORMAT)?;
        self.ensure_clip_pipelines(super::OFFSCREEN_FORMAT)?;

        let retained_target_matches = self
            .base
            .offscreen_target
            .as_ref()
            .is_some_and(|target| target.matches(extent, super::OFFSCREEN_FORMAT));
        let update_plan = if retained_target_matches {
            self.base
                .offscreen_target
                .as_ref()
                .map_or_else(PublicationUpdatePlan::full_resync, |target| {
                    target.lineage.plan(publication)
                })
        } else {
            PublicationUpdatePlan::full_resync()
        };

        if !retained_target_matches {
            let target = self.base.create_offscreen_target(extent)?;
            self.base.offscreen_target = Some(target);
        }

        let readback = self.base.create_readback_buffer(layout);
        let mut encoder =
            self.base
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("runenui literal-rect offscreen publication encoder"),
                });
        let target = self
            .base
            .offscreen_target
            .as_ref()
            .unwrap_or_else(|| unreachable!("matching or newly created target is retained"));
        let target_generation = target.generation;
        let stencil_target = (update_plan.mode() != PublicationUpdateMode::AlreadyCurrent)
            .then(|| create_stencil_target(&self.base.device, extent));
        if let Some((_, stencil_view)) = stencil_target.as_ref() {
            let ordinary_pipeline = self
                .base
                .fill_rect_pipelines
                .get(&target.format)
                .unwrap_or_else(|| unreachable!("ordinary target pipeline is cached"));
            let clip_pipelines = self
                .clip_pipelines
                .get(&target.format)
                .unwrap_or_else(|| unreachable!("literal mask pipelines are cached"));
            encode_clipped_scene_to_target(
                &self.base.device,
                ordinary_pipeline,
                clip_pipelines,
                &mut encoder,
                &target.view,
                stencil_view,
                extent,
                canvas_extent,
                publication.raster_scale(),
                &literal_rects,
            );
        }

        super::encode_target_copy(&mut encoder, &target.texture, &readback, extent, layout);
        let submission = self.base.queue.submit([encoder.finish()]);
        let rgba8_srgb = match self.base.map_readback(&readback, layout, submission) {
            Ok(pixels) => pixels,
            Err(error) => {
                self.base.offscreen_target = None;
                return Err(error);
            }
        };
        let readback = super::OffscreenReadback {
            extent,
            format: super::OFFSCREEN_FORMAT,
            rgba8_srgb,
        };

        self.base
            .offscreen_target
            .as_mut()
            .unwrap_or_else(|| unreachable!("successful readback retains its target"))
            .lineage
            .record_success(publication);
        Ok(super::OffscreenPublicationReadback {
            update_plan,
            target_generation,
            readback,
        })
    }

    /// Executes one real wgpu render-pass clear and returns actual texture bytes from GPU readback.
    ///
    /// # Errors
    ///
    /// Returns structured extent, device-wait, buffer-map, or mapped-range failures.
    pub fn clear_offscreen(
        &self,
        extent: super::OffscreenExtent,
        color: Color,
    ) -> Result<super::OffscreenReadback, super::OffscreenRenderError> {
        self.base.clear_offscreen(extent, color)
    }

    fn ensure_clip_pipelines(
        &mut self,
        format: wgpu::TextureFormat,
    ) -> Result<(), super::OffscreenRenderError> {
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(super::OffscreenRenderError::UnsupportedTargetFormat { format });
        }
        let device = &self.base.device;
        self.clip_pipelines
            .entry(format)
            .or_insert_with(|| ClipTargetPipelines {
                clipped_fill: create_clipped_fill_pipeline(device, format),
                mask: ClipMaskPipeline::new(device),
                stroke_mask: stroke_mask::StrokeMaskPipeline::new(device),
            });
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LiteralRectItem {
    literal: SupportedLiteralRect,
    clips: Vec<SceneClip>,
}

fn validate_clipped_scene_subset(
    publication: &PaintPublication,
) -> Result<Vec<LiteralRectItem>, SceneValidationError> {
    let unsupported_resource_kind = publication_resource_error(publication);
    let mut literal_rects = Vec::with_capacity(publication.scene().items().len());
    for (item_index, item) in publication.scene().items().iter().enumerate() {
        if let Some(literal) = validate_literal_rect_item(item_index, item)? {
            literal_rects.push(LiteralRectItem {
                literal,
                clips: item.clips().to_vec(),
            });
        }
    }
    if let Some(error) = unsupported_resource_kind {
        return Err(error);
    }
    Ok(literal_rects)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClipUniform {
    values: [f32; 16],
}

impl ClipUniform {
    fn from_scene_clip(clip: SceneClip, raster_scale: RasterScale) -> Option<Self> {
        let surface_to_clip = clip.clip_to_surface().inverse()?;
        let [m11, m12, m21, m22, tx, ty] = surface_to_clip.components();
        let shape = clip.shape();
        let rect = shape.outer_rect();
        let radii = normalized_clip_radii(shape);
        let shape_kind = if shape.radius().is_some() { 1.0 } else { 0.0 };
        Some(Self {
            values: [
                m11,
                m12,
                m21,
                m22,
                tx,
                ty,
                raster_scale.get(),
                shape_kind,
                rect.x(),
                rect.y(),
                rect.max_x(),
                rect.max_y(),
                radii[0],
                radii[1],
                radii[2],
                radii[3],
            ],
        })
    }

    fn bytes(&self) -> [u8; 64] {
        let mut bytes = [0_u8; 64];
        for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(self.values) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

fn prepare_clip_uniforms(
    clips: &[SceneClip],
    raster_scale: RasterScale,
) -> Option<Vec<ClipUniform>> {
    clips
        .iter()
        .copied()
        .map(|clip| ClipUniform::from_scene_clip(clip, raster_scale))
        .collect()
}

fn normalized_clip_radii(shape: SceneShape) -> [f32; 4] {
    let Some(radius) = shape.radius() else {
        return [0.0; 4];
    };
    let rect = shape.outer_rect();
    let radii = [
        f64::from(radius.top_left().get()),
        f64::from(radius.top_right().get()),
        f64::from(radius.bottom_right().get()),
        f64::from(radius.bottom_left().get()),
    ];
    let width = f64::from(rect.width());
    let height = f64::from(rect.height());
    let mut factor = 1.0_f64;
    for (extent, first, second) in [
        (width, radii[0], radii[1]),
        (width, radii[3], radii[2]),
        (height, radii[0], radii[3]),
        (height, radii[1], radii[2]),
    ] {
        let denominator = first + second;
        if denominator > 0.0 {
            factor = factor.min(extent / denominator);
        }
    }
    narrow_normalized_radii(radii.map(|value| value * factor))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "normalized radii originate as finite f32 logical lengths and the common normalization factor never exceeds one"
)]
fn narrow_normalized_radii(radii: [f64; 4]) -> [f32; 4] {
    radii.map(|value| value as f32)
}

fn create_clipped_fill_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui clipped FillRect shader"),
        source: wgpu::ShaderSource::Wgsl(super::FILL_RECT_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui clipped FillRect pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui clipped FillRect pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &super::FILL_RECT_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(clipped_fill_stencil_state()),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

const fn mask_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Zero,
    }
}

fn mask_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: mask_stencil_face(),
            back: mask_stencil_face(),
            read_mask: 0xff,
            write_mask: 0xff,
        },
    )
}

const fn clipped_fill_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Keep,
    }
}

fn clipped_fill_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: clipped_fill_stencil_face(),
            back: clipped_fill_stencil_face(),
            read_mask: 0xff,
            write_mask: 0,
        },
    )
}

fn create_stencil_target(
    device: &wgpu::Device,
    extent: super::OffscreenExtent,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("runenui literal-mask stencil target"),
        size: super::texture_extent(extent),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: STENCIL_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the literal-rectangle low-level encoder keeps the validated target, exact canvas, raster scale, and explicit ordinary/mask pipelines visible at the realization boundary"
)]
fn encode_clipped_scene_to_target(
    device: &wgpu::Device,
    ordinary_pipeline: &wgpu::RenderPipeline,
    clip_pipelines: &ClipTargetPipelines,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: &wgpu::TextureView,
    extent: super::OffscreenExtent,
    canvas_extent: super::RasterCanvasExtent,
    raster_scale: RasterScale,
    literal_rects: &[LiteralRectItem],
) {
    clear_color_target(encoder, color_view);
    for item in literal_rects {
        let vertex_bytes = super::fill_rect_vertex_bytes(
            std::slice::from_ref(&item.literal.fill),
            extent,
            canvas_extent,
            raster_scale,
        );
        if vertex_bytes.is_empty() {
            continue;
        }
        let vertex_count = u32::try_from(vertex_bytes.len() / 24).unwrap_or(u32::MAX);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui ordered literal-rect vertices"),
            contents: &vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });

        if item.literal.stroke_inset.is_none() && item.clips.is_empty() {
            draw_unclipped_fill(
                encoder,
                color_view,
                ordinary_pipeline,
                &vertex_buffer,
                vertex_count,
            );
            continue;
        }

        let Some(clip_uniforms) = prepare_clip_uniforms(&item.clips, raster_scale) else {
            continue;
        };
        let stroke_uniform = if item.literal.stroke_inset.is_some() {
            let Some(uniform) =
                stroke_mask::StrokeMaskUniform::from_literal(&item.literal, raster_scale)
            else {
                continue;
            };
            Some(uniform)
        } else {
            None
        };

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
}

fn clear_color_target(encoder: &mut wgpu::CommandEncoder, color_view: &wgpu::TextureView) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    });
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui literal-rect scene clear pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn clear_stencil_mask(encoder: &mut wgpu::CommandEncoder, stencil_view: &wgpu::TextureView) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(STENCIL_ALLOWED),
            store: wgpu::StoreOp::Store,
        }),
    };
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui literal stencil reset pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn apply_clip_mask(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &ClipMaskPipeline,
    uniform: &ClipUniform,
) {
    let uniform_bytes = uniform.bytes();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui clip-mask uniform"),
        contents: &uniform_bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("runenui clip-mask bind group"),
        layout: &pipeline.bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        }),
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui conjunctive clip-mask pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(&pipeline.pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}

fn draw_unclipped_fill(
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    });
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui ordered unclipped literal-rect pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.draw(0..vertex_count, 0..1);
}

fn draw_clipped_fill(
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    });
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        }),
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui ordered masked literal-rect pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);
    render_pass.set_stencil_reference(STENCIL_ALLOWED);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.draw(0..vertex_count, 0..1);
}

#[cfg(test)]
mod tests {
    #![allow(refining_impl_trait)]

    use core::{error::Error, future::Future, pin::pin, task::Poll};
    use std::{
        sync::Arc,
        task::{Context, Wake, Waker},
        thread,
    };

    use runenui_core::{
        Color, ContributionClip, Element, LogicalLength, LogicalPoint, LogicalRect, LogicalSize,
        LogicalTransform, NoHostProtocol, PaintContribution, PaintContributionContext,
        PaintContributionItem, Radius, SceneShape, StyleTokens, UiApp, Widget, WidgetInvalidation,
        WidgetMeasure, WidgetUpdateContext,
    };
    use runenui_runtime::{
        AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SceneClip,
        SurfaceBuildContext,
    };

    use super::{ClipUniform, Renderer, prepare_clip_uniforms, validate_clipped_scene_subset};
    use crate::{BackendSelection, RendererInitError, RendererOptions};

    const SURFACE_WIDTH: u16 = 64;
    const SURFACE_HEIGHT: u16 = 48;

    #[derive(Clone, Debug)]
    struct SceneFixture {
        items: Vec<PaintContributionItem>,
    }

    impl Widget<Vec<PaintContributionItem>> for SceneFixture {
        type State = Vec<PaintContributionItem>;

        fn create_state(&self) -> Self::State {
            self.items.clone()
        }

        fn update(
            &self,
            state: &mut Self::State,
            context: &mut WidgetUpdateContext<Vec<PaintContributionItem>>,
        ) {
            *state = self.items.clone();
            context.invalidate(WidgetInvalidation::PAINT);
        }

        fn measure(&self, _: &Self::State) -> WidgetMeasure {
            WidgetMeasure::Fixed {
                width: LogicalLength::from(SURFACE_WIDTH),
                height: LogicalLength::from(SURFACE_HEIGHT),
            }
        }

        fn paint(&self, items: &Self::State, _: PaintContributionContext) -> PaintContribution {
            PaintContribution::new(items.clone())
        }
    }

    struct FixtureApp;

    impl UiApp for FixtureApp {
        type State = Vec<PaintContributionItem>;
        type Action = Vec<PaintContributionItem>;
        type HostProtocol = NoHostProtocol;

        fn root(items: &Self::State) -> Element<Self::Action> {
            Element::new(SceneFixture {
                items: items.clone(),
            })
        }

        fn update(items: &mut Self::State, replacement: Self::Action) {
            *items = replacement;
        }
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn publication(items: Vec<PaintContributionItem>, scale: f32) -> PaintPublication {
        let mut runtime = AppRuntime::<FixtureApp>::mount(items);
        let tokens = StyleTokens::new();
        let logical_size =
            LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
                .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
        let raster_scale = RasterScale::new(scale)
            .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
            .with_raster_scale(raster_scale);
        runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
            .paint_publication()
            .clone()
    }

    fn pixel(readback: &crate::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
        let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
        readback.rgba8_srgb()[index..index + 4]
            .try_into()
            .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct ClipProofProbes {
        inside_both: Option<(u32, u32)>,
        first_only: Option<(u32, u32)>,
        second_only: Option<(u32, u32)>,
        rounded_corner_cut: Option<(u32, u32)>,
        transformed_inside: Option<(u32, u32)>,
    }

    fn find_clip_proof_probes(
        first_clip: SceneClip,
        second_clip: SceneClip,
        inverse_second: LogicalTransform,
        scale: f32,
    ) -> Result<ClipProofProbes, Box<dyn Error>> {
        let mut probes = ClipProofProbes::default();
        for y in 0_u16..63 {
            for x in 0_u16..84 {
                let point =
                    LogicalPoint::new((f32::from(x) + 0.5) / scale, (f32::from(y) + 0.5) / scale)?;
                let first = first_clip.contains_surface_point(point);
                let second = second_clip.contains_surface_point(point);
                let probe = (u32::from(x), u32::from(y));
                match (first, second) {
                    (true, true) => {
                        probes.inside_both.get_or_insert(probe);
                        if !second_clip.shape().contains(point) {
                            probes.transformed_inside.get_or_insert(probe);
                        }
                    }
                    (true, false) => {
                        probes.first_only.get_or_insert(probe);
                    }
                    (false, true) => {
                        probes.second_only.get_or_insert(probe);
                    }
                    (false, false) => {}
                }
                if first
                    && inverse_second.transform_point(point).is_some_and(|local| {
                        second_clip.shape().outer_rect().contains(local) && !second
                    })
                {
                    probes.rounded_corner_cut.get_or_insert(probe);
                }
            }
        }
        Ok(probes)
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "this test compares repeated renderer-prepared f32 protocol values byte-for-byte to prove there is no clip-count-dependent mutation"
    )]
    fn clip_preparation_has_no_stencil_count_ceiling_and_singular_clips_fail_closed()
    -> Result<(), Box<dyn Error>> {
        let rounded = SceneShape::rounded_rect(
            rect(8.0, 7.0, 20.0, 14.0),
            Radius::new(
                LogicalLength::new(10.0)?,
                LogicalLength::new(8.0)?,
                LogicalLength::new(6.0)?,
                LogicalLength::new(12.0)?,
            ),
        );
        let transform = LogicalTransform::try_new(1.0, 0.1, 0.25, 1.0, 5.0, 3.0)?;
        let clip = ContributionClip::new(rounded, transform);
        let mut many = PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), Color::WHITE);
        for _ in 0..300 {
            many = many.with_clip(clip);
        }
        let many_publication = publication(vec![many], 1.25);
        let fills = validate_clipped_scene_subset(&many_publication)?;
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].clips.len(), 300);
        let uniforms = prepare_clip_uniforms(&fills[0].clips, many_publication.raster_scale())
            .unwrap_or_else(|| unreachable!("all repeated clips are invertible"));
        assert_eq!(uniforms.len(), 300);
        assert_eq!(uniforms[0], uniforms[299]);

        let singular = LogicalTransform::try_new(1.0, 0.0, 0.0, 0.0, 2.0, 1.0)?;
        let singular_publication = publication(
            vec![
                PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), Color::WHITE)
                    .with_clip(ContributionClip::new(
                        SceneShape::rect(rect(0.0, 0.0, 64.0, 48.0)),
                        singular,
                    )),
            ],
            1.0,
        );
        let singular_fills = validate_clipped_scene_subset(&singular_publication)?;
        assert!(
            prepare_clip_uniforms(
                &singular_fills[0].clips,
                singular_publication.raster_scale(),
            )
            .is_none()
        );
        Ok(())
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "the prepared uniform must retain the canonical f32 inverse, scale, rectangle, and deterministic normalized radii exactly before GPU upload"
    )]
    fn rounded_clip_uniform_uses_canonical_inverse_and_common_radius_normalization()
    -> Result<(), Box<dyn Error>> {
        let shape = SceneShape::rounded_rect(
            rect(2.0, 3.0, 12.0, 10.0),
            Radius::new(
                LogicalLength::new(9.0)?,
                LogicalLength::new(7.0)?,
                LogicalLength::new(5.0)?,
                LogicalLength::new(11.0)?,
            ),
        );
        let transform = LogicalTransform::try_new(1.0, 0.25, -0.2, 1.0, 6.0, 4.0)?;
        let publication = publication(
            vec![
                PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), Color::WHITE)
                    .with_clip(ContributionClip::new(shape, transform)),
            ],
            2.0,
        );
        let scene_clip = publication.scene().items()[0].clips()[0];
        let uniform = ClipUniform::from_scene_clip(scene_clip, publication.raster_scale())
            .unwrap_or_else(|| unreachable!("fixture clip is invertible"));
        let inverse = scene_clip
            .clip_to_surface()
            .inverse()
            .unwrap_or_else(|| unreachable!("fixture clip is invertible"));
        assert_eq!(&uniform.values[..6], &inverse.components());
        assert_eq!(uniform.values[6], 2.0);
        assert_eq!(uniform.values[7], 1.0);
        assert_eq!(&uniform.values[8..12], &[2.0, 3.0, 14.0, 13.0]);
        assert!(uniform.values[12..].iter().all(|radius| *radius >= 0.0));
        assert!(uniform.values[12] + uniform.values[13] <= 12.0);
        assert!(uniform.values[15] + uniform.values[14] <= 12.0);
        assert!(uniform.values[12] + uniform.values[15] <= 10.0);
        assert!(uniform.values[13] + uniform.values[14] <= 10.0);
        Ok(())
    }

    #[test]
    fn clipped_validator_reuses_primitive_authority_while_base_validator_stays_fail_closed() {
        let clipped = PaintContributionItem::fill_rect(rect(1.0, 1.0, 10.0, 10.0), Color::WHITE)
            .with_clip(ContributionClip::identity(SceneShape::rect(rect(
                2.0, 2.0, 4.0, 4.0,
            ))));
        let publication = publication(vec![clipped], 1.0);
        assert!(validate_clipped_scene_subset(&publication).is_ok());
        assert!(matches!(
            crate::scene_subset::validate_scene_subset(&publication),
            Err(crate::scene_subset::SceneValidationError::UnsupportedItem {
                item_index: 0,
                semantic: crate::scene_subset::UnsupportedSceneSemantic::NonEmptyClips,
            })
        ));
    }

    #[test]
    fn real_gpu_conjunctive_transformed_rounded_and_singular_clips_match_scene_contract()
    -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let background = Color::rgb(0x25, 0x4D, 0x78);
        let clipped_color = Color::rgb(0xC3, 0x4A, 0x42);
        let singular_color = Color::rgb(0x37, 0x86, 0xC8);
        let first_clip = ContributionClip::identity(SceneShape::rect(rect(18.0, 12.0, 28.0, 28.0)));
        let rounded_shape = SceneShape::rounded_rect(
            rect(8.0, 8.0, 28.0, 22.0),
            Radius::all(LogicalLength::new(7.0)?),
        );
        let rounded_transform = LogicalTransform::try_new(1.0, 0.0, 0.25, 1.0, 14.0, 6.0)?;
        let second_clip = ContributionClip::new(rounded_shape, rounded_transform);
        let singular_transform = LogicalTransform::try_new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0)?;
        let publication = publication(
            vec![
                PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), background),
                PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), clipped_color)
                    .with_clip(first_clip)
                    .with_clip(second_clip),
                PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), singular_color)
                    .with_clip(ContributionClip::new(
                        SceneShape::rect(rect(0.0, 0.0, 64.0, 48.0)),
                        singular_transform,
                    )),
            ],
            1.3,
        );
        let clips = publication.scene().items()[1].clips();
        assert_eq!(clips.len(), 2);
        let inverse_second = clips[1]
            .clip_to_surface()
            .inverse()
            .unwrap_or_else(|| unreachable!("rounded fixture clip is invertible"));
        let output = renderer.render_offscreen_publication(&publication)?;
        assert_eq!(
            output.readback().extent(),
            crate::OffscreenExtent::new(84, 63)?
        );
        let scale = publication.raster_scale().get();
        let probes = find_clip_proof_probes(clips[0], clips[1], inverse_second, scale)?;

        let inside = probes
            .inside_both
            .unwrap_or_else(|| unreachable!("fixture has an intersection"));
        assert_eq!(
            pixel(output.readback(), inside.0, inside.1),
            [0xC3, 0x4A, 0x42, 0xFF]
        );
        for (label, probe) in [
            ("first clip alone", probes.first_only),
            ("second clip alone", probes.second_only),
            ("rounded outer-rect corner", probes.rounded_corner_cut),
        ] {
            let probe = probe.unwrap_or_else(|| unreachable!("fixture contains {label} probe"));
            assert_eq!(
                pixel(output.readback(), probe.0, probe.1),
                [0x25, 0x4D, 0x78, 0xFF],
                "{label} must be excluded by conjunctive clip coverage"
            );
        }
        let transformed = probes.transformed_inside.unwrap_or_else(|| {
            unreachable!("fixture proves transformed rather than local placement")
        });
        assert_eq!(
            pixel(output.readback(), transformed.0, transformed.1),
            [0xC3, 0x4A, 0x42, 0xFF],
            "the rounded clip must use clip-to-surface placement rather than its untransformed local shape"
        );
        eprintln!(
            "REAL GPU CLIP PROOF: fractional-scale conjunctive rect+affine-rounded clips, rounded-corner exclusion, transformed placement, and singular-clip fail-closed behavior match the M6 scene oracle; adapter={:?} backend={}",
            renderer.diagnostics().adapter_info().name,
            renderer.diagnostics().adapter_info().backend,
        );
        Ok(())
    }

    fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
        match block_on(Renderer::request(RendererOptions::new())) {
            Ok(renderer) => Ok(Some(renderer)),
            Err(RendererInitError::AdapterUnavailable {
                requested,
                compatible_surface_required,
                detail,
            }) => {
                eprintln!(
                    "native wgpu clip proof unavailable under {requested:?}; structured adapter failure: {detail}"
                );
                assert_eq!(requested, BackendSelection::AllNative);
                assert!(!compatible_surface_required);
                assert!(!detail.is_empty());
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    struct ThreadWake(thread::Thread);

    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => thread::park(),
            }
        }
    }
}

mod image;

use std::collections::HashSet;

use runenui_core::{ResourceKind, ResourceRef};
use runenui_runtime::SceneCapabilities;

use crate::{ResourceProvider, ResourceResolveError};

/// Structured failure while realizing one provider-backed paint publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationRenderError {
    /// Existing renderer/device/target/readback failure.
    Backend(super::OffscreenRenderError),
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

impl From<super::OffscreenRenderError> for PublicationRenderError {
    fn from(error: super::OffscreenRenderError) -> Self {
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
    pub async fn request(
        options: super::RendererOptions,
    ) -> Result<Self, super::RendererInitError> {
        Renderer::request(options).await.map(Self::from_literal)
    }

    /// Selects a native adapter using a caller-owned display connection.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request_with_display_handle(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, super::RendererInitError> {
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
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, super::RendererInitError> {
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
    pub const fn diagnostics(&self) -> &super::RendererDiagnostics {
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
    #[must_use]
    pub fn discard_resource_cache(&mut self) -> bool {
        self.images.discard_cache()
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
    pub fn render_offscreen_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
    ) -> Result<super::OffscreenPublicationReadback, PublicationRenderError> {
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

        let scene =
            validate_resource_scene_subset(publication).map_err(super::scene_validation_error)?;
        let (canvas_extent, extent) = super::publication_extents(publication)?;
        self.literal.base.validate_extent(extent)?;
        let layout = super::ReadbackLayout::new(extent)?;
        self.literal.base.validate_readback_buffer(layout)?;

        let retained_target_matches = self
            .literal
            .base
            .offscreen_target
            .as_ref()
            .is_some_and(|target| target.matches(extent, super::OFFSCREEN_FORMAT));
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
                    .ensure_fill_rect_pipeline(super::OFFSCREEN_FORMAT)?;
            }
            if needs_stencil {
                self.literal.ensure_clip_pipelines(super::OFFSCREEN_FORMAT)?;
            }
            self.images
                .ensure_pipelines(&self.literal.base.device, super::OFFSCREEN_FORMAT)?;
            self.images
                .realize(&self.literal.base.device, &self.literal.base.queue, resolved);

            if !retained_target_matches {
                let target = self.literal.base.create_offscreen_target(extent)?;
                self.literal.base.offscreen_target = Some(target);
            }
        }

        let readback = self.literal.base.create_readback_buffer(layout);
        let mut encoder = self.literal.base.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("runenui provider-backed offscreen publication encoder"),
            },
        );
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

        super::encode_target_copy(&mut encoder, &target.texture, &readback, extent, layout);
        let submission = self.literal.base.queue.submit([encoder.finish()]);
        let rgba8_srgb = match self.literal.base.map_readback(&readback, layout, submission) {
            Ok(pixels) => pixels,
            Err(error) => {
                self.literal.base.offscreen_target = None;
                return Err(PublicationRenderError::Backend(error));
            }
        };
        let readback = super::OffscreenReadback {
            extent,
            format: super::OFFSCREEN_FORMAT,
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
        Ok(super::OffscreenPublicationReadback {
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

    /// Executes one real wgpu render-pass clear and returns actual texture bytes from GPU readback.
    ///
    /// # Errors
    ///
    /// Returns structured extent, device-wait, buffer-map, or mapped-range failures.
    pub fn clear_offscreen(
        &self,
        extent: super::OffscreenExtent,
        color: Color,
    ) -> Result<super::OffscreenReadback, super::OffscreenRenderError> {
        self.literal.clear_offscreen(extent, color)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ResourceSceneItem {
    Literal(LiteralRectItem),
    Image(ImageSceneItem),
}

impl ResourceSceneItem {
    fn needs_stencil(&self) -> bool {
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
    let unsupported_resource_kind = capabilities
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
    extent: super::OffscreenExtent,
    canvas_extent: super::RasterCanvasExtent,
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
    extent: super::OffscreenExtent,
    canvas_extent: super::RasterCanvasExtent,
    raster_scale: RasterScale,
    item: &LiteralRectItem,
) {
    let vertex_bytes = super::fill_rect_vertex_bytes(
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
        let Some(uniform) = stroke_mask::StrokeMaskUniform::from_literal(&item.literal, raster_scale)
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
    extent: super::OffscreenExtent,
    canvas_extent: super::RasterCanvasExtent,
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
