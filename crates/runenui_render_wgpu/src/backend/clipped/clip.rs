//! Disposable all-`SceneShape` clip realization for the production wgpu renderer.
//!
//! Clip geometry is prepared from the same private fill tessellation substrate used
//! by ordinary paint geometry. The resulting triangles are only a disposable
//! coverage decomposition: one temporary stencil bit records clip coverage and a
//! second pass intersects that coverage with the item's existing allowed bit.

use runenui_core::LogicalTransform;
use runenui_runtime::{RasterScale, SceneClip};
use wgpu::util::DeviceExt;

use crate::tessellation::{TessellatedGeometry, TessellationError, tessellate_fill};

use super::super::{
    OffscreenExtent, RasterCanvasExtent, clip_polygon_to_canvas, physical_point_to_ndc,
};
use super::{STENCIL_ALLOWED, STENCIL_FORMAT};

const CLIP_TEMP: u32 = 1 << 1;
const CLIP_VERTEX_SIZE: usize = 8;
const CLIP_VERTEX_STRIDE: u64 = 8;

const CLIP_SHADER: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
}

@vertex
fn vs_geometry(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    return output;
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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
fn fs_main() {}
";

const CLIP_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x2,
    offset: 0,
    shader_location: 0,
}];

/// One renderer-private clip geometry prepared before any retained target mutation.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreparedClip {
    geometry: TessellatedGeometry,
    clip_to_surface: LogicalTransform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ClipPrepareError {
    clip_index: usize,
    error: TessellationError,
}

impl ClipPrepareError {
    pub(super) const fn clip_index(self) -> usize {
        self.clip_index
    }

    pub(super) const fn error(self) -> TessellationError {
        self.error
    }
}

/// Converts every accepted clip shape into disposable fill geometry before rendering mutates a
/// retained target. Rectangles, rounded rectangles, ellipses, and paths therefore share one
/// realization contract.
pub(super) fn prepare_clips(clips: &[SceneClip]) -> Result<Vec<PreparedClip>, ClipPrepareError> {
    clips
        .iter()
        .enumerate()
        .map(|(clip_index, clip)| {
            tessellate_fill(clip.shape())
                .map(|geometry| PreparedClip {
                    geometry,
                    clip_to_surface: clip.clip_to_surface(),
                })
                .map_err(|error| ClipPrepareError { clip_index, error })
        })
        .collect()
}

#[derive(Debug)]
struct ClipPipelines {
    coverage: wgpu::RenderPipeline,
    intersect: wgpu::RenderPipeline,
    clear_temp: wgpu::RenderPipeline,
}

/// Renderer-owned disposable stencil realization for conjunctive clips.
#[derive(Debug)]
pub(super) struct ClipRenderer {
    pipelines: ClipPipelines,
}

impl ClipRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("runenui generic clip shader"),
            source: wgpu::ShaderSource::Wgsl(CLIP_SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("runenui generic clip pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        Self {
            pipelines: ClipPipelines {
                coverage: create_clip_pipeline(
                    device,
                    &layout,
                    &shader,
                    "runenui clip coverage pipeline",
                    "vs_geometry",
                    clip_coverage_stencil_state(),
                    &[Some(wgpu::VertexBufferLayout {
                        array_stride: CLIP_VERTEX_STRIDE,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &CLIP_ATTRIBUTES,
                    })],
                ),
                intersect: create_clip_pipeline(
                    device,
                    &layout,
                    &shader,
                    "runenui clip intersection pipeline",
                    "vs_fullscreen",
                    clip_intersection_stencil_state(),
                    &[],
                ),
                clear_temp: create_clip_pipeline(
                    device,
                    &layout,
                    &shader,
                    "runenui clip temporary-bit clear pipeline",
                    "vs_fullscreen",
                    clip_temp_clear_stencil_state(),
                    &[],
                ),
            },
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "generic clip realization keeps target/canvas/scale and the existing stencil authority explicit"
    )]
    pub(super) fn apply_clips(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        stencil_view: &wgpu::TextureView,
        extent: OffscreenExtent,
        canvas_extent: RasterCanvasExtent,
        raster_scale: RasterScale,
        clips: &[PreparedClip],
    ) {
        for clip in clips {
            let vertex_bytes = clip_vertex_bytes(clip, extent, canvas_extent, raster_scale);
            if !vertex_bytes.is_empty() {
                let vertex_count =
                    u32::try_from(vertex_bytes.len() / CLIP_VERTEX_SIZE).unwrap_or(u32::MAX);
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("runenui generic clip coverage vertices"),
                    contents: &vertex_bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                draw_clip_geometry(
                    encoder,
                    stencil_view,
                    &self.pipelines.coverage,
                    &vertex_buffer,
                    vertex_count,
                );
            }
            intersect_allowed_with_clip(encoder, stencil_view, &self.pipelines.intersect);
            clear_clip_temp(encoder, stencil_view, &self.pipelines.clear_temp);
        }
    }
}

fn create_clip_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &'static str,
    vertex_entry: &'static str,
    stencil: wgpu::DepthStencilState,
    buffers: &[Option<wgpu::VertexBufferLayout<'static>>],
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(stencil),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    })
}

const fn replace_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Replace,
    }
}

fn clip_coverage_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: replace_stencil_face(),
            back: replace_stencil_face(),
            read_mask: CLIP_TEMP,
            write_mask: CLIP_TEMP,
        },
    )
}

const fn intersection_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::NotEqual,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Zero,
    }
}

fn clip_intersection_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: intersection_stencil_face(),
            back: intersection_stencil_face(),
            read_mask: CLIP_TEMP,
            write_mask: STENCIL_ALLOWED,
        },
    )
}

const fn clear_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Zero,
    }
}

fn clip_temp_clear_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: clear_stencil_face(),
            back: clear_stencil_face(),
            read_mask: CLIP_TEMP,
            write_mask: CLIP_TEMP,
        },
    )
}

fn draw_clip_geometry(
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let stencil_attachment = stencil_attachment(stencil_view);
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui generic clip coverage pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);
    render_pass.set_stencil_reference(CLIP_TEMP);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.draw(0..vertex_count, 0..1);
}

fn intersect_allowed_with_clip(
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
) {
    let stencil_attachment = stencil_attachment(stencil_view);
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui conjunctive clip intersection pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);
    render_pass.set_stencil_reference(CLIP_TEMP);
    render_pass.draw(0..3, 0..1);
}

fn clear_clip_temp(
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
) {
    let stencil_attachment = stencil_attachment(stencil_view);
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui clip temporary-bit clear pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);
    render_pass.set_stencil_reference(0);
    render_pass.draw(0..3, 0..1);
}

const fn stencil_attachment(
    stencil_view: &wgpu::TextureView,
) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        }),
    }
}

fn clip_vertex_bytes(
    clip: &PreparedClip,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
) -> Vec<u8> {
    if clip.clip_to_surface.inverse().is_none() {
        return Vec::new();
    }
    let [m11, m12, m21, m22, tx, ty] = clip.clip_to_surface.components().map(f64::from);
    let scale = f64::from(raster_scale.get());
    let positions = clip.geometry.positions();
    let mut bytes = Vec::with_capacity(clip.geometry.indices().len().saturating_mul(8));

    for triangle in clip.geometry.indices().as_chunks::<3>().0 {
        let mut polygon = Vec::with_capacity(3);
        for index in triangle {
            let index = usize::try_from(*index)
                .unwrap_or_else(|_| unreachable!("validated clip tessellation index fits usize"));
            let [x, y] = positions[index].map(f64::from);
            polygon.push([
                m11.mul_add(x, m21.mul_add(y, tx)) * scale,
                m12.mul_add(x, m22.mul_add(y, ty)) * scale,
            ]);
        }
        let polygon = clip_polygon_to_canvas(polygon, canvas_extent);
        if polygon.len() < 3 {
            continue;
        }
        let positions = polygon
            .into_iter()
            .map(|point| physical_point_to_ndc(point, extent))
            .collect::<Vec<_>>();
        for index in 1..positions.len() - 1 {
            for position in [positions[0], positions[index], positions[index + 1]] {
                for component in position {
                    bytes.extend_from_slice(&component.to_ne_bytes());
                }
            }
        }
    }
    bytes
}
