use core::num::NonZeroUsize;

use runenui_core::{
    LayoutDimension, LayoutStyle, LogicalLength, NoHostProtocol, SemanticAction, SemanticRole,
    UiApp, View, button, children, column, text,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Activate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    enabled: bool,
    activations: usize,
}

struct ControlsApp;

impl UiApp for ControlsApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut control = button("Activate")
            .id("m11.button")
            .on_activate(|| Action::Activate)
            .with_layout(button_layout());
        if !state.enabled {
            control = control.disabled();
        }
        column(children![text("Status").id("m11.text"), control,])
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Activate => state.activations += 1,
        }
    }
}

fn button_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::Length(LogicalLength::from(96_u16)))
        .with_height(LayoutDimension::Length(LogicalLength::from(28_u16)))
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn button_query() -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("Activate")
        .with_supported_action(SemanticAction::Activate)
}

#[test]
fn text_is_static_semantic_content_without_activation() {
    let mut harness = TestHarness::<ControlsApp>::mount(State {
        enabled: true,
        activations: 0,
    });
    assert!(harness.publish().is_ok());

    let text_query = SemanticQuery::new()
        .with_role(SemanticRole::Text)
        .with_name("Status");
    assert!(harness.unique_semantic_target(&text_query).is_ok());

    let actionable_text = text_query.with_supported_action(SemanticAction::Activate);
    let matches = harness
        .query_semantics(&actionable_text)
        .unwrap_or_else(|error| unreachable!("published semantics remain queryable: {error:?}"));
    assert!(matches.is_empty());
}

#[test]
fn button_semantic_activation_is_repeatable_and_application_owned() {
    let mut harness = TestHarness::<ControlsApp>::mount(State {
        enabled: true,
        activations: 0,
    });
    assert!(harness.publish().is_ok());

    let target = harness
        .unique_semantic_target(&button_query())
        .unwrap_or_else(|error| unreachable!("button semantic target is unique: {error:?}"));

    for expected in [1, 2] {
        harness
            .submit_semantic_action(&target, SemanticAction::Activate)
            .unwrap_or_else(|error| {
                unreachable!("enabled button activation is accepted: {error:?}")
            });
        assert_eq!(
            harness.run_until_idle(settle_budget()).outcome(),
            SettleOutcome::Idle
        );
        assert_eq!(harness.state().activations, expected);
        assert!(harness.publish().is_ok());
    }
}

#[test]
fn disabled_button_remains_semantic_but_rejects_activation() {
    let mut harness = TestHarness::<ControlsApp>::mount(State {
        enabled: false,
        activations: 0,
    });
    assert!(harness.publish().is_ok());

    let target = harness
        .unique_semantic_target(&button_query().with_disabled(true))
        .unwrap_or_else(|error| {
            unreachable!("disabled button remains semantically visible: {error:?}")
        });

    assert!(
        harness
            .submit_semantic_action(&target, SemanticAction::Activate)
            .is_err()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.state().activations, 0);
}
