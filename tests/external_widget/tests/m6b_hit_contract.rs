#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, ElementId, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, SemanticCommand, StyleEnvironment, UiApp, View,
    Widget, WidgetActivation, WidgetMeasure, children, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SubmitSurfaceCommandErrorKind, SurfaceBuildContext,
};

#[derive(Debug)]
struct FocusableNoHit;

impl Widget<()> for FocusableNoHit {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(10_u16),
        }
    }
}

#[derive(Debug)]
struct HitOnly;

impl Widget<()> for HitOnly {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(10_u16),
        }
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

#[test]
fn hit_regions_membership_and_focusability_are_independent_authorities() {
    let mut runtime = AppRuntime::<App>::mount(());
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
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
    assert!(
        runtime
            .index()
            .node(&focusable)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable)
    );
    assert!(
        runtime
            .index()
            .node(&hit_only)
            .is_some_and(|node| !node.is_focusable())
    );

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
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.focus().focused_node(), Some(&focusable));
}
