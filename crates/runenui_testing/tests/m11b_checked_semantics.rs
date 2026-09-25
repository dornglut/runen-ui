use core::num::NonZeroUsize;

use runenui_core::{
    Element, NoHostProtocol, SemanticAction, SemanticCheckedState, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, SemanticState, UiApp,
    View, Widget, WidgetActivation, WidgetActivationContext, WidgetActivationOutput,
    WidgetInvalidation,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug)]
struct Toggle;

#[derive(Debug)]
struct CheckableProbe {
    checked: bool,
}

impl Widget<Toggle> for CheckableProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<Toggle>,
    ) -> WidgetActivationOutput<Toggle> {
        context.invalidate(WidgetInvalidation::SEMANTICS);
        WidgetActivationOutput::changed_with_action(Toggle)
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Checkbox)
                .with_name("Feature")
                .with_state(SemanticState::ENABLED.with_checked(if self.checked {
                    SemanticCheckedState::Checked
                } else {
                    SemanticCheckedState::Unchecked
                }))
                .with_action(SemanticAction::Activate),
        )
    }
}

struct CheckedApp;

impl UiApp for CheckedApp {
    type State = bool;
    type Action = Toggle;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(CheckableProbe { checked: *state }).id("m11b.checked")
    }

    fn update(
        state: &mut Self::State,
        Toggle: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state = !*state;
    }
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

#[test]
fn checked_semantics_follow_application_state_through_ordinary_activation() {
    let unchecked = SemanticQuery::new()
        .with_role(SemanticRole::Checkbox)
        .with_name("Feature")
        .with_checked(SemanticCheckedState::Unchecked)
        .with_supported_action(SemanticAction::Activate);
    let checked = SemanticQuery::new()
        .with_role(SemanticRole::Checkbox)
        .with_name("Feature")
        .with_checked(SemanticCheckedState::Checked)
        .with_supported_action(SemanticAction::Activate);

    let mut harness = TestHarness::<CheckedApp>::mount(false);
    assert!(harness.publish().is_ok());
    let target = harness
        .unique_semantic_target(&unchecked)
        .unwrap_or_else(|error| unreachable!("unchecked semantic target is unique: {error:?}"));
    let before = harness
        .semantic_snapshot()
        .unwrap_or_else(|error| unreachable!("initial semantic snapshot is available: {error:?}"));
    let before_revision = before.revision();

    assert!(
        harness
            .submit_semantic_action(&target, SemanticAction::Activate)
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(*harness.state());
    assert!(harness.publish().is_ok());

    let matches = harness
        .query_semantics(&unchecked)
        .unwrap_or_else(|error| unreachable!("updated semantics remain queryable: {error:?}"));
    assert!(matches.is_empty());
    assert!(harness.unique_semantic_target(&checked).is_ok());
    let after = harness
        .semantic_snapshot()
        .unwrap_or_else(|error| unreachable!("updated semantic snapshot is available: {error:?}"));
    assert!(after.revision() > before_revision);
}
