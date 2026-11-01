use core::num::NonZeroUsize;

use runenui_core::{
    Element, NoHostProtocol, SemanticAction, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, SemanticState, UiApp, View, Widget, WidgetActivation,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Availability {
    Enabled,
    Disabled,
    Inert,
    Hidden,
}

impl Availability {
    const fn semantic_state(self) -> SemanticState {
        match self {
            Self::Enabled => SemanticState::ENABLED,
            Self::Disabled => SemanticState::ENABLED.with_disabled(true),
            Self::Inert => SemanticState::ENABLED.with_inert(true),
            Self::Hidden => SemanticState::ENABLED.with_hidden(true),
        }
    }
}

#[derive(Debug)]
struct SemanticProbe {
    availability: Availability,
}

impl Widget<()> for SemanticProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name("Probe")
                .with_state(self.availability.semantic_state())
                .with_action(SemanticAction::Activate),
        )
    }
}

struct StateApp;

impl StateApp {
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

impl UiApp for StateApp {
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

    type State = Availability;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(SemanticProbe {
            availability: *state,
        })
        .id("semantic.probe")
    }

    fn update(
        _: &mut Self::State,
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

fn probe_query() -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("Probe")
        .with_supported_action(SemanticAction::Activate)
}

#[test]
fn disabled_and_inert_nodes_remain_queryable_but_fail_action_admission() {
    for (availability, query) in [
        (Availability::Disabled, probe_query().with_disabled(true)),
        (Availability::Inert, probe_query().with_inert(true)),
    ] {
        let mut harness = TestHarness::<StateApp>::mount(availability).unwrap_or_else(|error| {
            unreachable!("fixture initial admission must succeed: {error}")
        });
        assert!(harness.publish().is_ok());
        let Ok(target) = harness.unique_semantic_target(&query) else {
            unreachable!("disabled or inert probe must be uniquely queryable");
        };
        assert!(
            harness
                .submit_semantic_action(&target, SemanticAction::Activate)
                .is_err()
        );
    }
}

#[test]
#[allow(clippy::assert_is_empty)]
fn hidden_nodes_are_absent_from_the_committed_semantic_snapshot() {
    let mut harness = TestHarness::<StateApp>::mount(Availability::Hidden)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(harness.publish().is_ok());

    let Ok(matches) = harness.query_semantics(&probe_query()) else {
        unreachable!("semantic query must succeed after publication");
    };
    assert!(matches.is_empty());
}

#[test]
fn enabled_action_is_accepted_but_snapshot_scope_cannot_cross_runtimes() {
    let mut first = TestHarness::<StateApp>::mount(Availability::Enabled)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(first.publish().is_ok());
    let Ok(target) = first.unique_semantic_target(&probe_query()) else {
        unreachable!("enabled probe must be uniquely queryable");
    };
    assert!(
        first
            .submit_semantic_action(&target, SemanticAction::Activate)
            .is_ok()
    );
    assert_eq!(
        first.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );

    let mut foreign = TestHarness::<StateApp>::mount(Availability::Enabled)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(foreign.publish().is_ok());
    assert!(
        foreign
            .submit_semantic_action(&target, SemanticAction::Activate)
            .is_err()
    );
}
