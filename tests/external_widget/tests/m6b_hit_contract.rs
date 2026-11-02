#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, ElementId, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, SemanticCommand, StyleEnvironment, UiApp, View,
    Widget, WidgetActivation, WidgetMeasure, children, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, SubmitSurfaceCommandErrorKind,
    SurfaceBuildContext,
};

#[derive(Debug)]
struct FocusableNoHit;

impl Widget<()> for FocusableNoHit {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(10_u16))
    }
}

#[derive(Debug)]
struct HitOnly;

impl Widget<()> for HitOnly {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(10_u16))
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rectangle"));
        HitContribution::single_rect(rect)
    }
}

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 0)
            .with_reconciliation(3, 0, 3)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            Element::new(FocusableNoHit).id("focusable-no-hit"),
            Element::new(HitOnly).id("hit-only"),
        ])
        .key("root")
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn authored(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("test authored id is valid"))
}

fn center(rect: LogicalRect) -> LogicalPoint {
    LogicalPoint::new(
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published logical bounds are finite"))
}

fn assert_focusability(
    runtime: &mut AppRuntime<App>,
    focusable: &MountedNodeId,
    hit_only: &MountedNodeId,
) {
    assert!(
        runtime
            .index()
            .node(focusable)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable)
    );
    assert!(
        runtime
            .index()
            .node(hit_only)
            .is_some_and(|node| !node.is_focusable())
    );
}

#[test]
fn hit_regions_membership_and_focusability_are_independent_authorities() {
    let mut runtime = AppRuntime::<App>::mount(())
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let focusable_id = authored("focusable-no-hit");
    let hit_only_id = authored("hit-only");
    let focusable = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&focusable_id))
        .unwrap_or_else(|| unreachable!("focusable probe is mounted"))
        .id()
        .clone();
    let hit_only = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&hit_only_id))
        .unwrap_or_else(|| unreachable!("hit-only probe is mounted"))
        .id()
        .clone();
    assert_focusability(&mut runtime, &focusable, &hit_only);

    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("hit authority publication is admitted"));
    let focusable_point = center(
        publication
            .frame()
            .node(&focusable)
            .unwrap_or_else(|| unreachable!("focusable probe is laid out"))
            .bounds(),
    );
    let hit_only_point = center(
        publication
            .frame()
            .node(&hit_only)
            .unwrap_or_else(|| unreachable!("hit-only probe is laid out"))
            .bounds(),
    );
    let scene = publication.hit_test_scene();

    assert!(scene.contains_mounted_target(&focusable));
    assert!(scene.contains_mounted_target(&hit_only));
    assert_eq!(scene.target_at(focusable_point), None);
    assert_eq!(scene.target_at(hit_only_point), Some(&hit_only));
    assert_eq!(scene.regions().len(), 1);

    let context = publication.input_context().clone();
    let Err(error) = runtime.submit_surface_command(
        context.clone(),
        focusable_point,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("point ingress must not infer targetability from membership")
    };
    assert_eq!(error.kind(), SubmitSurfaceCommandErrorKind::NoTarget);

    runtime
        .submit_resolved_surface_command(
            context,
            focusable.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| {
            unreachable!(
                "resolved ingress validates exact snapshot membership without a hit region"
            )
        });
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.focus().focused_node(), Some(&focusable));
}
