//! Disposable wgpu realization of preflighted ordinary-shadow alpha masks.
//!
//! Alpha masks are derived exclusively from ADR 0015 neutral support before this
//! module is reached. This module owns no support semantics; it only uploads one
//! cropped mask and composites the authored shadow color in painter order.

use std::collections::HashMap;

use runenui_core::Color;
use wgpu::util::DeviceExt;

use super::super::super::super::{OffscreenRenderError, srgb8_to_linear_f32};
use super::mask::AlphaMask;

const SHADOW_UNIFORM_SIZE: usize = 32;

const SHADOW_SHADER: &str = r"
struct ShadowUniform {
    origin_and_size: vec4<u32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var shadow_mask: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> shadow: ShadowUniform;

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

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<u32>(position.xy);
    let origin = shadow.origin_and_size.xy;
    let size = shadow.origin_and_size.zw;
    if (pixel.x < origin.x || pixel.y < origin.y ||
        pixel.x >= origin.x + size.x || pixel.y >= origin.y + size.y) {
        discard;
    }
    let local = pixel - origin;
    let coverage = textureLoad(shadow_mask, vec2<i32>(local), 0).r;
    let alpha = coverage * shadow.color.a;
    if (alpha <= 0.0) {
        discard;
    }
    return vec4<f32>(shadow.color.rgb * alpha, alpha);
}
";

#[derive(Debug)]
pub(super) struct ShadowRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
}

impl ShadowRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui ordinary-shadow bind-group layout"),
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        Self {
            bind_group_layout,
            pipelines: HashMap::new(),
        }
    }

    pub(super) fn ensure_pipeline(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), OffscreenRenderError> {
        if !matches!(
            target_format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(OffscreenRenderError::UnsupportedTargetFormat {
                format: target_format,
            });
        }
        if !self.pipelines.contains_key(&target_format) {
            self.pipelines.insert(
                target_format,
                create_pipeline(device, target_format, &self.bind_group_layout),
            );
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "ordinary-shadow realization keeps the exact disposable mask, authored color, queue upload, destination target, and target format explicit"
    )]
    pub(super) fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        mask: &AlphaMask,
        color: Color,
    ) {
        if color.alpha() == 0 {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("runenui ordinary-shadow alpha mask"),
            size: wgpu::Extent3d {
                width: mask.width(),
                height: mask.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
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
            mask.alpha().as_ref(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(mask.width()),
                rows_per_image: Some(mask.height()),
            },
            wgpu::Extent3d {
                width: mask.width(),
                height: mask.height(),
                depth_or_array_layers: 1,
            },
        );
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let uniform = shadow_uniform(mask, color);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui ordinary-shadow uniform"),
            contents: &uniform,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("runenui ordinary-shadow bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        let pipeline = self
            .pipelines
            .get(&target_format)
            .unwrap_or_else(|| unreachable!("ordinary-shadow target pipeline was ensured"));
        let color_attachment = Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("runenui ordinary-shadow composite pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

fn shadow_uniform(mask: &AlphaMask, color: Color) -> [u8; SHADOW_UNIFORM_SIZE] {
    let mut bytes = [0_u8; SHADOW_UNIFORM_SIZE];
    for (index, value) in [
        mask.origin_x(),
        mask.origin_y(),
        mask.width(),
        mask.height(),
    ]
    .into_iter()
    .enumerate()
    {
        let offset = index * 4;
        bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }
    let alpha = f32::from(color.alpha()) / 255.0;
    for (index, value) in [
        srgb8_to_linear_f32(color.red()),
        srgb8_to_linear_f32(color.green()),
        srgb8_to_linear_f32(color.blue()),
        alpha,
    ]
    .into_iter()
    .enumerate()
    {
        let offset = 16 + index * 4;
        bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn create_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui ordinary-shadow shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui ordinary-shadow pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui ordinary-shadow pipeline"),
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
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(premultiplied_source_over()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

const fn premultiplied_source_over() -> wgpu::BlendState {
    let component = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}
