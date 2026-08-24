use runenui_runtime::RasterScale;
use wgpu::util::DeviceExt;

use crate::scene_subset::SupportedLiteralRect;

const STROKE_MASK_SHADER: &str = r"
struct StrokeUniform {
    transform_a: vec4<f32>,
    transform_b: vec4<f32>,
    inset: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> stroke: StrokeUniform;

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
fn fs_main(input: VertexOutput) {
    let surface = input.position.xy / stroke.transform_b.z;
    let local = vec2<f32>(
        stroke.transform_a.x * surface.x
            + stroke.transform_a.z * surface.y
            + stroke.transform_b.x,
        stroke.transform_a.y * surface.x
            + stroke.transform_a.w * surface.y
            + stroke.transform_b.y,
    );
    let inside_inset = local.x >= stroke.inset.x
        && local.x < stroke.inset.z
        && local.y >= stroke.inset.y
        && local.y < stroke.inset.w;
    if !inside_inset {
        discard;
    }
}
";

#[derive(Debug)]
pub(super) struct StrokeMaskPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl StrokeMaskPipeline {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui stroke-inset bind-group layout"),
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
            label: Some("runenui stroke-inset pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("runenui stroke-inset shader"),
            source: wgpu::ShaderSource::Wgsl(STROKE_MASK_SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("runenui stroke-inset pipeline"),
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
            depth_stencil: Some(super::mask_stencil_state()),
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct StrokeMaskUniform {
    values: [f32; 12],
}

impl StrokeMaskUniform {
    pub(super) fn from_literal(
        literal: &SupportedLiteralRect,
        raster_scale: RasterScale,
    ) -> Option<Self> {
        let inset = literal.stroke_inset?;
        let surface_to_local = literal.fill.local_to_surface.inverse()?;
        let [m11, m12, m21, m22, tx, ty] = surface_to_local.components();
        Some(Self {
            values: [
                m11,
                m12,
                m21,
                m22,
                tx,
                ty,
                raster_scale.get(),
                0.0,
                inset.x(),
                inset.y(),
                inset.max_x(),
                inset.max_y(),
            ],
        })
    }

    fn bytes(self) -> [u8; 48] {
        let mut bytes = [0_u8; 48];
        for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(self.values) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

pub(super) fn apply_stroke_mask(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &StrokeMaskPipeline,
    uniform: StrokeMaskUniform,
) {
    let uniform_bytes = uniform.bytes();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui stroke-inset uniform"),
        contents: &uniform_bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("runenui stroke-inset bind group"),
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
        label: Some("runenui centered stroke inset-mask pass"),
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
