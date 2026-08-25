use std::collections::{HashMap, HashSet};

use runenui_core::{Color, LogicalPoint, LogicalRect, LogicalTransform, ResourceRef, SceneOpacity};
use runenui_runtime::RasterScale;

use crate::{
    ResourcePayload, ResourceProvider, ResourceRequest, ResourceResolveError, ShapedRunRaster,
    scene_subset::SupportedFillRect,
};

const SHAPED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
const SHAPED_VERTEX_STRIDE: u64 = 36;
const SHAPED_SHADER: &str = r"
@group(0) @binding(0)
var coverage_texture: texture_2d<f32>;

@group(0) @binding(1)
var coverage_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
    @location(3) foreground: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) foreground: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.opacity = input.opacity;
    output.foreground = input.foreground;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(coverage_texture, coverage_sampler, input.uv).r;
    return vec4<f32>(input.foreground.rgb, input.foreground.a * input.opacity * coverage);
}
";

const SHAPED_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 8,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 16,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 20,
        shader_location: 3,
    },
];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SupportedShapedRun {
    pub(super) item_index: usize,
    pub(super) resource: ResourceRef,
    pub(super) origin: LogicalPoint,
    pub(super) foreground: Color,
    pub(super) opacity: SceneOpacity,
    pub(super) local_to_surface: LogicalTransform,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ShapedRunCacheKey {
    resource: ResourceRef,
    raster_scale_bits: u32,
}

impl ShapedRunCacheKey {
    fn new(resource: &ResourceRef, raster_scale: RasterScale) -> Self {
        Self {
            resource: resource.clone(),
            raster_scale_bits: raster_scale.get().to_bits(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedShapedRun {
    key: ShapedRunCacheKey,
    payload: ShapedRunRaster,
}

impl ResolvedShapedRun {
    const fn key(&self) -> &ShapedRunCacheKey {
        &self.key
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.payload.width() == 0 || self.payload.height() == 0
    }

    pub(super) fn resource_key(&self) -> (ResourceRef, u32) {
        (self.key.resource.clone(), self.key.raster_scale_bits)
    }
}

#[derive(Debug)]
struct ShapedRunRealization {
    payload: ShapedRunRaster,
    _texture: Option<wgpu::Texture>,
    bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug)]
struct ShapedRunTargetPipelines {
    ordinary: wgpu::RenderPipeline,
    clipped: wgpu::RenderPipeline,
}

#[derive(Debug)]
pub(super) struct ShapedRunRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipelines: HashMap<wgpu::TextureFormat, ShapedRunTargetPipelines>,
    cache: HashMap<ShapedRunCacheKey, ShapedRunRealization>,
}

impl ShapedRunRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui shaped-run bind-group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("runenui shaped-run nearest sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Self {
            bind_group_layout,
            sampler,
            pipelines: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    pub(super) fn contains(&self, resource: &ResourceRef, raster_scale: RasterScale) -> bool {
        self.cache
            .contains_key(&ShapedRunCacheKey::new(resource, raster_scale))
    }

    pub(super) fn ensure_pipelines(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), super::super::OffscreenRenderError> {
        if !matches!(
            target_format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(
                super::super::OffscreenRenderError::UnsupportedTargetFormat {
                    format: target_format,
                },
            );
        }
        if !self.pipelines.contains_key(&target_format) {
            let ordinary =
                create_shaped_pipeline(device, target_format, &self.bind_group_layout, false);
            let clipped =
                create_shaped_pipeline(device, target_format, &self.bind_group_layout, true);
            self.pipelines.insert(
                target_format,
                ShapedRunTargetPipelines { ordinary, clipped },
            );
        }
        Ok(())
    }

    pub(super) fn realize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resolved: Vec<ResolvedShapedRun>,
    ) {
        for resolved in resolved {
            if self.cache.contains_key(resolved.key()) {
                continue;
            }
            if resolved.payload.width() == 0 || resolved.payload.height() == 0 {
                self.cache.insert(
                    resolved.key,
                    ShapedRunRealization {
                        payload: resolved.payload,
                        _texture: None,
                        bind_group: None,
                    },
                );
                continue;
            }
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("runenui shaped-run coverage realization"),
                size: wgpu::Extent3d {
                    width: resolved.payload.width(),
                    height: resolved.payload.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SHAPED_FORMAT,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                resolved.payload.alpha8(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(resolved.payload.width()),
                    rows_per_image: Some(resolved.payload.height()),
                },
                wgpu::Extent3d {
                    width: resolved.payload.width(),
                    height: resolved.payload.height(),
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("runenui shaped-run bind group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.cache.insert(
                resolved.key,
                ShapedRunRealization {
                    payload: resolved.payload,
                    _texture: Some(texture),
                    bind_group: Some(bind_group),
                },
            );
        }
    }

    pub(super) fn retain(&mut self, live: &HashSet<(ResourceRef, u32)>) {
        self.cache
            .retain(|key, _| live.contains(&(key.resource.clone(), key.raster_scale_bits)));
    }

    pub(super) fn discard_cache(&mut self) -> bool {
        let had_entries = !self.cache.is_empty();
        self.cache.clear();
        had_entries
    }

    pub(super) fn raster(
        &self,
        resource: &ResourceRef,
        raster_scale: RasterScale,
    ) -> &ShapedRunRaster {
        &self
            .cache
            .get(&ShapedRunCacheKey::new(resource, raster_scale))
            .unwrap_or_else(|| unreachable!("shaped-run raster is realized before drawing"))
            .payload
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the shaped-run draw boundary keeps target format, color/stencil attachments, exact scale-qualified cached resource identity, and the prepared vertex buffer explicit"
    )]
    pub(super) fn draw(
        &self,
        target_format: wgpu::TextureFormat,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: Option<&wgpu::TextureView>,
        resource: &ResourceRef,
        raster_scale: RasterScale,
        vertex_buffer: &wgpu::Buffer,
        vertex_count: u32,
    ) {
        let pipelines = self.pipelines.get(&target_format).unwrap_or_else(|| {
            unreachable!("shaped-run target pipelines are cached before drawing")
        });
        let realization = self
            .cache
            .get(&ShapedRunCacheKey::new(resource, raster_scale))
            .unwrap_or_else(|| unreachable!("shaped-run resource is realized before drawing"));
        let Some(bind_group) = realization.bind_group.as_ref() else {
            return;
        };
        let color_attachment = Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        });
        let stencil_attachment = stencil_view.map(|view| wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: None,
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
        });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("runenui ordered shaped-run pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if stencil_view.is_some() {
            render_pass.set_pipeline(&pipelines.clipped);
            render_pass.set_stencil_reference(super::STENCIL_ALLOWED);
        } else {
            render_pass.set_pipeline(&pipelines.ordinary);
        }
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_count, 0..1);
    }
}

pub(super) fn resolve_shaped_run(
    provider: &(impl ResourceProvider + ?Sized),
    shaped_run: &SupportedShapedRun,
    raster_scale: RasterScale,
    max_texture_dimension_2d: u32,
) -> Result<ResolvedShapedRun, ShapedRunResolveFailure> {
    let payload = match crate::resolve_resource(
        provider,
        &shaped_run.resource,
        ResourceRequest::ShapedTextRun { raster_scale },
    ) {
        Ok(ResourcePayload::ShapedTextRun(payload)) => payload,
        Ok(ResourcePayload::Image(_)) => {
            unreachable!("the shared resolver rejects payload-kind mismatch")
        }
        Err(error) => return Err(ShapedRunResolveFailure::Resource(error)),
    };
    if payload.width() > max_texture_dimension_2d || payload.height() > max_texture_dimension_2d {
        return Err(ShapedRunResolveFailure::ExtentExceedsDeviceLimit {
            width: payload.width(),
            height: payload.height(),
            max_texture_dimension_2d,
        });
    }
    Ok(ResolvedShapedRun {
        key: ShapedRunCacheKey::new(&shaped_run.resource, raster_scale),
        payload,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ShapedRunResolveFailure {
    Resource(ResourceResolveError),
    ExtentExceedsDeviceLimit {
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    },
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "validated provider extents and normalized UVs are narrowed only at the renderer-local f32 GPU ABI boundary"
)]
pub(super) fn vertex_bytes(
    shaped_run: &SupportedShapedRun,
    raster: &ShapedRunRaster,
    extent: super::super::OffscreenExtent,
    canvas_extent: super::super::RasterCanvasExtent,
    raster_scale: RasterScale,
) -> Vec<u8> {
    if raster.width() == 0 || raster.height() == 0 {
        return Vec::new();
    }
    let Some(origin) = LogicalPoint::new(
        shaped_run.origin.x() + raster.logical_origin().x(),
        shaped_run.origin.y() + raster.logical_origin().y(),
    )
    .ok() else {
        return Vec::new();
    };
    let Ok(rect) = LogicalRect::try_new(
        origin.x(),
        origin.y(),
        f64::from(raster.width()) as f32 / raster_scale.get(),
        f64::from(raster.height()) as f32 / raster_scale.get(),
    ) else {
        return Vec::new();
    };
    let Ok(local_to_surface) = LogicalTransform::translation(0.0, 0.0)
        .and_then(|identity| identity.then(shaped_run.local_to_surface))
    else {
        return Vec::new();
    };
    let fill = SupportedFillRect {
        rect,
        color: shaped_run.foreground,
        opacity: shaped_run.opacity,
        local_to_surface,
    };
    let polygon = super::super::transformed_fill_polygon(&fill, canvas_extent, raster_scale);
    if polygon.len() < 3 {
        return Vec::new();
    }
    let Some(surface_to_local) = local_to_surface.inverse() else {
        return Vec::new();
    };
    let [m11, m12, m21, m22, tx, ty] = surface_to_local.components().map(f64::from);
    let scale = f64::from(raster_scale.get());
    let origin_x = f64::from(rect.x());
    let origin_y = f64::from(rect.y());
    let width = f64::from(rect.width());
    let height = f64::from(rect.height());
    let foreground = [
        super::super::srgb8_to_linear_f32(shaped_run.foreground.red()),
        super::super::srgb8_to_linear_f32(shaped_run.foreground.green()),
        super::super::srgb8_to_linear_f32(shaped_run.foreground.blue()),
        f32::from(shaped_run.foreground.alpha()) / 255.0,
    ];
    let vertices = polygon
        .into_iter()
        .map(|point| {
            let surface_x = point[0] / scale;
            let surface_y = point[1] / scale;
            let local_x = m11.mul_add(surface_x, m21.mul_add(surface_y, tx));
            let local_y = m12.mul_add(surface_x, m22.mul_add(surface_y, ty));
            let uv = [
                ((local_x - origin_x) / width).clamp(0.0, 1.0) as f32,
                ((local_y - origin_y) / height).clamp(0.0, 1.0) as f32,
            ];
            (
                super::super::physical_point_to_ndc(point, extent),
                uv,
                shaped_run.opacity.get(),
                foreground,
            )
        })
        .collect::<Vec<_>>();
    let mut bytes = Vec::with_capacity(
        vertices
            .len()
            .saturating_mul(3 * SHAPED_VERTEX_STRIDE as usize),
    );
    for index in 1..vertices.len() - 1 {
        for (position, uv, opacity, foreground) in
            [vertices[0], vertices[index], vertices[index + 1]]
        {
            push_shaped_vertex(&mut bytes, position, uv, opacity, foreground);
        }
    }
    bytes
}

fn push_shaped_vertex(
    bytes: &mut Vec<u8>,
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    foreground: [f32; 4],
) {
    for component in position
        .into_iter()
        .chain(uv)
        .chain([opacity])
        .chain(foreground)
    {
        bytes.extend_from_slice(&component.to_ne_bytes());
    }
}

fn create_shaped_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    clipped: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui shaped-run shader"),
        source: wgpu::ShaderSource::Wgsl(SHAPED_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui shaped-run pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if clipped {
            "runenui clipped shaped-run pipeline"
        } else {
            "runenui shaped-run pipeline"
        }),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: SHAPED_VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &SHAPED_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: clipped.then(super::clipped_fill_stencil_state),
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
