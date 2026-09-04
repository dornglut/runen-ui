#[path = "support/reference_consumer.rs"]
mod reference_consumer;

use reference_consumer::{
    ReferenceConsumer, ReferencePaintRecord, ReferenceSnapshot, ReferenceUpdateMode,
};
use runenui_core::{
    Color, ContributionClip, Element, HitContribution, HitContributionContext, HitRegion,
    IntoEffects, LogicalLength, LogicalPoint, LogicalRect, LogicalSize, LogicalTransform,
    MountedNodeId, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, PaintPrimitive, PointerPolicy, Radius, ResourceKind, ResourceRef,
    SceneLayer, SceneOpacity, SceneShape, StyleEnvironment, UiApp, View, Widget, WidgetMeasure,
};
use runenui_external_renderer_conformance::{
    ConsumerSnapshot, SceneConsumer, UpdateMode, sample_literal_paint,
};
use runenui_runtime::{
    AppRuntime, HitTestScene, LayoutConstraints, PaintPublication, RasterScale, SceneCapabilities,
    SurfaceBuildContext,
};

#[derive(Debug)]
struct SceneOwner {
    image: ResourceRef,
    shaped: ResourceRef,
}

impl Widget<()> for SceneOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(40_u16), LogicalLength::from(40_u16))
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

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(SceneOwner {
            image: state.image.clone(),
            shaped: state.shaped.clone(),
        })
        .key("scene-owner")
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
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

fn reference_sample(snapshot: &ReferenceSnapshot, surface_point: LogicalPoint) -> [f32; 4] {
    snapshot
        .paint_items()
        .iter()
        .filter_map(|item| reference_literal_source(item, surface_point))
        .fold([0.0; 4], reference_source_over)
}

fn reference_literal_source(
    item: &ReferencePaintRecord,
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
            .is_some_and(|clip_point| reference_shape_contains(clip.shape(), clip_point))
    }) {
        return None;
    }

    let color = match item.primitive() {
        PaintPrimitive::FillRect { rect, color }
            if rect.width() > 0.0
                && rect.height() > 0.0
                && reference_rect_contains(*rect, local_point) =>
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
    if !reference_rect_contains(expanded, point) {
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
    !reference_rect_contains(inset, point)
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
    snapshot: &ReferenceSnapshot,
    surface_point: LogicalPoint,
) -> Option<MountedNodeId> {
    for region in snapshot.hit_regions().iter().rev() {
        let Some(local_point) = region
            .local_to_surface()
            .inverse()
            .and_then(|surface_to_local| surface_to_local.transform_point(surface_point))
        else {
            continue;
        };
        if !reference_shape_contains(region.shape(), local_point) {
            continue;
        }
        if !region.clips().iter().all(|clip| {
            clip.clip_to_surface()
                .inverse()
                .and_then(|surface_to_clip| surface_to_clip.transform_point(surface_point))
                .is_some_and(|clip_point| reference_shape_contains(clip.shape(), clip_point))
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

fn reference_shape_contains(shape: SceneShape, point: LogicalPoint) -> bool {
    let rect = shape.outer_rect();
    if !reference_rect_contains(rect, point) {
        return false;
    }
    let Some(radius) = shape.radius() else {
        return true;
    };

    let radii = reference_normalized_radii(rect, radius);
    let x = f64::from(point.x());
    let y = f64::from(point.y());
    let left = f64::from(rect.x());
    let top = f64::from(rect.y());
    let right = f64::from(rect.max_x());
    let bottom = f64::from(rect.max_y());

    for (center_x, center_y, active, radius) in [
        (
            left + radii[0],
            top + radii[0],
            x < left + radii[0] && y < top + radii[0],
            radii[0],
        ),
        (
            right - radii[1],
            top + radii[1],
            x >= right - radii[1] && y < top + radii[1],
            radii[1],
        ),
        (
            right - radii[2],
            bottom - radii[2],
            x >= right - radii[2] && y >= bottom - radii[2],
            radii[2],
        ),
        (
            left + radii[3],
            bottom - radii[3],
            x < left + radii[3] && y >= bottom - radii[3],
            radii[3],
        ),
    ] {
        if active && radius > 0.0 {
            let dx = x - center_x;
            let dy = y - center_y;
            if dx.mul_add(dx, dy * dy) > radius * radius {
                return false;
            }
        }
    }
    true
}

fn reference_rect_contains(rect: LogicalRect, point: LogicalPoint) -> bool {
    (rect.x()..rect.max_x()).contains(&point.x()) && (rect.y()..rect.max_y()).contains(&point.y())
}

fn reference_normalized_radii(rect: LogicalRect, radius: Radius) -> [f64; 4] {
    let authored = [
        f64::from(radius.top_left().get()),
        f64::from(radius.top_right().get()),
        f64::from(radius.bottom_right().get()),
        f64::from(radius.bottom_left().get()),
    ];
    let horizontal_top = authored[0] + authored[1];
    let horizontal_bottom = authored[3] + authored[2];
    let vertical_left = authored[0] + authored[3];
    let vertical_right = authored[1] + authored[2];
    let mut factor = 1.0_f64;
    for (extent, sum) in [
        (f64::from(rect.width()), horizontal_top),
        (f64::from(rect.width()), horizontal_bottom),
        (f64::from(rect.height()), vertical_left),
        (f64::from(rect.height()), vertical_right),
    ] {
        if sum > 0.0 {
            factor = factor.min(extent / sum);
        }
    }
    authored.map(|value| value * factor)
}

fn reference_image_surface_point(
    item: &ReferencePaintRecord,
    normalized: LogicalPoint,
) -> Option<LogicalPoint> {
    if !(0.0..=1.0).contains(&normalized.x()) || !(0.0..=1.0).contains(&normalized.y()) {
        return None;
    }
    let image = item.primitive().as_image()?;
    let destination = image.destination();
    let local = LogicalPoint::new(
        destination.width().mul_add(normalized.x(), destination.x()),
        destination
            .height()
            .mul_add(normalized.y(), destination.y()),
    )
    .ok()?;
    item.local_to_surface().transform_point(local)
}

fn reference_shaped_run_surface_origin(item: &ReferencePaintRecord) -> Option<LogicalPoint> {
    let run = item.primitive().as_shaped_text_run()?;
    item.local_to_surface().transform_point(run.origin())
}

fn assert_color_close(actual: [f32; 4], expected: [f32; 4]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 1.0e-6,
            "channel mismatch: {actual} != {expected}"
        );
    }
}

fn assert_modes_agree(actual: UpdateMode, reference: ReferenceUpdateMode) {
    assert!(
        matches!(
            (actual, reference),
            (UpdateMode::FullResync, ReferenceUpdateMode::FullResync)
                | (
                    UpdateMode::ExactBaseMatch,
                    ReferenceUpdateMode::ExactBaseMatch
                )
                | (
                    UpdateMode::AlreadyCurrent,
                    ReferenceUpdateMode::AlreadyCurrent
                )
        ),
        "consumer revision modes diverged: {actual:?} != {reference:?}"
    );
}

fn assert_snapshot_contract(
    snapshot: &ConsumerSnapshot,
    reference: &ReferenceSnapshot,
    paint: &PaintPublication,
    hit: &HitTestScene,
) {
    assert_eq!(snapshot.surface_id(), reference.surface_id());
    assert_eq!(reference.surface_id(), paint.surface_id());
    assert_eq!(snapshot.revision(), reference.revision());
    assert_eq!(reference.revision(), paint.revision());
    assert_eq!(snapshot.base_revision(), reference.base_revision());
    assert_eq!(reference.base_revision(), paint.base_revision());
    assert_eq!(snapshot.logical_size(), reference.logical_size());
    assert_eq!(reference.logical_size(), paint.logical_size());
    assert_eq!(snapshot.raster_scale(), reference.raster_scale());
    assert_eq!(reference.raster_scale(), paint.raster_scale());
    assert_eq!(snapshot.damage(), reference.damage());
    assert_eq!(reference.damage(), paint.damage());
    assert_eq!(snapshot.input_context(), reference.input_context());
    assert_eq!(reference.input_context(), hit.input_context());

    let requirements = paint.scene().requirements();
    assert_eq!(
        snapshot.required_resource_kinds(),
        reference.required_resource_kinds()
    );
    assert_eq!(
        reference.required_resource_kinds(),
        requirements.resource_kinds()
    );
    assert_eq!(snapshot.paint_items().len(), reference.paint_items().len());
    assert_eq!(reference.paint_items().len(), paint.scene().items().len());
    assert_eq!(snapshot.hit_regions().len(), reference.hit_regions().len());
    assert_eq!(reference.hit_regions().len(), hit.regions().len());
    assert_eq!(snapshot.mounted_targets(), reference.mounted_targets());
    assert_eq!(reference.mounted_targets(), hit.mounted_targets());

    for ((copied, reference), public) in snapshot
        .paint_items()
        .iter()
        .zip(reference.paint_items())
        .zip(paint.scene().items())
    {
        assert_eq!(copied.primitive(), reference.primitive());
        assert_eq!(reference.primitive(), public.primitive());
        assert_eq!(copied.local_to_surface(), reference.local_to_surface());
        assert_eq!(reference.local_to_surface(), public.local_to_surface());
        assert_eq!(copied.clips(), reference.clips());
        assert_eq!(reference.clips(), public.clips());
        assert_eq!(copied.opacity(), reference.opacity());
        assert_eq!(reference.opacity(), public.opacity());
        assert_eq!(copied.layer(), reference.layer());
        assert_eq!(reference.layer(), public.layer());
    }
    for ((copied, reference), public) in snapshot
        .hit_regions()
        .iter()
        .zip(reference.hit_regions())
        .zip(hit.regions())
    {
        assert_eq!(copied.target(), reference.target());
        assert_eq!(reference.target(), public.target());
        assert_eq!(copied.shape(), reference.shape());
        assert_eq!(reference.shape(), public.shape());
        assert_eq!(copied.local_to_surface(), reference.local_to_surface());
        assert_eq!(reference.local_to_surface(), public.local_to_surface());
        assert_eq!(copied.clips(), reference.clips());
        assert_eq!(reference.clips(), public.clips());
        assert_eq!(copied.layer(), reference.layer());
        assert_eq!(reference.layer(), public.layer());
        assert_eq!(copied.pointer_policy(), reference.pointer_policy());
        assert_eq!(reference.pointer_policy(), public.pointer_policy());
    }
}

fn assert_resource_contract(snapshot: &ConsumerSnapshot, reference: &ReferenceSnapshot) {
    assert_eq!(
        snapshot.required_resource_kinds(),
        &[ResourceKind::Image, ResourceKind::ShapedTextRun]
    );

    let image_record = &snapshot.paint_items()[3];
    let reference_image_record = &reference.paint_items()[3];
    let image = image_record
        .primitive()
        .as_image()
        .unwrap_or_else(|| unreachable!("fourth canonical item is image"));
    assert_eq!(image.destination(), rect(1.0, 20.0, 8.0, 8.0));
    assert_eq!(image.resource_ref().kind(), ResourceKind::Image);
    assert_eq!(image_record.primitive(), reference_image_record.primitive());
    for normalized in [point(0.0, 0.0), point(0.5, 0.5), point(1.0, 1.0)] {
        assert_eq!(
            image_record.image_surface_point(normalized),
            reference_image_surface_point(reference_image_record, normalized)
        );
    }
    assert_eq!(image_record.image_surface_point(point(1.01, 0.5)), None);
    assert_eq!(
        reference_image_surface_point(reference_image_record, point(1.01, 0.5)),
        None
    );

    let first_record = &snapshot.paint_items()[4];
    let second_record = &snapshot.paint_items()[5];
    let reference_first = &reference.paint_items()[4];
    let reference_second = &reference.paint_items()[5];
    let first_run = first_record
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("fifth canonical item is shaped run"));
    let second_run = second_record
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("sixth canonical item is shaped run"));
    assert_eq!(first_run.resource_ref(), second_run.resource_ref());
    assert_eq!(first_run.origin(), second_run.origin());
    assert_ne!(first_run.foreground(), second_run.foreground());
    assert_eq!(first_record.primitive(), reference_first.primitive());
    assert_eq!(second_record.primitive(), reference_second.primitive());
    assert_eq!(
        first_record.shaped_run_surface_origin(),
        reference_shaped_run_surface_origin(reference_first)
    );
    assert_eq!(
        second_record.shaped_run_surface_origin(),
        reference_shaped_run_surface_origin(reference_second)
    );
}

fn assert_interpreters_agree(
    snapshot: &ConsumerSnapshot,
    reference: &ReferenceSnapshot,
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
            reference_sample(reference, sample),
        );
        let copied_target = snapshot.target_at(sample).cloned();
        let reference_target = reference_target(reference, sample);
        assert_eq!(copied_target, reference_target);
        assert_eq!(copied_target.as_ref(), hit.target_at(sample));
    }
}

fn assert_capability_rejection(paint: &PaintPublication, hit: &HitTestScene) {
    for (capabilities, expected_missing) in [
        (SceneCapabilities::default(), ResourceKind::Image),
        (
            SceneCapabilities::new([ResourceKind::Image]),
            ResourceKind::ShapedTextRun,
        ),
    ] {
        let mut downstream = SceneConsumer::new(capabilities.clone());
        let Err(error) = downstream.consume(paint, hit) else {
            unreachable!("resource-backed fixture must reject incomplete capabilities");
        };
        assert_eq!(error.resource_kind(), expected_missing);

        let mut reference = ReferenceConsumer::new(capabilities);
        let Err(reference_error) = reference.consume(paint, hit) else {
            unreachable!("reference consumer must reject incomplete capabilities");
        };
        assert_eq!(reference_error, expected_missing);
    }
}

fn assert_state_loss_resync(
    downstream: &mut SceneConsumer,
    reference: &mut ReferenceConsumer,
    paint: &PaintPublication,
    hit: &HitTestScene,
    expected_downstream: &ConsumerSnapshot,
    expected_reference: &ReferenceSnapshot,
) {
    downstream.reset();
    reference.reset();
    let state_loss = downstream
        .consume(paint, hit)
        .unwrap_or_else(|_| unreachable!("full resync consumes complete current scene"));
    let reference_state_loss = reference
        .consume(paint, hit)
        .unwrap_or_else(|_| unreachable!("reference full resync consumes current scene"));
    assert_modes_agree(state_loss.mode(), reference_state_loss.mode());
    assert_eq!(state_loss.snapshot(), expected_downstream);
    assert_eq!(reference_state_loss.snapshot(), expected_reference);
    assert_snapshot_contract(
        state_loss.snapshot(),
        reference_state_loss.snapshot(),
        paint,
        hit,
    );
}

fn assert_already_current(
    downstream: &mut SceneConsumer,
    reference: &mut ReferenceConsumer,
    paint: &PaintPublication,
    hit: &HitTestScene,
) {
    let same = downstream
        .consume(paint, hit)
        .unwrap_or_else(|_| unreachable!("identical supported publication remains consumable"));
    let reference_same = reference
        .consume(paint, hit)
        .unwrap_or_else(|_| unreachable!("reference consumer supports identical publication"));
    assert_modes_agree(same.mode(), reference_same.mode());
    assert_snapshot_contract(same.snapshot(), reference_same.snapshot(), paint, hit);
}

fn assert_revision_modes(
    runtime: &mut AppRuntime<App>,
    style_environment: &StyleEnvironment,
    first_paint: &PaintPublication,
    first_hit: &HitTestScene,
    downstream: &mut SceneConsumer,
    reference: &mut ReferenceConsumer,
) {
    assert_already_current(downstream, reference, first_paint, first_hit);

    let size = LogicalSize::try_new(40.0, 40.0).unwrap_or(LogicalSize::ZERO);
    let scale_two = SurfaceBuildContext::new(style_environment, LayoutConstraints::tight(size))
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
    let contiguous = downstream
        .consume(second.paint_publication(), second.hit_test_scene())
        .unwrap_or_else(|_| unreachable!("declared capabilities satisfy fixture"));
    let reference_contiguous = reference
        .consume(second.paint_publication(), second.hit_test_scene())
        .unwrap_or_else(|_| unreachable!("reference capabilities satisfy fixture"));
    assert_modes_agree(contiguous.mode(), reference_contiguous.mode());
    assert_snapshot_contract(
        contiguous.snapshot(),
        reference_contiguous.snapshot(),
        second.paint_publication(),
        second.hit_test_scene(),
    );
    let contiguous_snapshot = contiguous.snapshot().clone();
    let reference_contiguous_snapshot = reference_contiguous.snapshot().clone();
    assert_state_loss_resync(
        downstream,
        reference,
        second.paint_publication(),
        second.hit_test_scene(),
        &contiguous_snapshot,
        &reference_contiguous_snapshot,
    );

    let second_revision = second.paint_publication().revision();
    let scale_three = SurfaceBuildContext::new(style_environment, LayoutConstraints::tight(size))
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
    let mut reference_lagging = ReferenceConsumer::new(capabilities());
    let _ = lagging
        .consume(first_paint, first_hit)
        .unwrap_or_else(|_| unreachable!("first snapshot is supported"));
    let _ = reference_lagging
        .consume(first_paint, first_hit)
        .unwrap_or_else(|_| unreachable!("reference first snapshot is supported"));
    let skipped = lagging
        .consume(third.paint_publication(), third.hit_test_scene())
        .unwrap_or_else(|_| unreachable!("skipped revision still admits full snapshot"));
    let reference_skipped = reference_lagging
        .consume(third.paint_publication(), third.hit_test_scene())
        .unwrap_or_else(|_| unreachable!("reference skipped revision admits full snapshot"));
    assert_modes_agree(skipped.mode(), reference_skipped.mode());
    assert_snapshot_contract(
        skipped.snapshot(),
        reference_skipped.snapshot(),
        third.paint_publication(),
        third.hit_test_scene(),
    );

    let mut fresh = SceneConsumer::new(capabilities());
    let mut reference_fresh = ReferenceConsumer::new(capabilities());
    let fresh_third = fresh
        .consume(third.paint_publication(), third.hit_test_scene())
        .unwrap_or_else(|_| unreachable!("fresh consumer admits complete current snapshot"));
    let reference_fresh_third = reference_fresh
        .consume(third.paint_publication(), third.hit_test_scene())
        .unwrap_or_else(|_| unreachable!("fresh reference consumer admits current snapshot"));
    assert_modes_agree(fresh_third.mode(), reference_fresh_third.mode());
    assert_eq!(skipped.snapshot(), fresh_third.snapshot());
    assert_eq!(
        reference_skipped.snapshot(),
        reference_fresh_third.snapshot()
    );
    assert_snapshot_contract(
        fresh_third.snapshot(),
        reference_fresh_third.snapshot(),
        third.paint_publication(),
        third.hit_test_scene(),
    );
}

#[test]
fn independent_consumers_agree_on_public_scene_semantics_and_metadata() {
    let mut runtime = AppRuntime::<App>::mount(state());
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(40.0, 40.0).unwrap_or(LogicalSize::ZERO);
    let context = SurfaceBuildContext::new(&style_environment, LayoutConstraints::tight(size));
    let first = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("first fixture publication is admitted"));
    let first_paint = first.paint_publication().clone();
    let first_hit = first.hit_test_scene().clone();

    let mut downstream = SceneConsumer::new(capabilities());
    let mut reference = ReferenceConsumer::new(capabilities());
    let first_consumption = downstream
        .consume(&first_paint, &first_hit)
        .unwrap_or_else(|_| unreachable!("declared capabilities satisfy fixture"));
    let reference_first = reference
        .consume(&first_paint, &first_hit)
        .unwrap_or_else(|_| unreachable!("reference capabilities satisfy fixture"));
    assert_modes_agree(first_consumption.mode(), reference_first.mode());

    let snapshot = first_consumption.snapshot();
    let reference_snapshot = reference_first.snapshot();
    assert_snapshot_contract(snapshot, reference_snapshot, &first_paint, &first_hit);
    assert_resource_contract(snapshot, reference_snapshot);
    assert_interpreters_agree(snapshot, reference_snapshot, &first_hit);
    assert_capability_rejection(&first_paint, &first_hit);
    assert_revision_modes(
        &mut runtime,
        &style_environment,
        &first_paint,
        &first_hit,
        &mut downstream,
        &mut reference,
    );
}
