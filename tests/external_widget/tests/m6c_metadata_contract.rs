#![allow(refining_impl_trait)]

use runenui_core::{
    Color, Element, HitContribution, HitContributionContext, HitRegion, LogicalLength,
    LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, StyleEnvironment, UiApp, Widget,
    WidgetInvalidation, WidgetMeasure, WidgetUpdateContext,
};
use runenui_runtime::{
    AppRuntime, PaintDamage, PaintPublication, PaintRevision, PumpBudget, RasterScale,
    RasterScaleError, SurfaceBuildContext, SurfaceId,
};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height)
        .unwrap_or_else(|_| unreachable!("test surface size is finite and non-negative"))
}

fn scale_two() -> RasterScale {
    RasterScale::new(2.0).unwrap_or_else(|_| unreachable!("test raster scale is valid"))
}

#[derive(Debug)]
struct MetadataProbe {
    color: Color,
}

impl Widget<Action> for MetadataProbe {
    type State = Color;

    fn create_state(&self) -> Self::State {
        self.color
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.color {
            *state = self.color;
            context.invalidate(WidgetInvalidation::PAINT);
        }
    }

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(10_u16),
            height: LogicalLength::from(10_u16),
        }
    }

    fn paint(&self, state: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::single(PaintContributionItem::fill_rect(rect(), *state))
    }

    fn hit_test(&self, _: &Self::State, _: HitContributionContext) -> HitContribution {
        HitContribution::new(vec![HitRegion::rect(rect())])
    }
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Recolor(Color),
}

#[derive(Debug)]
struct State {
    color: Color,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(MetadataProbe { color: state.color }).key("metadata-probe")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Recolor(color) => state.color = color,
        }
    }
}

fn drain_mount(runtime: &mut AppRuntime<App>) {
    let outcome = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(outcome.is_quiescent());
}

fn publish(
    runtime: &mut AppRuntime<App>,
    context: &SurfaceBuildContext<'_>,
) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("metadata fixture publication is admitted"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConsumerPlan {
    FullSnapshot,
    Incremental,
    Reused,
}

#[derive(Default)]
struct Consumer {
    realized: Option<(SurfaceId, PaintRevision)>,
}

impl Consumer {
    fn consume(&mut self, publication: &PaintPublication) -> ConsumerPlan {
        let plan = match self.realized.as_ref() {
            Some((surface, revision))
                if surface == publication.surface_id() && *revision == publication.revision() =>
            {
                ConsumerPlan::Reused
            }
            Some((surface, revision))
                if surface == publication.surface_id()
                    && publication.base_revision() == Some(*revision) =>
            {
                ConsumerPlan::Incremental
            }
            _ => ConsumerPlan::FullSnapshot,
        };
        self.realized = Some((publication.surface_id().clone(), publication.revision()));
        plan
    }
}

#[test]
fn raster_scale_is_finite_positive_and_defaults_to_one() {
    assert_eq!(RasterScale::default(), RasterScale::ONE);
    assert_eq!(RasterScale::ONE.get().to_bits(), 1.0_f32.to_bits());
    assert_eq!(RasterScale::new(f32::NAN), Err(RasterScaleError::NotFinite));
    assert_eq!(
        RasterScale::new(f32::INFINITY),
        Err(RasterScaleError::NotFinite)
    );
    assert_eq!(
        RasterScale::new(f32::NEG_INFINITY),
        Err(RasterScaleError::NotFinite)
    );
    assert_eq!(RasterScale::new(0.0), Err(RasterScaleError::NotPositive));
    assert_eq!(RasterScale::new(-0.0), Err(RasterScaleError::NotPositive));
    assert_eq!(RasterScale::new(-1.0), Err(RasterScaleError::NotPositive));
    assert_eq!(scale_two().get().to_bits(), 2.0_f32.to_bits());
}

#[test]
fn renderer_tuple_revision_base_damage_and_logical_hit_coordinates_are_exact() {
    let initial_color = Color::rgba(10, 20, 30, 255);
    let changed_color = Color::rgba(30, 20, 10, 255);
    let style_environment = StyleEnvironment::default();
    let logical_size = size(20.0, 20.0);
    let larger_size = size(30.0, 20.0);
    let scale_two = scale_two();
    let sample = point(5.0, 5.0);

    let mut runtime = AppRuntime::<App>::mount(State {
        color: initial_color,
    });
    drain_mount(&mut runtime);

    let scale_one_context = SurfaceBuildContext::tight(&style_environment, logical_size);
    let initial = publish(&mut runtime, &scale_one_context);
    let initial_paint = initial.paint_publication();
    let target = initial
        .hit_test_scene()
        .target_at(sample)
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture point targets the root"));

    assert_eq!(initial_paint.revision().get(), 1);
    assert_eq!(initial_paint.base_revision(), None);
    assert_eq!(initial_paint.logical_size(), logical_size);
    assert_eq!(initial_paint.raster_scale(), RasterScale::ONE);
    assert_eq!(initial_paint.damage(), PaintDamage::FullSurface);

    let repeated = publish(&mut runtime, &scale_one_context);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_eq!(repeated.paint_publication(), initial_paint);
    assert_ne!(
        repeated.input_context().hit_test_generation(),
        initial.input_context().hit_test_generation()
    );
    assert_eq!(repeated.hit_test_scene().target_at(sample), Some(&target));

    let scale_two_context = scale_one_context.with_raster_scale(scale_two);
    let scaled = publish(&mut runtime, &scale_two_context);
    let scaled_paint = scaled.paint_publication();
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_eq!(scaled_paint.scene(), initial_paint.scene());
    assert_eq!(scaled_paint.logical_size(), initial_paint.logical_size());
    assert_eq!(scaled_paint.raster_scale(), scale_two);
    assert_eq!(scaled_paint.revision().get(), 2);
    assert_eq!(scaled_paint.base_revision(), Some(initial_paint.revision()));
    assert_eq!(scaled_paint.damage(), PaintDamage::FullSurface);
    assert_eq!(scaled.hit_test_scene().target_at(sample), Some(&target));

    let repeated_scaled = publish(&mut runtime, &scale_two_context);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_eq!(repeated_scaled.paint_publication(), scaled_paint);

    let larger_context =
        SurfaceBuildContext::tight(&style_environment, larger_size).with_raster_scale(scale_two);
    let resized = publish(&mut runtime, &larger_context);
    let resized_paint = resized.paint_publication();
    assert_eq!(resized_paint.scene(), scaled_paint.scene());
    assert_eq!(resized_paint.logical_size(), larger_size);
    assert_eq!(resized_paint.revision().get(), 3);
    assert_eq!(resized_paint.base_revision(), Some(scaled_paint.revision()));
    assert_eq!(resized_paint.damage(), PaintDamage::FullSurface);
    assert_eq!(resized.hit_test_scene().target_at(sample), Some(&target));

    runtime
        .submit_action(Action::Recolor(changed_color))
        .unwrap_or_else(|_| unreachable!("recolor action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let recolored = publish(&mut runtime, &larger_context);
    let recolored_paint = recolored.paint_publication();
    assert_ne!(recolored_paint.scene(), resized_paint.scene());
    assert_eq!(recolored_paint.logical_size(), resized_paint.logical_size());
    assert_eq!(recolored_paint.raster_scale(), scale_two);
    assert_eq!(recolored_paint.revision().get(), 4);
    assert_eq!(
        recolored_paint.base_revision(),
        Some(resized_paint.revision())
    );
    assert_eq!(recolored_paint.damage(), PaintDamage::FullSurface);
}

#[test]
fn consumer_uses_damage_only_for_matching_surface_and_base_revision() {
    let initial_color = Color::rgba(10, 20, 30, 255);
    let changed_color = Color::rgba(30, 20, 10, 255);
    let style_environment = StyleEnvironment::default();
    let logical_size = size(20.0, 20.0);
    let scale_one_context = SurfaceBuildContext::tight(&style_environment, logical_size);
    let scale_two_context = scale_one_context.with_raster_scale(scale_two());

    let mut runtime = AppRuntime::<App>::mount(State {
        color: initial_color,
    });
    drain_mount(&mut runtime);
    let initial = publish(&mut runtime, &scale_one_context);
    let scaled = publish(&mut runtime, &scale_two_context);
    let repeated_scaled = publish(&mut runtime, &scale_two_context);
    runtime
        .submit_action(Action::Recolor(changed_color))
        .unwrap_or_else(|_| unreachable!("recolor action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let recolored = publish(&mut runtime, &scale_two_context);

    let mut contiguous = Consumer::default();
    assert_eq!(
        contiguous.consume(initial.paint_publication()),
        ConsumerPlan::FullSnapshot
    );
    assert_eq!(
        contiguous.consume(scaled.paint_publication()),
        ConsumerPlan::Incremental
    );
    assert_eq!(
        contiguous.consume(repeated_scaled.paint_publication()),
        ConsumerPlan::Reused
    );

    let mut skipped = Consumer::default();
    assert_eq!(
        skipped.consume(initial.paint_publication()),
        ConsumerPlan::FullSnapshot
    );
    assert_eq!(
        skipped.consume(recolored.paint_publication()),
        ConsumerPlan::FullSnapshot
    );

    let mut foreign_runtime = AppRuntime::<App>::mount(State {
        color: initial_color,
    });
    drain_mount(&mut foreign_runtime);
    let _foreign_initial = publish(&mut foreign_runtime, &scale_one_context);
    let foreign_scaled = publish(&mut foreign_runtime, &scale_two_context);
    assert_eq!(
        foreign_scaled.paint_publication().base_revision(),
        Some(initial.paint_publication().revision())
    );
    assert_ne!(
        foreign_scaled.paint_publication().surface_id(),
        initial.paint_publication().surface_id()
    );

    let mut surface_sensitive = Consumer::default();
    assert_eq!(
        surface_sensitive.consume(initial.paint_publication()),
        ConsumerPlan::FullSnapshot
    );
    assert_eq!(
        surface_sensitive.consume(foreign_scaled.paint_publication()),
        ConsumerPlan::FullSnapshot
    );
}
