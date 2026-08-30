#![allow(refining_impl_trait)]

use runenui_core::{
    Element, ElementId, LogicalPoint, LogicalRect, NoHostProtocol, SemanticAction,
    StyleEnvironment, UiApp, View, button,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePhase,
};

#[derive(Clone, Copy, Debug)]
enum Action {
    Disable,
    Activated,
}

#[derive(Debug)]
struct State {
    enabled: bool,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        button("Control")
            .enabled(state.enabled)
            .on_activate(|| Action::Activated)
            .id("control")
            .key("control")
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Disable => state.enabled = false,
            Action::Activated => {}
        }
    }
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

fn publish(
    runtime: &mut AppRuntime<App>,
    style_environment: &StyleEnvironment,
) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("disabled-control publication is admitted"))
}

#[test]
fn disabled_semantic_state_does_not_implicitly_remove_physical_hit_targetability() {
    let mut runtime = AppRuntime::<App>::mount(State { enabled: true });
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let style_environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &style_environment);
    let control = initial
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored("control")))
        .unwrap_or_else(|| unreachable!("control is published"));
    let target = control.id().clone();
    let sample = center(control.bounds());
    let initial_regions = initial.hit_test_scene().regions().to_vec();
    let initial_semantic = initial
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Control"))
        .unwrap_or_else(|| unreachable!("control semantics are published"));

    assert!(!initial_semantic.state().disabled());
    assert!(
        initial_semantic
            .supported_actions()
            .contains(&SemanticAction::Activate)
    );
    assert_eq!(initial.hit_test_scene().target_at(sample), Some(&target));

    runtime
        .submit_action(Action::Disable)
        .unwrap_or_else(|_| unreachable!("disable action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );

    let disabled = publish(&mut runtime, &style_environment);
    let report = runtime.last_surface_phase_report();
    assert!(report.contains(SurfacePhase::Paint));
    assert!(report.contains(SurfacePhase::Semantics));
    assert!(!report.contains(SurfacePhase::HitTesting));
    assert_eq!(
        disabled.hit_test_scene().regions(),
        initial_regions.as_slice()
    );
    assert_eq!(disabled.hit_test_scene().target_at(sample), Some(&target));
    assert!(
        runtime
            .index()
            .node(&target)
            .is_some_and(|node| !node.is_focusable())
    );

    let disabled_semantic = disabled
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Control"))
        .unwrap_or_else(|| unreachable!("disabled control semantics are published"));
    assert!(disabled_semantic.state().disabled());
    assert!(
        disabled_semantic
            .supported_actions()
            .contains(&SemanticAction::Activate)
    );
}
