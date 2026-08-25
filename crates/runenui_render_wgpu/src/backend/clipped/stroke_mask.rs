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
        PaintContributionItem, SceneShape, StyleTokens, UiApp, Widget, WidgetInvalidation,
        WidgetMeasure, WidgetUpdateContext,
    };
    use runenui_runtime::{AppRuntime, LayoutConstraints, PaintPublication, SurfaceBuildContext};

    use super::super::{Renderer, validate_clipped_scene_subset};
    use super::StrokeMaskUniform;
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

    fn length(value: f32) -> LogicalLength {
        LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is valid"))
    }

    fn publication(items: Vec<PaintContributionItem>, scale: f32) -> PaintPublication {
        let mut runtime = AppRuntime::<FixtureApp>::mount(items);
        let tokens = StyleTokens::new();
        let logical_size =
            LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
                .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
        let raster_scale = runenui_runtime::RasterScale::new(scale)
            .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
            .with_raster_scale(raster_scale);
        runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
            .paint_publication()
            .clone()
    }

    #[test]
    fn centered_stroke_decomposition_preserves_checked_m6_geometry() -> Result<(), Box<dyn Error>> {
        let normal = publication(
            vec![PaintContributionItem::stroke_rect(
                rect(10.0, 10.0, 20.0, 12.0),
                Color::WHITE,
                length(4.0),
            )],
            1.0,
        );
        let normal_literals = validate_clipped_scene_subset(&normal)?;
        assert_eq!(normal_literals.len(), 1);
        assert_eq!(
            normal_literals[0].literal.fill.rect,
            rect(8.0, 8.0, 24.0, 16.0)
        );
        assert_eq!(
            normal_literals[0].literal.stroke_inset,
            Some(rect(12.0, 12.0, 16.0, 8.0))
        );

        let collapsed = publication(
            vec![PaintContributionItem::stroke_rect(
                rect(10.0, 10.0, 10.0, 6.0),
                Color::WHITE,
                length(8.0),
            )],
            1.0,
        );
        let collapsed_literals = validate_clipped_scene_subset(&collapsed)?;
        assert_eq!(collapsed_literals.len(), 1);
        assert_eq!(
            collapsed_literals[0].literal.fill.rect,
            rect(6.0, 6.0, 18.0, 14.0)
        );
        assert_eq!(collapsed_literals[0].literal.stroke_inset, None);

        for noncovering in [
            PaintContributionItem::stroke_rect(
                rect(10.0, 10.0, 20.0, 12.0),
                Color::WHITE,
                LogicalLength::ZERO,
            ),
            PaintContributionItem::stroke_rect(
                rect(10.0, 10.0, 0.0, 12.0),
                Color::WHITE,
                length(4.0),
            ),
            PaintContributionItem::stroke_rect(
                rect(0.0, 0.0, f32::MAX, 1.0),
                Color::WHITE,
                length(f32::MAX),
            ),
        ] {
            let publication = publication(vec![noncovering], 1.0);
            assert!(validate_clipped_scene_subset(&publication)?.is_empty());
        }
        Ok(())
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "the prepared stroke mask must retain canonical f32 inverse, scale, and accepted inset values exactly before GPU upload"
    )]
    fn stroke_mask_uniform_uses_canonical_inverse_scale_and_inset() -> Result<(), Box<dyn Error>> {
        let transform = LogicalTransform::try_new(1.0, 0.2, -0.15, 1.0, 6.0, 4.0)?;
        let affine_publication = publication(
            vec![
                PaintContributionItem::stroke_rect(
                    rect(10.0, 10.0, 20.0, 12.0),
                    Color::WHITE,
                    length(4.0),
                )
                .with_transform(transform),
            ],
            1.3,
        );
        let literals = validate_clipped_scene_subset(&affine_publication)?;
        let literal = &literals[0].literal;
        let uniform = StrokeMaskUniform::from_literal(literal, affine_publication.raster_scale())
            .unwrap_or_else(|| {
                unreachable!("fixture stroke has an inset and invertible transform")
            });
        let inverse = literal
            .fill
            .local_to_surface
            .inverse()
            .unwrap_or_else(|| unreachable!("fixture transform is invertible"));
        assert_eq!(&uniform.values[..6], &inverse.components());
        assert_eq!(uniform.values[6], 1.3);
        assert_eq!(&uniform.values[8..], &[12.0, 12.0, 28.0, 20.0]);

        let singular = LogicalTransform::try_new(1.0, 0.0, 0.0, 0.0, 2.0, 1.0)?;
        let singular_publication = publication(
            vec![
                PaintContributionItem::stroke_rect(
                    rect(10.0, 10.0, 20.0, 12.0),
                    Color::WHITE,
                    length(4.0),
                )
                .with_transform(singular),
            ],
            1.3,
        );
        let singular_literals = validate_clipped_scene_subset(&singular_publication)?;
        assert!(
            StrokeMaskUniform::from_literal(
                &singular_literals[0].literal,
                singular_publication.raster_scale(),
            )
            .is_none()
        );
        Ok(())
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct StrokeProofProbes {
        visible_ring: Option<(u32, u32)>,
        transformed_ring: Option<(u32, u32)>,
        inset_hole: Option<(u32, u32)>,
        clip_cut: Option<(u32, u32)>,
        collapsed_center: Option<(u32, u32)>,
    }

    fn find_stroke_proof_probes(
        publication: &PaintPublication,
    ) -> Result<StrokeProofProbes, Box<dyn Error>> {
        let literals = validate_clipped_scene_subset(publication)?;
        let stroke = &literals[1].literal;
        let collapsed = &literals[2].literal;
        let stroke_item = &publication.scene().items()[1];
        let surface_to_local = stroke
            .fill
            .local_to_surface
            .inverse()
            .unwrap_or_else(|| unreachable!("fixture stroke transform is invertible"));
        let scale = publication.raster_scale().get();
        let (_, extent) = crate::backend::publication_extents(publication)?;
        let physical_width = u16::try_from(extent.width())?;
        let physical_height = u16::try_from(extent.height())?;
        let collapsed_source = rect(44.0, 10.0, 8.0, 6.0);
        let mut probes = StrokeProofProbes::default();

        for y in 0..physical_height {
            for x in 0..physical_width {
                let surface =
                    LogicalPoint::new((f32::from(x) + 0.5) / scale, (f32::from(y) + 0.5) / scale)?;
                let Some(local) = surface_to_local.transform_point(surface) else {
                    continue;
                };
                let in_expanded = stroke.fill.rect.contains(local);
                let in_inset = stroke
                    .stroke_inset
                    .is_some_and(|inset| inset.contains(local));
                let ring = in_expanded && !in_inset;
                let clips = stroke_item
                    .clips()
                    .iter()
                    .all(|clip| clip.contains_surface_point(surface));
                let probe = (u32::from(x), u32::from(y));

                if ring && clips {
                    probes.visible_ring.get_or_insert(probe);
                    if !stroke.fill.rect.contains(surface) {
                        probes.transformed_ring.get_or_insert(probe);
                    }
                }
                if in_expanded && in_inset && clips {
                    probes.inset_hole.get_or_insert(probe);
                }
                if ring && !clips {
                    probes.clip_cut.get_or_insert(probe);
                }
                if collapsed_source.contains(surface) && collapsed.fill.rect.contains(surface) {
                    probes.collapsed_center.get_or_insert(probe);
                }
            }
        }
        Ok(probes)
    }

    fn pixel(readback: &crate::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
        let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
        readback.rgba8_srgb()[index..index + 4]
            .try_into()
            .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
    }

    #[test]
    fn real_gpu_centered_stroke_transform_clip_and_collapsed_inset_match_contract()
    -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let background = Color::rgb(0x25, 0x4D, 0x78);
        let stroke_color = Color::rgb(0xC3, 0x4A, 0x42);
        let collapsed_color = Color::rgb(0x47, 0x9A, 0x63);
        let zero_width_color = Color::rgb(0x3A, 0x6D, 0xD8);
        let stroke_transform = LogicalTransform::try_new(1.0, 0.0, 0.0, 1.0, 6.0, 4.0)?;
        let stroke_clip =
            ContributionClip::identity(SceneShape::rect(rect(16.0, 12.0, 20.0, 16.0)));
        let collapsed_rect = rect(44.0, 10.0, 8.0, 6.0);
        let publication = publication(
            vec![
                PaintContributionItem::fill_rect(rect(0.0, 0.0, 64.0, 48.0), background),
                PaintContributionItem::stroke_rect(
                    rect(10.0, 8.0, 20.0, 16.0),
                    stroke_color,
                    length(4.0),
                )
                .with_transform(stroke_transform)
                .with_clip(stroke_clip),
                PaintContributionItem::stroke_rect(collapsed_rect, collapsed_color, length(8.0)),
                PaintContributionItem::stroke_rect(
                    collapsed_rect,
                    zero_width_color,
                    LogicalLength::ZERO,
                ),
            ],
            1.3,
        );
        let output = renderer.render_offscreen_publication(&publication)?;
        assert_eq!(
            output.readback().extent(),
            crate::OffscreenExtent::new(84, 63)?
        );
        let probes = find_stroke_proof_probes(&publication)?;

        for (label, probe) in [
            ("visible centered ring", probes.visible_ring),
            ("transformed ring", probes.transformed_ring),
        ] {
            let probe = probe.unwrap_or_else(|| unreachable!("fixture contains {label} probe"));
            assert_eq!(
                pixel(output.readback(), probe.0, probe.1),
                [0xC3, 0x4A, 0x42, 0xFF],
                "{label} must retain stroke color"
            );
        }
        for (label, probe) in [
            ("accepted inset", probes.inset_hole),
            ("authored clip exclusion", probes.clip_cut),
        ] {
            let probe = probe.unwrap_or_else(|| unreachable!("fixture contains {label} probe"));
            assert_eq!(
                pixel(output.readback(), probe.0, probe.1),
                [0x25, 0x4D, 0x78, 0xFF],
                "{label} must expose the prior background"
            );
        }
        let collapsed = probes
            .collapsed_center
            .unwrap_or_else(|| unreachable!("fixture contains collapsed-inset center probe"));
        assert_eq!(
            pixel(output.readback(), collapsed.0, collapsed.1),
            [0x47, 0x9A, 0x63, 0xFF],
            "collapsed inset must cover its center and later zero-width stroke must not overwrite it"
        );
        eprintln!(
            "REAL GPU STROKE PROOF: centered expanded-minus-inset coverage, transform, explicit clip, fractional scale, collapsed inset, and zero-width noncoverage match the accepted M6 contract; adapter={:?} backend={}",
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
                    "native wgpu stroke proof unavailable under {requested:?}; structured adapter failure: {detail}"
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
