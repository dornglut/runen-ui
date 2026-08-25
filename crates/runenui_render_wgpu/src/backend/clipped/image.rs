use std::collections::{HashMap, HashSet};

use runenui_core::{Color, LogicalRect, LogicalTransform, ResourceRef, SceneOpacity};
use runenui_runtime::RasterScale;

use crate::{
    ImagePayload, ResourcePayload, ResourceProvider, ResourceRequest, ResourceResolveError,
    scene_subset::SupportedFillRect,
};

const IMAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const IMAGE_VERTEX_STRIDE: u64 = 20;
const IMAGE_SHADER: &str = r"
@group(0) @binding(0)
var image_texture: texture_2d<f32>;

@group(0) @binding(1)
var image_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
}

struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.opacity = input.opacity;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(image_texture, image_sampler, input.uv);
    return vec4<f32>(source.rgb, source.a * input.opacity);
}
";

const IMAGE_ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
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
];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SupportedImage {
    pub(super) item_index: usize,
    pub(super) resource: ResourceRef,
    pub(super) destination: LogicalRect,
    pub(super) opacity: SceneOpacity,
    pub(super) local_to_surface: LogicalTransform,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedImage {
    resource: ResourceRef,
    payload: ImagePayload,
    row_bytes: u32,
}

impl ResolvedImage {
    pub(super) const fn resource(&self) -> &ResourceRef {
        &self.resource
    }
}

#[derive(Debug)]
struct ImageRealization {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
struct ImageTargetPipelines {
    ordinary: wgpu::RenderPipeline,
    clipped: wgpu::RenderPipeline,
}

#[derive(Debug)]
pub(super) struct ImageRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipelines: HashMap<wgpu::TextureFormat, ImageTargetPipelines>,
    cache: HashMap<ResourceRef, ImageRealization>,
}

impl ImageRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui image bind-group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some("runenui image nearest sampler"),
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

    pub(super) fn contains(&self, resource: &ResourceRef) -> bool {
        self.cache.contains_key(resource)
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
                create_image_pipeline(device, target_format, &self.bind_group_layout, false);
            let clipped =
                create_image_pipeline(device, target_format, &self.bind_group_layout, true);
            self.pipelines
                .insert(target_format, ImageTargetPipelines { ordinary, clipped });
        }
        Ok(())
    }

    pub(super) fn realize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resolved: Vec<ResolvedImage>,
    ) {
        for resolved in resolved {
            if self.cache.contains_key(resolved.resource()) {
                continue;
            }
            let payload = &resolved.payload;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("runenui image realization"),
                size: wgpu::Extent3d {
                    width: payload.width(),
                    height: payload.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: IMAGE_FORMAT,
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
                payload.rgba8_srgb(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(resolved.row_bytes),
                    rows_per_image: Some(payload.height()),
                },
                wgpu::Extent3d {
                    width: payload.width(),
                    height: payload.height(),
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("runenui image bind group"),
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
                resolved.resource,
                ImageRealization {
                    _texture: texture,
                    bind_group,
                },
            );
        }
    }

    pub(super) fn retain(&mut self, live: &HashSet<ResourceRef>) {
        self.cache.retain(|resource, _| live.contains(resource));
    }

    pub(super) fn discard_cache(&mut self) -> bool {
        let had_entries = !self.cache.is_empty();
        self.cache.clear();
        had_entries
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the image draw boundary keeps target format, color/stencil attachments, exact cached resource identity, and the already-prepared vertex buffer explicit"
    )]
    pub(super) fn draw(
        &self,
        target_format: wgpu::TextureFormat,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: Option<&wgpu::TextureView>,
        resource: &ResourceRef,
        vertex_buffer: &wgpu::Buffer,
        vertex_count: u32,
    ) {
        let pipelines = self
            .pipelines
            .get(&target_format)
            .unwrap_or_else(|| unreachable!("image target pipelines are cached before drawing"));
        let realization = self
            .cache
            .get(resource)
            .unwrap_or_else(|| unreachable!("image resource is realized before drawing"));
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
            label: Some("runenui ordered image pass"),
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
        render_pass.set_bind_group(0, &realization.bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_count, 0..1);
    }
}

pub(super) fn resolve_image(
    provider: &(impl ResourceProvider + ?Sized),
    image: &SupportedImage,
    max_texture_dimension_2d: u32,
) -> Result<ResolvedImage, ImageResolveFailure> {
    let payload = match crate::resolve_resource(provider, &image.resource, ResourceRequest::Image) {
        Ok(ResourcePayload::Image(payload)) => payload,
        Ok(ResourcePayload::ShapedTextRun(_)) => {
            unreachable!("the shared resolver rejects payload-kind mismatch")
        }
        Err(error) => return Err(ImageResolveFailure::Resource(error)),
    };
    if payload.width() > max_texture_dimension_2d || payload.height() > max_texture_dimension_2d {
        return Err(ImageResolveFailure::ExtentExceedsDeviceLimit {
            width: payload.width(),
            height: payload.height(),
            max_texture_dimension_2d,
        });
    }
    let Some(row_bytes) = payload.width().checked_mul(4) else {
        return Err(ImageResolveFailure::RowBytesOverflow {
            width: payload.width(),
        });
    };
    Ok(ResolvedImage {
        resource: image.resource.clone(),
        payload,
        row_bytes,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ImageResolveFailure {
    Resource(ResourceResolveError),
    ExtentExceedsDeviceLimit {
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    },
    RowBytesOverflow {
        width: u32,
    },
}

pub(super) fn vertex_bytes(
    image: &SupportedImage,
    extent: super::super::OffscreenExtent,
    canvas_extent: super::super::RasterCanvasExtent,
    raster_scale: RasterScale,
) -> Vec<u8> {
    if image.destination.width() == 0.0 || image.destination.height() == 0.0 {
        return Vec::new();
    }
    let fill = SupportedFillRect {
        rect: image.destination,
        color: Color::WHITE,
        opacity: image.opacity,
        local_to_surface: image.local_to_surface,
    };
    let polygon = super::super::transformed_fill_polygon(&fill, canvas_extent, raster_scale);
    if polygon.len() < 3 {
        return Vec::new();
    }
    let Some(surface_to_local) = image.local_to_surface.inverse() else {
        return Vec::new();
    };
    let [m11, m12, m21, m22, tx, ty] = surface_to_local.components().map(f64::from);
    let scale = f64::from(raster_scale.get());
    let destination_x = f64::from(image.destination.x());
    let destination_y = f64::from(image.destination.y());
    let destination_width = f64::from(image.destination.width());
    let destination_height = f64::from(image.destination.height());
    let vertices = polygon
        .into_iter()
        .map(|point| {
            let surface_x = point[0] / scale;
            let surface_y = point[1] / scale;
            let local_x = m11.mul_add(surface_x, m21.mul_add(surface_y, tx));
            let local_y = m12.mul_add(surface_x, m22.mul_add(surface_y, ty));
            let uv = [
                narrow_uv((local_x - destination_x) / destination_width),
                narrow_uv((local_y - destination_y) / destination_height),
            ];
            (
                super::super::physical_point_to_ndc(point, extent),
                uv,
                image.opacity.get(),
            )
        })
        .collect::<Vec<_>>();
    let mut bytes = Vec::with_capacity(vertices.len().saturating_mul(3 * 20));
    for index in 1..vertices.len() - 1 {
        for (position, uv, opacity) in [vertices[0], vertices[index], vertices[index + 1]] {
            push_image_vertex(&mut bytes, position, uv, opacity);
        }
    }
    bytes
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "exact-canvas clipping plus canonical inverse reconstruction bounds normalized image coordinates to the finite unit resource domain before the f32 GPU ABI"
)]
const fn narrow_uv(value: f64) -> f32 {
    value.clamp(0.0, 1.0) as f32
}

fn push_image_vertex(bytes: &mut Vec<u8>, position: [f32; 2], uv: [f32; 2], opacity: f32) {
    for component in position.into_iter().chain(uv).chain([opacity]) {
        bytes.extend_from_slice(&component.to_ne_bytes());
    }
}

fn create_image_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    clipped: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui image shader"),
        source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui image pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if clipped {
            "runenui clipped image pipeline"
        } else {
            "runenui image pipeline"
        }),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: IMAGE_VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &IMAGE_ATTRIBUTES,
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

#[cfg(test)]
mod tests {
    use runenui_core::{LogicalRect, LogicalTransform, ResourceKind, ResourceRef, SceneOpacity};
    use runenui_runtime::RasterScale;

    use super::{SupportedImage, vertex_bytes};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn image(destination: LogicalRect, transform: LogicalTransform) -> SupportedImage {
        SupportedImage {
            item_index: 0,
            resource: ResourceRef::new(ResourceKind::Image),
            destination,
            opacity: SceneOpacity::OPAQUE,
            local_to_surface: transform,
        }
    }

    fn uv_vertices(bytes: &[u8]) -> Vec<[f32; 2]> {
        bytes
            .as_chunks::<20>()
            .0
            .iter()
            .map(|vertex| {
                let u = f32::from_ne_bytes(
                    vertex[8..12]
                        .try_into()
                        .unwrap_or_else(|_| unreachable!("u bytes are complete")),
                );
                let v = f32::from_ne_bytes(
                    vertex[12..16]
                        .try_into()
                        .unwrap_or_else(|_| unreachable!("v bytes are complete")),
                );
                [u, v]
            })
            .collect()
    }

    #[test]
    fn exact_canvas_clipping_reconstructs_normalized_image_domain()
    -> Result<(), Box<dyn std::error::Error>> {
        let transform = LogicalTransform::try_new(1.0, 0.0, 0.0, 1.0, -10.0, 0.0)?;
        let image = image(rect(0.0, 0.0, 20.0, 10.0), transform);
        let extent = super::super::super::OffscreenExtent::new(64, 48)?;
        let canvas = super::super::super::RasterCanvasExtent::new(64.0, 48.0);
        let bytes = vertex_bytes(&image, extent, canvas, RasterScale::ONE);
        let vertices = uv_vertices(&bytes);
        assert!(!vertices.is_empty());
        let min_u = vertices.iter().map(|uv| uv[0]).fold(1.0_f32, f32::min);
        let max_u = vertices.iter().map(|uv| uv[0]).fold(0.0_f32, f32::max);
        assert!((min_u - 0.5).abs() <= f32::EPSILON);
        assert!((max_u - 1.0).abs() <= f32::EPSILON);
        assert!(vertices.iter().all(|uv| (0.0..=1.0).contains(&uv[1])));
        Ok(())
    }

    #[test]
    fn singular_image_transform_produces_no_vertices() -> Result<(), Box<dyn std::error::Error>> {
        let singular = LogicalTransform::try_new(1.0, 0.0, 0.0, 0.0, 2.0, 1.0)?;
        let image = image(rect(2.0, 3.0, 20.0, 10.0), singular);
        let extent = super::super::super::OffscreenExtent::new(64, 48)?;
        let canvas = super::super::super::RasterCanvasExtent::new(64.0, 48.0);
        assert!(vertex_bytes(&image, extent, canvas, RasterScale::ONE).is_empty());
        Ok(())
    }
}
