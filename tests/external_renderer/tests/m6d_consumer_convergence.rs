#![allow(refining_impl_trait)]

use runenui_core::{
    Color, ContributionClip, Element, HitContribution, HitContributionContext, HitRegion,
    LogicalLength, LogicalPoint, LogicalRect, LogicalSize, LogicalTransform, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, PaintPrimitive,
    PointerButton, PointerButtons, PointerDeviceKind, PointerId, PointerPhase, PointerPolicy,
    Radius, ResourceKind, ResourceRef, SceneLayer, SceneOpacity, SceneShape, StyleTokens, UiApp,
    Widget, WidgetMeasure,
};
use runenui_external_renderer_conformance::{
    ConsumerSnapshot, SceneConsumer, UpdateMode, sample_literal_paint,
};
use runenui_runtime::{
    AppRuntime, HitTestScene, LayoutConstraints, PaintPublication, PaintScene, PaintSceneItem,
    PumpBudget, RasterScale, SceneCapabilities, SurfaceBuildContext, TraceRecordKind,
};
use runenui_testing::TestHarness;

#[derive(Debug)]
struct SceneOwner {
    image: ResourceRef,
    shaped: ResourceRef,
}

impl Widget<()> for SceneOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(40_u16),
            height: LogicalLength::from(40_u16),
        }
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let half =
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("fixture opacity is valid"));
        let rounded_clip = ContributionClip::identity(SceneShape::rounded_rect(
            rect(0.0, 0.0, 20.0, 20.0),
            Radius::all(LogicalLength::from(4_u16)),
        ));
        let translated = LogicalTransform::translation(5.0, 5.0)
            .unwrap_or_else(|_| unreachable!("fixture transform is valid"));

        PaintContribution::new(vec![
            PaintContributionItem::fill_rect(
                rect(0.0, 0.0, 30.0, 30.0),
                Color::rgba(255, 0, 0, 255),
            )
            .with_layer(SceneLayer::new(-1)),
            PaintContributionItem::fill_rect(
                rect(0.0, 0.0, 10.0, 10.0),
                Color::rgba(0, 0, 255, 255),
            )
            .with_transform(translated)
            .with_clip(rounded_clip)
            .with_opacity(half),
            PaintContributionItem::stroke_rect(
                rect(2.0, 2.0, 12.0, 12.0),
                Color::rgba(0, 255, 0, 128),
                LogicalLength::from(2_u16),
            )
            .with_layer(SceneLayer::new(1)),
            PaintContributionItem::image(self.image.clone(), rect(1.0, 20.0, 8.0, 8.0))
                .unwrap_or_else(|_| unreachable!("fixture image ref has image kind"))
                .with_layer(SceneLayer::new(2)),
            PaintContributionItem::shaped_text_run(
                self.shaped.clone(),
                point(15.0, 20.0),
                Color::rgba(10, 20, 30, 255),
            )
            .unwrap_or_else(|_| unreachable!("fixture shaped ref has shaped-run kind"))
            .with_layer(SceneLayer::new(3)),
            PaintContributionItem::shaped_text_run(
                self.shaped.clone(),
                point(15.0, 20.0),
                Color::rgba(30, 20, 10, 255),
            )
            .unwrap_or_else(|_| unreachable!("fixture shaped ref has shaped-run kind"))
            .with_layer(SceneLayer::new(3)),
        ])
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let rounded = HitRegion::rounded_rect(
            rect(0.0, 0.0, 30.0, 30.0),
            Radius::all(LogicalLength::from(5_u16)),
        );
        let block = HitRegion::rect(rect(0.0, 0.0, 6.0, 6.0))
            .with_transform(
                LogicalTransform::translation(8.0, 8.0)
                    .unwrap_or_else(|_| unreachable!("fixture transform is valid")),
            )
            .with_clip(ContributionClip::identity(SceneShape::rect(rect(
                0.0, 0.0, 20.0, 20.0,
            ))))
            .with_layer(SceneLayer::new(2))
            .with_pointer_policy(PointerPolicy::Block);
        HitContribution::new(vec![rounded, block])
    }
}

#[derive(Debug)]
struct State {
    image: ResourceRef,
    shaped: ResourceRef,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(SceneOwner {
            image: state.image.clone(),
            shaped: state.shaped.clone(),
        })
        .key("scene-owner")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn state() -> State {
    State {
        image: ResourceRef::new(ResourceKind::Image),
        shaped: ResourceRef::new(ResourceKind::ShapedTextRun),
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("fixture point is finite"))
}

fn capabilities() -> SceneCapabilities {
    SceneCapabilities::new([ResourceKind::Image, ResourceKind::ShapedTextRun])
}

fn reference_sample(scene: &PaintScene, surface_point: LogicalPoint) -> [f32; 4] {
    scene
        .items()
        .iter()
        .filter_map(|item| reference_literal_source(item, surface_point))
        .fold([0.0; 4], reference_source_over)
}

fn reference_literal_source(
    item: &PaintSceneItem,
    surface_point: LogicalPoint,
) -> Option<(Color, f32)> {
    let local_point = item
        .local_to_surface()
        .inverse()
        .and_then(|surface_to_local| surface_to_local.transform_point(surface_point))?;
    if !item.clips().iter().all(|clip| {
        clip.clip_to_surface()
            .inverse()
            .and_then(|surface_to_clip| surface_to_clip.transform_point(surface_point))
            .is_some_and(|clip_point| clip.shape().contains(clip_point))
    }) {
        return None;
    }

    let color = match item.primitive() {
        PaintPrimitive::FillRect { rect, color }
            if rect.width() > 0.0 && rect.height() > 0.0 && rect.contains(local_point) =>
        {
            *color
        }
        PaintPrimitive::StrokeRect { rect, color, width }
            if reference_stroke_covers(*rect, width.get(), local_point) =>
        {
            *color
        }
        _ => return None,
    };
    Some((color, item.opacity().get()))
}

fn reference_stroke_covers(rect: LogicalRect, width: f32, point: LogicalPoint) -> bool {
    if width == 0.0 || rect.width() == 0.0 || rect.height() == 0.0 {
        return false;
    }
    let half = width / 2.0;
    let expanded = LogicalRect::try_new(
        rect.x() - half,
        rect.y() - half,
        rect.width() + width,
        rect.height() + width,
    )
    .unwrap_or_else(|_| unreachable!("accepted finite stroke expansion remains valid"));
    if !expanded.contains(point) {
        return false;
    }
    if rect.width() <= width || rect.height() <= width {
        return true;
    }
    let inset = LogicalRect::try_new(
        rect.x() + half,
        rect.y() + half,
        rect.width() - width,
        rect.height() - width,
    )
    .unwrap_or_else(|_| unreachable!("positive inset remains valid"));
    !inset.contains(point)
}

fn reference_source_over(destination: [f32; 4], (color, opacity): (Color, f32)) -> [f32; 4] {
    let alpha = (f32::from(color.alpha()) / 255.0) * opacity;
    let inverse_alpha = 1.0 - alpha;
    [
        destination[0].mul_add(inverse_alpha, reference_srgb_to_linear(color.red()) * alpha),
        destination[1].mul_add(
            inverse_alpha,
            reference_srgb_to_linear(color.green()) * alpha,
        ),
        destination[2].mul_add(
            inverse_alpha,
            reference_srgb_to_linear(color.blue()) * alpha,
        ),
        destination[3].mul_add(inverse_alpha, alpha),
    ]
}

fn reference_srgb_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn reference_target(
    scene: &HitTestScene,
    surface_point: LogicalPoint,
) -> Option<runenui_core::MountedNodeId> {
    for region in scene.regions().iter().rev() {
        let Some(local_point) = region
            .local_to_surface()
            .inverse()
            .and_then(|surface_to_local| surface_to_local.transform_point(surface_point))
        else {
            continue;
        };
        if !region.shape().contains(local_point) {
            continue;
        }
        if !region.clips().iter().all(|clip| {
            clip.clip_to_surface()
                .inverse()
                .and_then(|surface_to_clip| surface_to_clip.transform_point(surface_point))
                .is_some_and(|clip_point| clip.shape().contains(clip_point))
        }) {
            continue;
        }
        return match region.pointer_policy() {
            PointerPolicy::Target => Some(region.target().clone()),
            PointerPolicy::Block => None,
        };
    }
    None
}

fn assert_color_close(actual: [f32; 4], expected: [f32; 4]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 1.0e-6,
            "channel mismatch: {actual} != {expected}"
        );
    }
}

fn assert_snapshot_contract(
    snapshot: &ConsumerSnapshot,
    paint: &PaintPublication,
    hit: &HitTestScene,
) {
    assert_eq!(snapshot.surface_id(), paint.surface_id());
    assert_eq!(snapshot.revision(), paint.revision());
    assert_eq!(snapshot.base_revision(), None);
    assert_eq!(snapshot.logical_size(), paint.logical_size());
    assert_eq!(snapshot.raster_scale(), RasterScale::ONE);
    assert_eq!(snapshot.damage(), paint.damage());
    assert_eq!(snapshot.input_context(), hit.input_context());
    assert_eq!(
        snapshot.required_resource_kinds(),
        &[ResourceKind::Image, ResourceKind::ShapedTextRun]
    );
    assert_eq!(snapshot.paint_items().len(), paint.scene().items().len());
    assert_eq!(snapshot.hit_regions().len(), hit.regions().len());
    assert_eq!(snapshot.mounted_targets(), hit.mounted_targets());

    for (copied, public) in snapshot.paint_items().iter().zip(paint.scene().items()) {
        assert_eq!(copied.primitive(), public.primitive());
        assert_eq!(copied.local_to_surface(), public.local_to_surface());
        assert_eq!(copied.clips(), public.clips());
        assert_eq!(copied.opacity(), public.opacity());
        assert_eq!(copied.layer(), public.layer());
    }
}

fn assert_resource_contract(snapshot: &ConsumerSnapshot) {
    let image = snapshot.paint_items()[3]
        .primitive()
        .as_image()
        .unwrap_or_else(|| unreachable!("fourth canonical item is image"));
    assert_eq!(image.destination(), rect(1.0, 20.0, 8.0, 8.0));
    assert_eq!(image.resource_ref().kind(), ResourceKind::Image);

    let first_run = snapshot.paint_items()[4]
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("fifth canonical item is shaped run"));
    let second_run = snapshot.paint_items()[5]
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("sixth canonical item is shaped run"));
    assert_eq!(first_run.resource_ref(), second_run.resource_ref());
    assert_eq!(first_run.origin(), second_run.origin());
    assert_ne!(first_run.foreground(), second_run.foreground());
}

fn assert_interpreters_agree(
    snapshot: &ConsumerSnapshot,
    paint: &PaintPublication,
    hit: &HitTestScene,
) {
    for sample in [
        point(1.0, 1.0),
        point(5.5, 5.5),
        point(10.0, 10.0),
        point(14.5, 8.0),
        point(31.0, 31.0),
    ] {
        assert_color_close(
            sample_literal_paint(snapshot, sample),
            reference_sample(paint.scene(), sample),
        );
        let copied_target = snapshot.target_at(sample).cloned();
        let reference = reference_target(hit, sample);
        assert_eq!(copied_target, reference);
        assert_eq!(copied_target.as_ref(), hit.target_at(sample));
    }
}

fn assert_capability_rejection(paint: &PaintPublication, hit: &HitTestScene) {
    let mut unsupported = SceneConsumer::new(SceneCapabilities::default());
    let Err(error) = unsupported.consume(paint, hit) else {
        unreachable!("resource-backed fixture must reject empty capabilities");
    };
    assert_eq!(error.resource_kind(), ResourceKind::Image);
}

fn assert_revision_modes(
    runtime: &mut AppRuntime<App>,
    tokens: &StyleTokens,
    first_paint: &PaintPublication,
    first_hit: &HitTestScene,
    downstream: &mut SceneConsumer,
) {
    let size = LogicalSize::try_new(40.0, 40.0).unwrap_or(LogicalSize::ZERO);
    let scale_two = SurfaceBuildContext::new(tokens, LayoutConstraints::tight(size))
        .with_raster_scale(
            RasterScale::new(2.0).unwrap_or_else(|_| unreachable!("fixture scale is valid")),
        );
    let second = runtime
        .publish_surface(&scale_two)
        .unwrap_or_else(|_| unreachable!("scale-only publication is admitted"));
    assert_eq!(
        second.paint_publication().base_revision(),
        Some(first_paint.revision())
    );
    assert_eq!(
        downstream
            .consume(second.paint_publication(), second.hit_test_scene())
            .unwrap_or_else(|_| unreachable!("declared capabilities satisfy fixture"))
            .mode(),
        UpdateMode::ExactBaseMatch
    );

    downstream.reset();
    assert_eq!(
        downstream
            .consume(second.paint_publication(), second.hit_test_scene())
            .unwrap_or_else(|_| unreachable!("full resync consumes complete current scene"))
            .mode(),
        UpdateMode::FullResync
    );

    let second_revision = second.paint_publication().revision();
    let scale_three = SurfaceBuildContext::new(tokens, LayoutConstraints::tight(size))
        .with_raster_scale(
            RasterScale::new(3.0).unwrap_or_else(|_| unreachable!("fixture scale is valid")),
        );
    let third = runtime
        .publish_surface(&scale_three)
        .unwrap_or_else(|_| unreachable!("third publication is admitted"));
    assert_eq!(
        third.paint_publication().base_revision(),
        Some(second_revision)
    );

    let mut lagging = SceneConsumer::new(capabilities());
    let _ = lagging
        .consume(first_paint, first_hit)
        .unwrap_or_else(|_| unreachable!("first snapshot is supported"));
    assert_eq!(
        lagging
            .consume(third.paint_publication(), third.hit_test_scene())
            .unwrap_or_else(|_| unreachable!("skipped revision still admits full snapshot"))
            .mode(),
        UpdateMode::FullResync
    );
}

#[test]
fn independent_consumers_agree_on_public_scene_semantics_and_metadata() {
    let mut runtime = AppRuntime::<App>::mount(state());
    let tokens = StyleTokens::new();
    let size = LogicalSize::try_new(40.0, 40.0).unwrap_or(LogicalSize::ZERO);
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(size));
    let first = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("first fixture publication is admitted"));
    let first_paint = first.paint_publication().clone();
    let first_hit = first.hit_test_scene().clone();

    let mut downstream = SceneConsumer::new(capabilities());
    let first_consumption = downstream
        .consume(&first_paint, &first_hit)
        .unwrap_or_else(|_| unreachable!("declared capabilities satisfy fixture"));
    assert_eq!(first_consumption.mode(), UpdateMode::FullResync);
    let snapshot = first_consumption.snapshot();

    assert_snapshot_contract(snapshot, &first_paint, &first_hit);
    assert_resource_contract(snapshot);
    assert_interpreters_agree(snapshot, &first_paint, &first_hit);
    assert_capability_rejection(&first_paint, &first_hit);
    assert_revision_modes(
        &mut runtime,
        &tokens,
        &first_paint,
        &first_hit,
        &mut downstream,
    );
}

#[test]
fn testing_harness_exposes_the_same_ordinary_public_products_without_fabricated_context() {
    let mut harness = TestHarness::<App>::mount(state());
    assert!(
        harness
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
    let publication = harness
        .publish()
        .unwrap_or_else(|_| unreachable!("harness publication is admitted"))
        .clone();
    let exact_context = publication.input_context().clone();

    let mut consumer = SceneConsumer::new(capabilities());
    let consumption = consumer
        .consume(
            publication.paint_publication(),
            publication.hit_test_scene(),
        )
        .unwrap_or_else(|_| unreachable!("harness publication is ordinary public scene input"));
    assert_eq!(consumption.snapshot().input_context(), &exact_context);
    assert_eq!(
        consumption.snapshot().mounted_targets(),
        publication.hit_test_scene().mounted_targets()
    );

    let input_point = point(2.0, 2.0);
    let expected_target = consumption
        .snapshot()
        .target_at(input_point)
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture point resolves to the published target"));
    assert_eq!(
        publication.hit_test_scene().target_at(input_point),
        Some(&expected_target)
    );

    let pointer = harness
        .pointer_event(
            PointerId::new(1).unwrap_or_else(|| unreachable!("pointer id is non-zero")),
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            input_point,
        )
        .unwrap_or_else(|_| unreachable!("publication supplies exact public context"))
        .with_changed_button(PointerButton::Primary)
        .with_buttons(PointerButtons::new([PointerButton::Primary]));
    assert_eq!(pointer.surface_context(), &exact_context);
    let sequence = harness
        .submit_pointer(pointer)
        .unwrap_or_else(|_| unreachable!("exact-context pointer is admitted"))
        .sequence();
    assert_eq!(
        harness
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let resolved = harness
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerPhysicalTargetResolved
                )
        })
        .unwrap_or_else(|| unreachable!("public runtime traces physical target resolution"));
    assert_eq!(
        resolved
            .target()
            .map(runenui_runtime::TraceTarget::mounted_node_id),
        Some(&expected_target)
    );
}
