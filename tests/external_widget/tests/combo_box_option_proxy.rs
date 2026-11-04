#![allow(refining_impl_trait)]

//! The accepted `OptionItem` is also a non-focusable `ComboBox` proxy Option:
//! source focus remains M4; target identity is resolved by existing M5 semantics.
use runenui_core::{
    ChildBearingWidget, CommandOrigin, ElementId, Focusability, NoHostProtocol, SemanticAction,
    SemanticCommand, SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticPopupKind, SemanticReference, SemanticRelationship, SemanticRelationshipKind,
    SemanticRole, SemanticState, UiApp, View, Widget, container, list_box, option_item,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Choose(u8),
}
struct App;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    committed: u8,
}

#[derive(Debug)]
struct DownstreamComboOwner;
impl Widget<Action> for DownstreamComboOwner {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Publication => {
                runenui_core::WidgetProducerBudget::new(0).with_publication_reveals(1)
            }
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
    fn semantics(
        &self,
        &(): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_name("Choose item")
            .with_popup(SemanticPopupKind::ListBox)
            .with_state(SemanticState::ENABLED.with_expanded(true))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Authored {
                    element_id: ElementId::new("combo.proxy.popup")
                        .unwrap_or_else(|_| unreachable!("valid popup ID")),
                    semantic_key: None,
                },
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Authored {
                    element_id: ElementId::new("combo.proxy.first")
                        .unwrap_or_else(|_| unreachable!("valid option ID")),
                    semantic_key: None,
                },
            ));
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl ChildBearingWidget<Action> for DownstreamComboOwner {}
impl App {
    const fn producer_budget(nodes: usize) -> runenui_core::ProducerBudget {
        let outputs = 6 * nodes + 2;
        runenui_core::ProducerBudget::new(outputs + nodes + 1, outputs)
            .with_application_outputs(0)
            .with_reconciliation(nodes, nodes, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }
}

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 0)
            .with_application_outputs(0)
            .with_reconciliation(4, 0, 4)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        Self::producer_budget(4)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget(4)
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        let popup = list_box([
            option_item("First", state.committed == 1)
                .id("combo.proxy.first")
                .combo_box_proxy()
                .on_activate(|| Action::Choose(1)),
            option_item("Second", state.committed == 2)
                .id("combo.proxy.second")
                .combo_box_proxy()
                .on_activate(|| Action::Choose(2)),
        ])
        .id("combo.proxy.popup");
        container(DownstreamComboOwner, vec![popup.into_element()])
            .id("combo.proxy.owner")
            .into_element()
            .with_focusability(Focusability::Focusable)
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        let Action::Choose(value) = action;
        state.committed = value;
    }
}
#[test]
fn popup_options_keep_standard_semantics_and_activation_without_stealing_real_focus() {
    let mut runtime = AppRuntime::<App>::mount(State { committed: 1 })
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
    let lookup = |s: &str| ElementId::new(s).unwrap_or_else(|_| unreachable!("valid authored ID"));
    let owner = runtime
        .index()
        .nodes()
        .iter()
        .find(|n| n.authored_id() == Some(&lookup("combo.proxy.owner")))
        .unwrap_or_else(|| unreachable!("mounted ComboBox"))
        .id()
        .clone();
    let option = runtime
        .index()
        .nodes()
        .iter()
        .find(|n| n.authored_id() == Some(&lookup("combo.proxy.first")))
        .unwrap_or_else(|| unreachable!("mounted option"))
        .id()
        .clone();
    assert!(
        runtime
            .index()
            .node(&owner)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable)
    );
    assert!(
        runtime
            .index()
            .node(&option)
            .is_some_and(|n| !n.is_focusable())
    );
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("owner focus"));
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
    assert_eq!(runtime.focus().focused_node(), Some(&owner));

    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(500.0, 220.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("semantic publication: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    assert!(
        snapshot
            .nodes()
            .iter()
            .any(|n| n.role() == SemanticRole::ComboBox)
    );
    let option_node = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::Option && n.name() == Some("First"))
        .unwrap_or_else(|| unreachable!("standard proxy Option role"));
    assert_eq!(option_node.state().selected(), Some(true));
    assert!(
        option_node
            .supported_actions()
            .contains(&SemanticAction::Activate)
    );
    assert!(
        !option_node
            .supported_actions()
            .contains(&SemanticAction::RequestFocus)
    );
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}
