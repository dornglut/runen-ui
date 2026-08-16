use core::num::NonZeroUsize;

use runenui_core::{
    Element, IntoEffects, NoHostProtocol, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, SemanticState, UiApp,
    View, Widget, WidgetActivation,
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

impl UiApp for StateApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
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
        let mut harness = TestHarness::<StateApp>::mount(availability);
        assert!(harness.publish().is_ok());
        let Ok(target) = harness.unique_semantic_target(&query) else {
            return;
        };
        assert!(
            harness
                .submit_semantic_action(&target, SemanticAction::Activate)
                .is_err()
        );
    }
}

#[test]
fn hidden_nodes_are_absent_from_the_committed_semantic_snapshot() {
    let mut harness = TestHarness::<StateApp>::mount(Availability::Hidden);
    assert!(harness.publish().is_ok());

    let Ok(matches) = harness.query_semantics(&probe_query()) else {
        return;
    };
    assert!(matches.is_empty());
}

#[test]
fn enabled_action_is_accepted_but_snapshot_scope_cannot_cross_runtimes() {
    let mut first = TestHarness::<StateApp>::mount(Availability::Enabled);
    assert!(first.publish().is_ok());
    let Ok(target) = first.unique_semantic_target(&probe_query()) else {
        return;
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

    let mut foreign = TestHarness::<StateApp>::mount(Availability::Enabled);
    assert!(foreign.publish().is_ok());
    assert!(
        foreign
            .submit_semantic_action(&target, SemanticAction::Activate)
            .is_err()
    );
}
