#![allow(refining_impl_trait)]

use runenui_core::{
    Element, NoHostProtocol, SemanticAction, SemanticActionRequest, SemanticContribution,
    SemanticContributionContext, SemanticKey, SemanticNodeContribution, SemanticRole,
    SemanticState, StyleEnvironment, UiApp, Widget, WidgetActivation,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SubmitSemanticActionError,
    SubmitSemanticActionErrorKind, SurfaceBuildContext,
};

#[derive(Clone, Copy, Debug)]
enum Case {
    FocusOwnerDisabled,
    MenuNodeDisabled,
    MenuNodeInert,
}

#[derive(Debug)]
struct ProbeWidget {
    case: Case,
}

impl Widget<()> for ProbeWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        match self.case {
            Case::FocusOwnerDisabled => WidgetActivation::disabled(),
            Case::MenuNodeDisabled | Case::MenuNodeInert => WidgetActivation::NONE,
        }
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        match self.case {
            Case::FocusOwnerDisabled => SemanticContribution::single(
                SemanticNodeContribution::primary(SemanticRole::Button)
                    .with_name("primary")
                    .with_action(SemanticAction::RequestFocus),
            ),
            Case::MenuNodeDisabled | Case::MenuNodeInert => {
                let named = SemanticKey::from_static("named")
                    .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
                let state = match self.case {
                    Case::MenuNodeDisabled => SemanticState::ENABLED.with_disabled(true),
                    Case::MenuNodeInert => SemanticState::ENABLED.with_inert(true),
                    Case::FocusOwnerDisabled => unreachable!("focus case handled above"),
                };
                SemanticContribution::single(
                    SemanticNodeContribution::primary(SemanticRole::Group).with_child(
                        SemanticNodeContribution::new(named, SemanticRole::Button)
                            .with_name("named")
                            .with_state(state)
                            .with_action(SemanticAction::OpenMenu)
                            .with_action(SemanticAction::OpenContextMenu),
                    ),
                )
            }
        }
    }
}

#[derive(Debug)]
struct State {
    case: Case,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let element = Element::new(ProbeWidget { case: state.case })
            .id("probe")
            .key("probe");
        match state.case {
            Case::FocusOwnerDisabled => element.focusable(true),
            Case::MenuNodeDisabled | Case::MenuNodeInert => element,
        }
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn runtime(case: Case) -> AppRuntime<App> {
    AppRuntime::<App>::mount(State { case })
}

fn expect_rejection(
    result: Result<runenui_runtime::CommandSubmission, SubmitSemanticActionError>,
) -> SubmitSemanticActionError {
    let Err(error) = result else {
        unreachable!("semantic action was expected to reject")
    };
    error
}

#[test]
fn explicitly_focusable_disabled_owner_retains_focus_support_but_is_unavailable() {
    let mut runtime = runtime(Case::FocusOwnerDisabled);
    runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("semantic publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();
    let target = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some("primary"))
        .unwrap_or_else(|| unreachable!("primary semantic node is published"));
    assert!(
        target
            .supported_actions()
            .contains(&SemanticAction::RequestFocus)
    );
    assert!(target.state().disabled());

    let request = SemanticActionRequest::new(
        snapshot.surface_id().clone(),
        target.id().clone(),
        SemanticAction::RequestFocus,
    );
    let expected = request.clone();
    let error = expect_rejection(runtime.submit_semantic_action(request));
    assert_eq!(
        error.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );
    assert_eq!(error.into_request(), expected);
    assert_eq!(runtime.focus().focused_node(), None);
}

#[test]
fn menu_actions_retain_support_but_reject_disabled_and_inert_named_nodes() {
    for case in [Case::MenuNodeDisabled, Case::MenuNodeInert] {
        let mut runtime = runtime(case);
        runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
        let style_environment = StyleEnvironment::default();
        let publication = runtime
            .publish_surface(&SurfaceBuildContext::new(
                &style_environment,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("semantic publication is admitted"));
        let snapshot = publication.semantic_publication().snapshot();
        let target = snapshot
            .nodes()
            .iter()
            .find(|node| node.name() == Some("named"))
            .unwrap_or_else(|| unreachable!("named semantic node is published"));
        assert!(
            target.state().disabled() || target.state().inert(),
            "test case must compose an unavailable named node"
        );

        for action in [SemanticAction::OpenMenu, SemanticAction::OpenContextMenu] {
            assert!(target.supported_actions().contains(&action));
            let request = SemanticActionRequest::new(
                snapshot.surface_id().clone(),
                target.id().clone(),
                action,
            );
            let expected = request.clone();
            let error = expect_rejection(runtime.submit_semantic_action(request));
            assert_eq!(
                error.kind(),
                SubmitSemanticActionErrorKind::UnavailableAction
            );
            assert_eq!(error.into_request(), expected);
        }
    }
}
