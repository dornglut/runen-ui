use core::num::NonZeroUsize;

use runenui_core::{
    NoHostProtocol, SemanticAction, SemanticCheckedState, SemanticRole, UiApp, View, checkbox,
    children, column, switch,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    ToggleCheckbox,
    ToggleSwitch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    checkbox: bool,
    switch: bool,
    enabled: bool,
    activations: usize,
}

struct BinaryControlsApp;

impl UiApp for BinaryControlsApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut checkbox = checkbox("Feature", state.checkbox)
            .id("m11.checkbox")
            .on_activate(|| Action::ToggleCheckbox);
        let mut switch = switch("Power", state.switch)
            .id("m11.switch")
            .on_activate(|| Action::ToggleSwitch);
        if !state.enabled {
            checkbox = checkbox.disabled();
            switch = switch.disabled();
        }
        column(children![checkbox, switch])
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.activations += 1;
        match action {
            Action::ToggleCheckbox => state.checkbox = !state.checkbox,
            Action::ToggleSwitch => state.switch = !state.switch,
        }
    }
}

struct StaticBinaryControlsApp;

impl UiApp for StaticBinaryControlsApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(children![
            checkbox("Unchecked", false).id("m11.checkbox.unchecked"),
            checkbox("Checked", true).id("m11.checkbox.checked"),
            checkbox("Mixed", SemanticCheckedState::Mixed).id("m11.checkbox.mixed"),
            switch("Off", false).id("m11.switch.off"),
            switch("On", true).id("m11.switch.on"),
        ])
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn checkbox_query(checked: SemanticCheckedState) -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Checkbox)
        .with_name("Feature")
        .with_checked(checked)
        .with_supported_action(SemanticAction::Activate)
}

fn switch_query(checked: SemanticCheckedState) -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Switch)
        .with_name("Power")
        .with_checked(checked)
        .with_supported_action(SemanticAction::Activate)
}

#[test]
fn passive_binary_controls_publish_exact_application_authored_state() {
    let mut harness = TestHarness::<StaticBinaryControlsApp>::mount(());
    assert!(harness.publish().is_ok());

    for (role, name, checked) in [
        (
            SemanticRole::Checkbox,
            "Unchecked",
            SemanticCheckedState::Unchecked,
        ),
        (
            SemanticRole::Checkbox,
            "Checked",
            SemanticCheckedState::Checked,
        ),
        (SemanticRole::Checkbox, "Mixed", SemanticCheckedState::Mixed),
        (SemanticRole::Switch, "Off", SemanticCheckedState::Unchecked),
        (SemanticRole::Switch, "On", SemanticCheckedState::Checked),
    ] {
        let query = SemanticQuery::new()
            .with_role(role)
            .with_name(name)
            .with_checked(checked);
        assert!(harness.unique_semantic_target(&query).is_ok());

        let actionable = query.with_supported_action(SemanticAction::Activate);
        let matches = harness
            .query_semantics(&actionable)
            .unwrap_or_else(|error| unreachable!("passive semantics remain queryable: {error:?}"));
        assert!(matches.is_empty());
    }
}

#[test]
fn binary_control_activation_updates_application_state_before_semantics() {
    let mut harness = TestHarness::<BinaryControlsApp>::mount(State {
        checkbox: false,
        switch: false,
        enabled: true,
        activations: 0,
    });
    assert!(harness.publish().is_ok());

    let checkbox_target = harness
        .unique_semantic_target(&checkbox_query(SemanticCheckedState::Unchecked))
        .unwrap_or_else(|error| unreachable!("unchecked checkbox is unique: {error:?}"));
    harness
        .submit_semantic_action(&checkbox_target, SemanticAction::Activate)
        .unwrap_or_else(|error| unreachable!("checkbox activation is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(harness.state().checkbox);
    assert_eq!(harness.state().activations, 1);
    assert!(harness.publish().is_ok());
    assert!(
        harness
            .unique_semantic_target(&checkbox_query(SemanticCheckedState::Checked))
            .is_ok()
    );

    let switch_target = harness
        .unique_semantic_target(&switch_query(SemanticCheckedState::Unchecked))
        .unwrap_or_else(|error| unreachable!("off switch is unique: {error:?}"));
    harness
        .submit_semantic_action(&switch_target, SemanticAction::Activate)
        .unwrap_or_else(|error| unreachable!("switch activation is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(harness.state().switch);
    assert_eq!(harness.state().activations, 2);
    assert!(harness.publish().is_ok());
    assert!(
        harness
            .unique_semantic_target(&switch_query(SemanticCheckedState::Checked))
            .is_ok()
    );

    let checkbox_target = harness
        .unique_semantic_target(&checkbox_query(SemanticCheckedState::Checked))
        .unwrap_or_else(|error| unreachable!("checked checkbox is unique: {error:?}"));
    harness
        .submit_semantic_action(&checkbox_target, SemanticAction::Activate)
        .unwrap_or_else(|error| unreachable!("repeat checkbox activation is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(!harness.state().checkbox);
    assert_eq!(harness.state().activations, 3);
    assert!(harness.publish().is_ok());
    assert!(
        harness
            .unique_semantic_target(&checkbox_query(SemanticCheckedState::Unchecked))
            .is_ok()
    );
}

#[test]
fn disabled_binary_controls_remain_semantic_and_reject_activation() {
    let mut harness = TestHarness::<BinaryControlsApp>::mount(State {
        checkbox: false,
        switch: true,
        enabled: false,
        activations: 0,
    });
    assert!(harness.publish().is_ok());

    let checkbox = checkbox_query(SemanticCheckedState::Unchecked).with_disabled(true);
    let switch = switch_query(SemanticCheckedState::Checked).with_disabled(true);
    let checkbox_target = harness
        .unique_semantic_target(&checkbox)
        .unwrap_or_else(|error| unreachable!("disabled checkbox remains semantic: {error:?}"));
    let switch_target = harness
        .unique_semantic_target(&switch)
        .unwrap_or_else(|error| unreachable!("disabled switch remains semantic: {error:?}"));

    assert!(
        harness
            .submit_semantic_action(&checkbox_target, SemanticAction::Activate)
            .is_err()
    );
    assert!(
        harness
            .submit_semantic_action(&switch_target, SemanticAction::Activate)
            .is_err()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(
        *harness.state(),
        State {
            checkbox: false,
            switch: true,
            enabled: false,
            activations: 0,
        }
    );
}
