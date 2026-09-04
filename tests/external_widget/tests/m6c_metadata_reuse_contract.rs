#![allow(refining_impl_trait)]

use runenui_core::{
    Color, Element, LogicalLength, LogicalRect, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, StyleEnvironment, UiApp,
    Widget, WidgetInvalidation, WidgetMeasure, WidgetUpdateContext,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, PaintRevision, PumpBudget, RasterScale,
    SurfaceBuildContext, SurfaceId, SurfacePhase,
};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

#[derive(Debug)]
struct SemanticOnlyProbe {
    name: &'static str,
}

impl Widget<Action> for SemanticOnlyProbe {
    type State = &'static str;

    fn create_state(&self) -> Self::State {
        self.name
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.name {
            *state = self.name;
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
    }

    fn measure(&self, _: &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(10_u16), LogicalLength::from(10_u16))
    }

    fn paint(&self, _: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::single(PaintContributionItem::fill_rect(rect(), Color::BLACK))
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Text).with_name(*state),
        )
    }
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Rename(&'static str),
}

#[derive(Debug)]
struct State {
    name: &'static str,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(SemanticOnlyProbe { name: state.name }).key("semantic-only")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Rename(name) => state.name = name,
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
        .unwrap_or_else(|_| unreachable!("metadata reuse fixture publication is admitted"))
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
fn semantic_only_publication_changes_semantics_without_allocating_a_paint_revision() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let mut runtime = AppRuntime::<App>::mount(State { name: "before" });
    drain_mount(&mut runtime);

    let initial = publish(&mut runtime, &context);
    let initial_paint = initial.paint_publication().clone();
    assert!(
        initial
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .any(|node| node.name() == Some("before"))
    );

    runtime
        .submit_action(Action::Rename("after"))
        .unwrap_or_else(|_| unreachable!("semantic-only action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );

    let changed = publish(&mut runtime, &context);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Semantics]
    );
    assert!(
        changed
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .any(|node| node.name() == Some("after"))
    );
    assert_eq!(changed.paint_publication(), &initial_paint);
    assert_eq!(
        changed.paint_publication().revision(),
        initial_paint.revision()
    );
}

#[test]
fn consumer_without_prior_runenui_state_reprocesses_the_complete_current_snapshot() {
    let environment = StyleEnvironment::default();
    let base_context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let scale_two =
        RasterScale::new(2.0).unwrap_or_else(|_| unreachable!("test raster scale is valid"));
    let scaled_context = base_context.with_raster_scale(scale_two);
    let mut runtime = AppRuntime::<App>::mount(State { name: "stable" });
    drain_mount(&mut runtime);

    let initial = publish(&mut runtime, &base_context);
    let scaled = publish(&mut runtime, &scaled_context);
    assert_eq!(
        scaled.paint_publication().scene(),
        initial.paint_publication().scene()
    );
    assert_eq!(
        scaled.paint_publication().base_revision(),
        Some(initial.paint_publication().revision())
    );

    let mut prior_consumer = Consumer::default();
    assert_eq!(
        prior_consumer.consume(initial.paint_publication()),
        ConsumerPlan::FullSnapshot
    );
    drop(prior_consumer);

    let mut rebuilt_consumer = Consumer::default();
    assert_eq!(
        rebuilt_consumer.consume(scaled.paint_publication()),
        ConsumerPlan::FullSnapshot
    );
    assert_eq!(scaled.paint_publication().raster_scale(), scale_two);
    assert_eq!(
        scaled.paint_publication().logical_size(),
        initial.paint_publication().logical_size()
    );
    assert_eq!(
        scaled.paint_publication().scene(),
        initial.paint_publication().scene()
    );
}
