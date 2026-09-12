//! Renderer-private realization of runtime-published atomic paint groups.
//!
//! Runtime remains the sole composition authority. This module copies only the
//! already-contracted `PaintScene` entry structure while preparing group clips,
//! ADR 0015's alpha-independent symbolic neutral support, and disposable ordinary-
//! shadow masks. Group color is rendered into a disposable full-surface intermediate
//! and composited exactly once into its parent. Shadow masks are derived before target
//! mutation and never become scene, publication, bounds, hit, or cache authority.

mod mask;
mod shadow;
mod support;

use std::{collections::HashMap, sync::Arc};

use runenui_core::{Color, SceneOpacity};
use runenui_runtime::{PaintPublication, PaintScene, PaintSceneEntry, RasterScale};
use wgpu::util::DeviceExt;

use crate::scene_subset::SceneValidationError;

use super::super::super::{OffscreenExtent, RasterCanvasExtent, texture_extent};
use super::super::{
    STENCIL_ALLOWED, clear_stencil_mask,
    clip::{self, ClipRenderer, PreparedClip},
    clipped_fill_stencil_state, image, shaped,
};
use super::{PublicationRenderError, ResourceSceneItem};

const GROUP_UNIFORM_SIZE: usize = 16;

const GROUP_SHADER: &str = r"
struct GroupUniform {
    opacity_and_padding: vec4<f32>,
}

@group(0) @binding(0)
var group_texture: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> group: GroupUniform;

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
    let source = textureLoad(group_texture, vec2<i32>(position.xy), 0);
    return source * group.opacity_and_padding.x;
}
";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreparedComposition {
    root_entries: Vec<PreparedSceneEntry>,
    has_groups: bool,
    needs_stencil: bool,
}

impl PreparedComposition {
    pub(super) const fn has_groups(&self) -> bool {
        self.has_groups
    }

    pub(super) const fn needs_stencil(&self) -> bool {
        self.needs_stencil
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PreparedSceneEntry {
    Item {
        item_index: usize,
        neutral_support: Arc<support::NeutralSupport>,
    },
    Group(PreparedGroup),
}

impl PreparedSceneEntry {
    fn neutral_support(&self) -> Arc<support::NeutralSupport> {
        match self {
            Self::Item {
                neutral_support, ..
            } => Arc::clone(neutral_support),
            Self::Group(group) => Arc::clone(&group.neutral_support),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PreparedShadow {
    mask: mask::AlphaMask,
    color: Color,
}

#[derive(Clone, Debug, PartialEq)]
struct PreparedGroup {
    entries: Vec<PreparedSceneEntry>,
    shadows: Vec<PreparedShadow>,
    clips: Vec<PreparedClip>,
    opacity: SceneOpacity,
    neutral_support: Arc<support::NeutralSupport>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "group preflight keeps the immutable runtime scene, exact raster/canvas/target facts, and bounded disposable-mask allocation policy explicit"
)]
pub(super) fn prepare(
    scene: &PaintScene,
    raster_scale: RasterScale,
    canvas_extent: RasterCanvasExtent,
    target_extent: OffscreenExtent,
    max_workspace_bytes: u64,
) -> Result<PreparedComposition, PublicationRenderError> {
    let mut has_groups = false;
    let mut needs_stencil = false;
    let mask_limits = mask::MaskLimits::new(max_workspace_bytes);
    let root_entries = prepare_entries(
        scene,
        scene.root_entries(),
        raster_scale,
        canvas_extent,
        target_extent,
        mask_limits,
        &mut has_groups,
        &mut needs_stencil,
    )?;
    let _scene_neutral_support = support::NeutralSupport::union(
        root_entries.iter().map(PreparedSceneEntry::neutral_support),
    );
    Ok(PreparedComposition {
        root_entries,
        has_groups,
        needs_stencil,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "recursive group preflight carries exact target-independent structure plus target-specific disposable mask facts without manufacturing another composition authority"
)]
fn prepare_entries(
    scene: &PaintScene,
    entries: &[PaintSceneEntry],
    raster_scale: RasterScale,
    canvas_extent: RasterCanvasExtent,
    target_extent: OffscreenExtent,
    mask_limits: mask::MaskLimits,
    has_groups: &mut bool,
    needs_stencil: &mut bool,
) -> Result<Vec<PreparedSceneEntry>, PublicationRenderError> {
    entries
        .iter()
        .copied()
        .map(|entry| {
            if let Some(item_index) = entry.item_index() {
                let item = scene
                    .items()
                    .get(item_index)
                    .unwrap_or_else(|| unreachable!("runtime composition item index resolves"));
                let neutral_support = support::NeutralSupport::from_item(item_index, item)
                    .map_err(|semantic| {
                        super::scene_failure(SceneValidationError::UnsupportedItem {
                            item_index,
                            semantic,
                        })
                    })?;
                return Ok(PreparedSceneEntry::Item {
                    item_index,
                    neutral_support,
                });
            }

            let group_id = entry
                .group_id()
                .unwrap_or_else(|| unreachable!("runtime scene entry is item or group"));
            let group = scene
                .group(group_id)
                .unwrap_or_else(|| unreachable!("runtime scene group reference resolves"));
            *has_groups = true;
            let clips = clip::prepare_clips(group.clips()).map_err(|failure| {
                PublicationRenderError::GroupClipGeometry {
                    clip_index: failure.clip_index(),
                    detail: failure.error().to_string(),
                }
            })?;
            *needs_stencil |= !clips.is_empty();
            let entries = prepare_entries(
                scene,
                group.entries(),
                raster_scale,
                canvas_extent,
                target_extent,
                mask_limits,
                has_groups,
                needs_stencil,
            )?;
            let child_support = support::NeutralSupport::union(
                entries.iter().map(PreparedSceneEntry::neutral_support),
            );
            let child_support = if group.shadows().is_empty() {
                child_support
            } else {
                support::NeutralSupport::resolve_shaped_text(scene, child_support)?
            };
            let shadow_facts = group
                .shadows()
                .iter()
                .copied()
                .map(|shadow| {
                    (
                        support::NeutralShadowFacts::new(
                            shadow.offset_x(),
                            shadow.offset_y(),
                            shadow.sigma().get(),
                            shadow.spread(),
                        ),
                        shadow,
                    )
                })
                .collect::<Vec<_>>();
            let shadows = shadow_facts
                .iter()
                .enumerate()
                .filter_map(|(shadow_index, (_facts, shadow))| {
                    match mask::prepare_visual_shadow(
                        &child_support,
                        f64::from(shadow.spread()),
                        f64::from(shadow.offset_x()),
                        f64::from(shadow.offset_y()),
                        f64::from(shadow.sigma().get()) * 3.0,
                        raster_scale,
                        canvas_extent,
                        target_extent,
                        mask_limits,
                    ) {
                        Ok(Some(mask)) => Some(Ok(PreparedShadow {
                            mask,
                            color: shadow.color(),
                        })),
                        Ok(None) => None,
                        Err(error) => Some(Err(PublicationRenderError::GroupShadowRealization {
                            shadow_index,
                            detail: error.to_string(),
                        })),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let neutral_support = support::NeutralSupport::group_from_child(
                child_support,
                shadow_facts.iter().map(|(facts, _)| *facts),
                group.clips(),
            );
            Ok(PreparedSceneEntry::Group(PreparedGroup {
                entries,
                shadows,
                clips,
                opacity: group.opacity(),
                neutral_support,
            }))
        })
        .collect()
}

#[derive(Debug)]
struct GroupTargetPipelines {
    ordinary: wgpu::RenderPipeline,
    clipped: wgpu::RenderPipeline,
}

#[derive(Debug)]
pub(super) struct GroupRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    pipelines: HashMap<wgpu::TextureFormat, GroupTargetPipelines>,
    shadows: shadow::ShadowRenderer,
}

impl GroupRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui composition-group bind-group layout"),
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
            shadows: shadow::ShadowRenderer::new(device),
        }
    }

    pub(super) fn ensure_pipelines(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), super::super::super::OffscreenRenderError> {
        if !matches!(
            target_format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(
                super::super::super::OffscreenRenderError::UnsupportedTargetFormat {
                    format: target_format,
                },
            );
        }
        if !self.pipelines.contains_key(&target_format) {
            let ordinary =
                create_group_pipeline(device, target_format, &self.bind_group_layout, false);
            let clipped =
                create_group_pipeline(device, target_format, &self.bind_group_layout, true);
            self.pipelines
                .insert(target_format, GroupTargetPipelines { ordinary, clipped });
        }
        self.shadows.ensure_pipeline(device, target_format)?;
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the group realization boundary keeps the runtime-published composition, exact target/canvas/scale, queue-backed disposable masks, item realizers, and shared stencil authority explicit"
    )]
    pub(super) fn encode_scene(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        solid_renderer: &super::solid::SolidRenderer,
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
        scene: &[ResourceSceneItem],
        composition: &PreparedComposition,
        shaped_renderer: &shaped::ShapedRunRenderer,
    ) {
        super::super::clear_color_target(encoder, color_view);
        self.encode_entries(
            device,
            queue,
            solid_renderer,
            clip_renderer,
            image_renderer,
            encoder,
            color_view,
            stencil_view,
            target_format,
            extent,
            canvas_extent,
            raster_scale,
            publication,
            scene,
            &composition.root_entries,
            shaped_renderer,
        );
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "recursive group encoding retains the same explicit renderer transaction inputs at every runtime-published structural level"
    )]
    fn encode_entries(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        solid_renderer: &super::solid::SolidRenderer,
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
        scene: &[ResourceSceneItem],
        entries: &[PreparedSceneEntry],
        shaped_renderer: &shaped::ShapedRunRenderer,
    ) {
        for entry in entries {
            match entry {
                PreparedSceneEntry::Item { item_index, .. } => {
                    let item = scene.get(*item_index).unwrap_or_else(|| {
                        unreachable!("runtime composition item index resolves in prepared scene")
                    });
                    super::encode_resource_item_to_target(
                        device,
                        solid_renderer,
                        clip_renderer,
                        image_renderer,
                        encoder,
                        color_view,
                        stencil_view,
                        target_format,
                        extent,
                        canvas_extent,
                        raster_scale,
                        publication,
                        item,
                        shaped_renderer,
                    );
                }
                PreparedSceneEntry::Group(group) => self.encode_group(
                    device,
                    queue,
                    solid_renderer,
                    clip_renderer,
                    image_renderer,
                    encoder,
                    color_view,
                    stencil_view,
                    target_format,
                    extent,
                    canvas_extent,
                    raster_scale,
                    publication,
                    scene,
                    group,
                    shaped_renderer,
                ),
            }
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "atomic group realization keeps authored shadows, child encoding, exact target facts, group clips/opacity, and parent compositing explicit"
    )]
    fn encode_group(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        solid_renderer: &super::solid::SolidRenderer,
        clip_renderer: &ClipRenderer,
        image_renderer: &image::ImageRenderer,
        encoder: &mut wgpu::CommandEncoder,
        parent_view: &wgpu::TextureView,
        stencil_view: Option<&wgpu::TextureView>,
        target_format: wgpu::TextureFormat,
        extent: OffscreenExtent,
        canvas_extent: RasterCanvasExtent,
        raster_scale: RasterScale,
        publication: &PaintPublication,
        scene: &[ResourceSceneItem],
        group: &PreparedGroup,
        shaped_renderer: &shaped::ShapedRunRenderer,
    ) {
        let intermediate = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("runenui atomic composition-group intermediate"),
            size: texture_extent(extent),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let intermediate_view = intermediate.create_view(&wgpu::TextureViewDescriptor::default());
        super::super::clear_color_target(encoder, &intermediate_view);
        for shadow in &group.shadows {
            self.shadows.encode(
                device,
                queue,
                encoder,
                &intermediate_view,
                target_format,
                &shadow.mask,
                shadow.color,
            );
        }
        self.encode_entries(
            device,
            queue,
            solid_renderer,
            clip_renderer,
            image_renderer,
            encoder,
            &intermediate_view,
            stencil_view,
            target_format,
            extent,
            canvas_extent,
            raster_scale,
            publication,
            scene,
            &group.entries,
            shaped_renderer,
        );

        let group_stencil = if group.clips.is_empty() {
            None
        } else {
            let stencil_view = stencil_view
                .unwrap_or_else(|| unreachable!("clipped composition group requires stencil"));
            clear_stencil_mask(encoder, stencil_view);
            clip_renderer.apply_clips(
                device,
                encoder,
                stencil_view,
                extent,
                canvas_extent,
                raster_scale,
                &group.clips,
            );
            Some(stencil_view)
        };
        self.composite(
            device,
            encoder,
            parent_view,
            group_stencil,
            target_format,
            &intermediate_view,
            group.opacity,
        );
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "group composite keeps source intermediate, destination target, optional published group clip mask, format, and exact opacity explicit"
    )]
    fn composite(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: Option<&wgpu::TextureView>,
        target_format: wgpu::TextureFormat,
        source_view: &wgpu::TextureView,
        opacity: SceneOpacity,
    ) {
        let mut uniform = [0_u8; GROUP_UNIFORM_SIZE];
        uniform[..4].copy_from_slice(&opacity.get().to_ne_bytes());
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui composition-group opacity uniform"),
            contents: &uniform,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("runenui composition-group bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        let pipelines = self
            .pipelines
            .get(&target_format)
            .unwrap_or_else(|| unreachable!("composition-group target pipelines were ensured"));
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
            label: Some("runenui atomic composition-group composite pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        render_pass.set_pipeline(if stencil_view.is_some() {
            &pipelines.clipped
        } else {
            &pipelines.ordinary
        });
        if stencil_view.is_some() {
            render_pass.set_stencil_reference(STENCIL_ALLOWED);
        }
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

fn create_group_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    clipped: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui composition-group shader"),
        source: wgpu::ShaderSource::Wgsl(GROUP_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui composition-group pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if clipped {
            "runenui clipped composition-group pipeline"
        } else {
            "runenui composition-group pipeline"
        }),
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
        depth_stencil: clipped.then(clipped_fill_stencil_state),
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
