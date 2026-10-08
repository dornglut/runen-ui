use core::num::NonZeroUsize;

use runenui_core::{
    Element, NoHostProtocol, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticPressedState,
    SemanticRole, SemanticState, UiApp, View, Widget, button, children, column,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug)]
enum Action {
    Activate,
    Author(Option<SemanticPressedState>),
    Enable(bool),
}

#[derive(Clone, Copy, Debug)]
struct State {
    pressed: Option<SemanticPressedState>,
    enabled: bool,
    activations: u32,
}

struct ButtonApp;

impl UiApp for ButtonApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut button = button("Pin").id("m11.pressed").on_activate(|| Action::Activate);
        if let Some(pressed) = state.pressed {
            button = button.pressed(pressed);
        }
        if !state.enabled {
            button = button.disabled();
        }
        button
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Activate => state.activations += 1,
            Action::Author(pressed) => state.pressed = pressed,
            Action::Enable(enabled) => state.enabled = enabled,
        }
    }
}

fn budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn button_query() -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("Pin")
        .with_supported_action(SemanticAction::Activate)
}

fn assert_pressed(harness: &TestHarness<ButtonApp>, pressed: Option<SemanticPressedState>) {
    let target = harness
        .unique_semantic_target(&button_query())
        .unwrap_or_else(|error| unreachable!("button must have one semantic node: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|error| unreachable!("published snapshot must exist: {error:?}"));
    let node = snapshot
        .node(target.node_id())
        .unwrap_or_else(|| unreachable!("the queried node must be in the snapshot"));
    assert_eq!(node.state().pressed(), pressed);
    assert_eq!(node.name(), Some("Pin"));
}

fn initial_state(pressed: Option<SemanticPressedState>) -> State {
    State {
        pressed,
        enabled: true,
        activations: 0,
    }
}

#[test]
fn button_without_authored_pressed_state_remains_momentary() {
    let mut harness = TestHarness::<ButtonApp>::mount(initial_state(None));
    assert!(harness.publish().is_ok());
    assert_pressed(&harness, None);
    let target = harness.unique_semantic_target(&button_query()).unwrap();
    assert!(harness.submit_semantic_action(&target, SemanticAction::Activate).is_ok());
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
    assert_eq!(harness.state().activations, 1);
    assert_eq!(harness.state().pressed, None);
    assert!(harness.publish().is_ok());
    assert_pressed(&harness, None);
}

#[test]
fn button_pressed_semantics_follow_only_application_authored_updates() {
    let mut harness = TestHarness::<ButtonApp>::mount(initial_state(Some(
        SemanticPressedState::Unpressed,
    )));
    assert!(harness.publish().is_ok());
    assert_pressed(&harness, Some(SemanticPressedState::Unpressed));
    let original_revision = harness.semantic_snapshot().unwrap().revision();

    let target = harness.unique_semantic_target(&button_query()).unwrap();
    assert!(harness.submit_semantic_action(&target, SemanticAction::Activate).is_ok());
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
    assert_eq!(harness.state().activations, 1);
    assert_eq!(harness.state().pressed, Some(SemanticPressedState::Unpressed));
    assert!(harness.publish().is_ok());
    assert_pressed(&harness, Some(SemanticPressedState::Unpressed));
    assert_eq!(harness.semantic_snapshot().unwrap().revision(), original_revision);

    let mut previous = original_revision;
    for pressed in [SemanticPressedState::Pressed, SemanticPressedState::Mixed] {
        assert!(harness.submit_action(Action::Author(Some(pressed))).is_ok());
        assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
        assert!(harness.publish().is_ok());
        assert_pressed(&harness, Some(pressed));
        let next = harness.semantic_snapshot().unwrap().revision();
        assert!(next > previous);
        previous = next;

        assert!(harness.submit_action(Action::Author(Some(pressed))).is_ok());
        assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
        assert!(harness.publish().is_ok());
        assert_eq!(harness.semantic_snapshot().unwrap().revision(), previous);
    }

    assert!(harness.submit_action(Action::Author(None)).is_ok());
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
    assert!(harness.publish().is_ok());
    assert_pressed(&harness, None);
}

#[test]
fn disabled_pressed_button_stays_semantic_and_rejects_activation() {
    let mut harness = TestHarness::<ButtonApp>::mount(initial_state(Some(
        SemanticPressedState::Pressed,
    )));
    assert!(harness.submit_action(Action::Enable(false)).is_ok());
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
    assert!(harness.publish().is_ok());
    assert_pressed(&harness, Some(SemanticPressedState::Pressed));

    let target = harness.unique_semantic_target(&button_query().with_disabled(true)).unwrap();
    assert!(harness.submit_semantic_action(&target, SemanticAction::Activate).is_err());
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
    assert_eq!(harness.state().activations, 0);
    assert_pressed(&harness, Some(SemanticPressedState::Pressed));
}

#[derive(Debug)]
struct DownstreamPressed {
    pressed: SemanticPressedState,
}

impl Widget<Action> for DownstreamPressed {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name("Custom Pin")
                .with_state(SemanticState::ENABLED.with_pressed(self.pressed)),
        )
    }
}

struct ParityApp;

impl UiApp for ParityApp {
    type State = SemanticPressedState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(pressed: &Self::State) -> impl View<Self::Action> {
        column(children![
            button("Pin").pressed(*pressed).id("built-in"),
            Element::new(DownstreamPressed { pressed: *pressed }).id("downstream"),
        ])
    }

    fn update(
        _: &mut Self::State,
        _: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn downstream_custom_widget_uses_identical_public_pressed_semantic_contract() {
    let mut harness = TestHarness::<ParityApp>::mount(SemanticPressedState::Mixed);
    assert!(harness.publish().is_ok());
    let snapshot = harness.semantic_snapshot().unwrap();
    let roles = snapshot
        .nodes()
        .iter()
        .filter(|node| {
            node.role() == SemanticRole::Button
                && matches!(node.name(), Some("Pin" | "Custom Pin"))
        })
        .map(|node| node.state().pressed())
        .collect::<Vec<_>>();
    assert_eq!(roles, vec![Some(SemanticPressedState::Mixed); 2]);
}
