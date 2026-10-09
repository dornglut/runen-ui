use core::num::NonZeroUsize;

use runenui_core::{
    Element, EventContext, EventPhase, NoHostProtocol, SemanticAction, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution, SemanticRole,
    SemanticState, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation, WidgetUpdateContext, column,
    disclosure,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug)]
enum Action {
    Standard,
    Custom,
}
#[derive(Clone, Copy, Debug)]
struct State {
    standard: bool,
    custom: bool,
}
#[derive(Debug)]
struct CustomDisclosure {
    expanded: bool,
}
impl Widget<Action> for CustomDisclosure {
    type State = bool;

    fn create_state(&self) -> Self::State {
        self.expanded
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.expanded {
            *state = self.expanded;
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _: &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Custom)
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target || context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        let Some(command) = event.as_semantic_command() else {
            return WidgetEventOutput::none();
        };
        if matches!(
            (*state, command.command()),
            (false, SemanticCommand::Expand) | (true, SemanticCommand::Collapse)
        ) {
            context.prevent_default();
            context.stop_propagation();
            context.emit(Action::Custom);
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Button)
            .with_name("Custom")
            .with_state(SemanticState::ENABLED.with_expanded(*state))
            .with_action(SemanticAction::Activate);
        node = node.with_action(if *state {
            SemanticAction::Collapse
        } else {
            SemanticAction::Expand
        });
        SemanticContribution::single(node)
    }
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        column([
            disclosure("Standard", state.standard)
                .id("standard")
                .on_activate(|| Action::Standard)
                .into_element(),
            Element::new(CustomDisclosure {
                expanded: state.custom,
            })
            .id("custom")
            .into_element(),
        ])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Standard => state.standard = !state.standard,
            Action::Custom => state.custom = !state.custom,
        }
    }
}

fn settle(h: &mut TestHarness<App>) {
    let budget = SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    );
    assert_eq!(h.run_until_idle(budget).outcome(), SettleOutcome::Idle);
}
fn target(h: &TestHarness<App>, name: &str) -> runenui_testing::SemanticTarget {
    h.unique_semantic_target(
        &SemanticQuery::new()
            .with_role(SemanticRole::Button)
            .with_name(name),
    )
    .unwrap_or_else(|error| unreachable!("semantic target exists: {error:?}"))
}
fn parity(h: &TestHarness<App>, expanded: bool) {
    let snapshot = h
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("published"));
    for name in ["Standard", "Custom"] {
        let id = target(h, name);
        let node = snapshot
            .node(id.node_id())
            .unwrap_or_else(|| unreachable!("target present"));
        assert_eq!(node.state().expanded(), Some(expanded));
        assert_eq!(node.actions().contains(&SemanticAction::Activate), true);
        assert_eq!(node.actions().contains(&SemanticAction::Expand), !expanded);
        assert_eq!(node.actions().contains(&SemanticAction::Collapse), expanded);
    }
}
#[test]
fn downstream_widget_matches_public_disclosure_routing_and_app_owned_rebuild() {
    let mut h = TestHarness::<App>::mount(State {
        standard: false,
        custom: false,
    });
    assert!(h.publish().is_ok());
    parity(&h, false);
    for name in ["Standard", "Custom"] {
        let t = target(&h, name);
        assert!(h.submit_semantic_action(&t, SemanticAction::Expand).is_ok());
        settle(&mut h);
        assert!(h.publish().is_ok());
    }
    assert!(h.state().standard && h.state().custom);
    parity(&h, true);
    for name in ["Standard", "Custom"] {
        let t = target(&h, name);
        assert!(
            h.submit_semantic_action(&t, SemanticAction::Collapse)
                .is_ok()
        );
        settle(&mut h);
        assert!(h.publish().is_ok());
    }
    assert!(!h.state().standard && !h.state().custom);
    parity(&h, false);
}
