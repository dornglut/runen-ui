//! Disposable real-wgpu realization for one fill/stroke paint item.
//!
//! Lyon output is consumed only as a private coverage decomposition. Triangle
//! overlap writes one idempotent stencil coverage bit; the logical item's brush
//! and opacity are then source-over composited exactly once per covered sample.

use std::collections::HashMap;

use runenui_core::{Brush, Color, LogicalTransform, SceneOpacity, SceneShape, StrokeStyle};
use runenui_runtime::RasterScale;
use wgpu::util::DeviceExt;

use crate::tessellation::{
    TessellatedGeometry, TessellationError, tessellate_fill, tessellate_stroke,
};

use super::super::super::{
    OffscreenExtent, OffscreenRenderError, RasterCanvasExtent, clip_polygon_to_canvas,
    physical_point_to_ndc, srgb8_to_linear_f32,
};
use super::super::{
    STENCIL_ALLOWED, STENCIL_FORMAT,
    clip::{ClipRenderer, PreparedClip},
};

const COVERAGE_VERTEX_SIZE: usize = 8;
const COVERAGE_VERTEX_STRIDE: u64 = 8;
const GRADIENT_STOP_SIZE: usize = 32;
const SHADE_UNIFORM_SIZE: usize = 80;
const BRUSH_SOLID: u32 = 0;
const BRUSH_LINEAR: u32 = 1;
const BRUSH_RADIAL: u32 = 2;

const COVERAGE_SHADER: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    return output;
}

@fragment
fn fs_main() {}
";

const SHADE_SHADER: &str = r"
struct ShadeUniform {
    solid: vec4<f32>,
    geometry: vec4<f32>,
    inverse_a: vec4<f32>,
    inverse_b: vec4<f32>,
    brush_info: vec4<u32>,
}

struct GradientStop {
    offset_and_padding: vec4<f32>,
    premultiplied_linear: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> shade: ShadeUniform;

@group(0) @binding(1)
var<storage, read> stops: array<GradientStop>;

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

fn local_point(position: vec2<f32>) -> vec2<f32> {
    let surface = position / shade.inverse_b.z;
    return vec2<f32>(
        shade.inverse_a.x * surface.x + shade.inverse_a.z * surface.y + shade.inverse_b.x,
        shade.inverse_a.y * surface.x + shade.inverse_a.w * surface.y + shade.inverse_b.y,
    );
}

fn sample_gradient(coordinate_in: f32) -> vec4<f32> {
    let coordinate = clamp(coordinate_in, 0.0, 1.0);
    let count = shade.brush_info.y;
    let first = stops[0u];
    let first_offset = first.offset_and_padding.x;
    if coordinate <= first_offset {
        return first.premultiplied_linear;
    }

    var index = 1u;
    loop {
        if index >= count {
            return stops[count - 1u].premultiplied_linear;
        }
        let current = stops[index];
        let current_offset = current.offset_and_padding.x;
        if coordinate > current_offset {
            index += 1u;
            continue;
        }
        if coordinate == current_offset {
            var first_equal = index;
            loop {
                if first_equal == 0u {
                    break;
                }
                let previous = stops[first_equal - 1u];
                if previous.offset_and_padding.x != current_offset {
                    break;
                }
                first_equal -= 1u;
            }
            return stops[first_equal].premultiplied_linear;
        }

        let previous = stops[index - 1u];
        let previous_offset = previous.offset_and_padding.x;
        let progress = (coordinate - previous_offset) / (current_offset - previous_offset);
        return mix(previous.premultiplied_linear, current.premultiplied_linear, progress);
    }
    return stops[count - 1u].premultiplied_linear;
}

fn gradient_color(position: vec2<f32>) -> vec4<f32> {
    let local = local_point(position);
    var coordinate = 0.0;
    if shade.brush_info.x == 1u {
        let start = shade.geometry.xy;
        let direction = shade.geometry.zw - start;
        coordinate = dot(local - start, direction) / dot(direction, direction);
    } else {
        coordinate = distance(local, shade.geometry.xy) / shade.geometry.z;
    }
    let premultiplied = sample_gradient(coordinate);
    if premultiplied.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(
        premultiplied.rgb / premultiplied.a,
        premultiplied.a * shade.inverse_b.w,
    );
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    if shade.brush_info.x == 0u {
        return shade.solid;
    }
    return gradient_color(position.xy);
}
";

const COVERAGE_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x2,
    offset: 0,
    shader_location: 0,
}];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SupportedSolid {
    geometry: TessellatedGeometry,
    brush: Brush,
    opacity: SceneOpacity,
    local_to_surface: LogicalTransform,
    clips: Vec<PreparedClip>,
}

impl SupportedSolid {
    pub(super) fn fill(
        shape: &SceneShape,
        brush: Brush,
        opacity: SceneOpacity,
        local_to_surface: LogicalTransform,
        clips: Vec<PreparedClip>,
    ) -> Result<Self, TessellationError> {
        tessellate_fill(shape).map(|geometry| Self {
            geometry,
            brush,
            opacity,
            local_to_surface,
            clips,
        })
    }

    pub(super) fn stroke(
        shape: &SceneShape,
        style: StrokeStyle,
        brush: Brush,
        opacity: SceneOpacity,
        local_to_surface: LogicalTransform,
        clips: Vec<PreparedClip>,
    ) -> Result<Self, TessellationError> {
        tessellate_stroke(shape, style).map(|geometry| Self {
            geometry,
            brush,
            opacity,
            local_to_surface,
            clips,
        })
    }

    pub(super) const fn has_clips(&self) -> bool {
        !self.clips.is_empty()
    }

    pub(super) fn gradient_stop_buffer_size(&self) -> Option<u64> {
        let count = match &self.brush {
            Brush::Solid(_) => return None,
            Brush::Linear(gradient) => gradient.stops().as_slice().len(),
            Brush::Radial(gradient) => gradient.stops().as_slice().len(),
        };
        Some(
            u64::try_from(count)
                .unwrap_or(u64::MAX)
                .saturating_mul(GRADIENT_STOP_SIZE as u64),
        )
    }
}

#[derive(Debug)]
struct SolidTargetPipelines {
    coverage: wgpu::RenderPipeline,
    shade: wgpu::RenderPipeline,
    shade_bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Debug, Default)]
pub(super) struct SolidRenderer {
    pipelines: HashMap<wgpu::TextureFormat, SolidTargetPipelines>,
}

impl SolidRenderer {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn ensure_pipelines(
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
        self.pipelines.entry(target_format).or_insert_with(|| {
            let shade_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("runenui paint shade bind-group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
            SolidTargetPipelines {
                coverage: create_coverage_pipeline(device),
                shade: create_shade_pipeline(device, target_format, &shade_bind_group_layout),
                shade_bind_group_layout,
            }
        });
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the paint realization boundary keeps the exact target/canvas/scale, generic clip realization, and logical item input explicit"
    )]
    pub(super) fn encode_item(
        &self,
        device: &wgpu::Device,
        clip_renderer: &ClipRenderer,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        extent: OffscreenExtent,
        canvas_extent: RasterCanvasExtent,
        raster_scale: RasterScale,
        item: &SupportedSolid,
    ) {
        let vertex_bytes = coverage_vertex_bytes(item, extent, canvas_extent, raster_scale);
        if vertex_bytes.is_empty() {
            return;
        }
        let vertex_count =
            u32::try_from(vertex_bytes.len() / COVERAGE_VERTEX_SIZE).unwrap_or(u32::MAX);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui paint primitive coverage vertices"),
            contents: &vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let pipelines = self
            .pipelines
            .get(&target_format)
            .unwrap_or_else(|| unreachable!("paint target pipelines were ensured"));

        clear_coverage(encoder, stencil_view);
        draw_coverage(
            encoder,
            stencil_view,
            &pipelines.coverage,
            &vertex_buffer,
            vertex_count,
        );

        if item.has_clips() {
            clip_renderer.apply_clips(
                device,
                encoder,
                stencil_view,
                extent,
                canvas_extent,
                raster_scale,
                &item.clips,
            );
        }

        shade_once(
            device,
            encoder,
            color_view,
            stencil_view,
            pipelines,
            item,
            raster_scale,
        );
    }
}

fn create_coverage_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui paint primitive coverage shader"),
        source: wgpu::ShaderSource::Wgsl(COVERAGE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui paint primitive coverage pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui paint primitive coverage pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: COVERAGE_VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &COVERAGE_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(coverage_stencil_state()),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_shade_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui paint primitive shade shader"),
        source: wgpu::ShaderSource::Wgsl(SHADE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui paint primitive shade pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui paint primitive shade pipeline"),
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
        depth_stencil: Some(shade_stencil_state()),
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

const fn coverage_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Always,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Replace,
    }
}

fn coverage_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: coverage_stencil_face(),
            back: coverage_stencil_face(),
            read_mask: STENCIL_ALLOWED,
            write_mask: STENCIL_ALLOWED,
        },
    )
}

const fn shade_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Keep,
    }
}

fn shade_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: shade_stencil_face(),
            back: shade_stencil_face(),
            read_mask: STENCIL_ALLOWED,
            write_mask: 0,
        },
    )
}

fn clear_coverage(encoder: &mut wgpu::CommandEncoder, stencil_view: &wgpu::TextureView) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(0),
            store: wgpu::StoreOp::Store,
        }),
    };
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui paint primitive coverage reset pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn draw_coverage(
    encoder: &mut wgpu::CommandEncoder,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        }),
    };
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui paint primitive coverage pass"),
        color_attachments: &[],
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

fn shade_once(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    stencil_view: &wgpu::TextureView,
    pipelines: &SolidTargetPipelines,
    item: &SupportedSolid,
    raster_scale: RasterScale,
) {
    let shade = shade_uniform_bytes(item, raster_scale);
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui paint primitive shade uniform"),
        contents: &shade,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let stop_bytes = gradient_stop_bytes(&item.brush);
    let stop_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("runenui paint primitive gradient stops"),
        contents: &stop_bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("runenui paint primitive shade bind group"),
        layout: &pipelines.shade_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: stop_buffer.as_entire_binding(),
            },
        ],
    });
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
        label: Some("runenui paint primitive one-source shade pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(&pipelines.shade);
    render_pass.set_stencil_reference(STENCIL_ALLOWED);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}

fn shade_uniform_bytes(
    item: &SupportedSolid,
    raster_scale: RasterScale,
) -> [u8; SHADE_UNIFORM_SIZE] {
    let inverse = item
        .local_to_surface
        .inverse()
        .unwrap_or_else(|| unreachable!("covered paint item has an invertible transform"));
    let [m11, m12, m21, m22, tx, ty] = inverse.components();
    let (solid, geometry, kind, stop_count) = match &item.brush {
        Brush::Solid(color) => (solid_color(*color, item.opacity), [0.0; 4], BRUSH_SOLID, 0),
        Brush::Linear(gradient) => (
            [0.0; 4],
            [
                gradient.start().x(),
                gradient.start().y(),
                gradient.end().x(),
                gradient.end().y(),
            ],
            BRUSH_LINEAR,
            gradient.stops().as_slice().len(),
        ),
        Brush::Radial(gradient) => (
            [0.0; 4],
            [
                gradient.center().x(),
                gradient.center().y(),
                gradient.radius().get(),
                0.0,
            ],
            BRUSH_RADIAL,
            gradient.stops().as_slice().len(),
        ),
    };
    let float_values = [
        solid[0],
        solid[1],
        solid[2],
        solid[3],
        geometry[0],
        geometry[1],
        geometry[2],
        geometry[3],
        m11,
        m12,
        m21,
        m22,
        tx,
        ty,
        raster_scale.get(),
        item.opacity.get(),
    ];
    let meta = [
        kind,
        u32::try_from(stop_count)
            .unwrap_or_else(|_| unreachable!("preflighted gradient stop count fits u32")),
        0,
        0,
    ];
    let mut bytes = [0_u8; SHADE_UNIFORM_SIZE];
    for (destination, value) in bytes[..64]
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(float_values)
    {
        destination.copy_from_slice(&value.to_ne_bytes());
    }
    for (destination, value) in bytes[64..].as_chunks_mut::<4>().0.iter_mut().zip(meta) {
        destination.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn solid_color(color: Color, opacity: SceneOpacity) -> [f32; 4] {
    [
        srgb8_to_linear_f32(color.red()),
        srgb8_to_linear_f32(color.green()),
        srgb8_to_linear_f32(color.blue()),
        f32::from(color.alpha()) / 255.0 * opacity.get(),
    ]
}

fn gradient_stop_bytes(brush: &Brush) -> Vec<u8> {
    let stops = match brush {
        Brush::Solid(_) => return vec![0; GRADIENT_STOP_SIZE],
        Brush::Linear(gradient) => gradient.stops().as_slice(),
        Brush::Radial(gradient) => gradient.stops().as_slice(),
    };
    let mut bytes = Vec::with_capacity(stops.len().saturating_mul(GRADIENT_STOP_SIZE));
    for stop in stops {
        let color = stop.color();
        let alpha = f32::from(color.alpha()) / 255.0;
        let values = [
            stop.offset().get(),
            0.0,
            0.0,
            0.0,
            srgb8_to_linear_f32(color.red()) * alpha,
            srgb8_to_linear_f32(color.green()) * alpha,
            srgb8_to_linear_f32(color.blue()) * alpha,
            alpha,
        ];
        for value in values {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

fn coverage_vertex_bytes(
    item: &SupportedSolid,
    extent: OffscreenExtent,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
) -> Vec<u8> {
    if item.local_to_surface.inverse().is_none() {
        return Vec::new();
    }
    let [m11, m12, m21, m22, tx, ty] = item.local_to_surface.components().map(f64::from);
    let scale = f64::from(raster_scale.get());
    let positions = item.geometry.positions();
    let mut bytes = Vec::with_capacity(item.geometry.indices().len().saturating_mul(8));

    for triangle in item.geometry.indices().chunks_exact(3) {
        let mut polygon = Vec::with_capacity(3);
        for index in triangle {
            let index = usize::try_from(*index)
                .unwrap_or_else(|_| unreachable!("validated tessellation index fits usize"));
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

#[cfg(test)]
mod tests {
    use runenui_core::{
        Brush, Color, GradientStop, GradientStops, LinearGradient, LogicalLength, LogicalPoint,
        LogicalRect, SceneShape, StrokeStyle, UnitInterval,
    };

    use super::{gradient_stop_bytes, solid_color};

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    #[test]
    fn solid_color_multiplies_item_opacity_into_alpha_once() {
        let color = solid_color(
            Color::rgba(0x80, 0x40, 0x20, 0x80),
            runenui_core::SceneOpacity::new(0.5)
                .unwrap_or_else(|_| unreachable!("test opacity is valid")),
        );
        assert!((128.0_f32 / 255.0).mul_add(-0.5, color[3]).abs() < f32::EPSILON);
    }

    #[test]
    fn gradient_stop_upload_is_premultiplied_linear_srgb() {
        let stops = GradientStops::new(vec![
            GradientStop::new(UnitInterval::ZERO, Color::rgba(255, 0, 0, 128)),
            GradientStop::new(UnitInterval::ONE, Color::rgba(0, 0, 255, 0)),
        ])
        .unwrap_or_else(|_| unreachable!("test stops are valid"));
        let brush = Brush::Linear(
            LinearGradient::new(point(0.0, 0.0), point(1.0, 0.0), stops)
                .unwrap_or_else(|_| unreachable!("test gradient is valid")),
        );
        let bytes = gradient_stop_bytes(&brush);
        assert_eq!(bytes.len(), 64);
        let red = f32::from_ne_bytes(
            bytes[16..20]
                .try_into()
                .unwrap_or_else(|_| unreachable!("red channel occupies four bytes")),
        );
        let alpha = f32::from_ne_bytes(
            bytes[28..32]
                .try_into()
                .unwrap_or_else(|_| unreachable!("alpha channel occupies four bytes")),
        );
        assert!((red - alpha).abs() < f32::EPSILON);
        assert!((alpha - 128.0 / 255.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zero_width_stroke_prepares_empty_geometry() {
        let rect = LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
            .unwrap_or_else(|_| unreachable!("test rect is valid"));
        let item = super::SupportedSolid::stroke(
            &SceneShape::rect(rect),
            StrokeStyle::new(LogicalLength::ZERO),
            Brush::solid(Color::BLACK),
            runenui_core::SceneOpacity::OPAQUE,
            runenui_core::LogicalTransform::IDENTITY,
            Vec::new(),
        )
        .unwrap_or_else(|_| unreachable!("zero stroke realization is valid"));
        assert!(item.geometry.positions().is_empty());
    }
}
