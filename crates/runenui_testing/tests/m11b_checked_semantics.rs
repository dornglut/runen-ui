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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(1)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

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

impl CheckedApp {
    const fn producer_budget() -> runenui_core::ProducerBudget {
        // Closed fixture: at most 1 mounted owner in every reachable tree.
        // Each event callback emits at most one output: four focus visits and
        // two ordinary route visits per owner, plus activation/default output.
        // New owners add one subscription reconciliation each; descendants
        // never start work, declare subscriptions, or grow this tree bound.
        let owners = 1;
        let outputs = 6 * owners + 2;
        runenui_core::ProducerBudget::new(outputs + owners + 1, outputs)
            .with_reconciliation(owners, owners, owners)
    }
}

impl UiApp for CheckedApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }
    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }
    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

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
#[allow(clippy::assert_is_empty)]
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

    let mut harness = TestHarness::<CheckedApp>::mount(false)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
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
