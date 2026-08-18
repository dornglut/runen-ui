#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;

use runenui_core::{IntoEffects, NoHostProtocol, SemanticAction, SemanticRole, UiApp, View};
use runenui_external_widget_conformance::{ChildAction, ParentAction, parent_view};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Debug, Default, Eq, PartialEq)]
struct MappedState {
    pulses: usize,
    resets: usize,
}

struct MappedApp;

impl UiApp for MappedApp {
    type State = MappedState;
    type Action = ParentAction;
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> impl View<Self::Action> {
        parent_view()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            ParentAction::Child(ChildAction::Pulse) => state.pulses += 1,
            ParentAction::Reset => state.resets += 1,
        }
    }
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn trace_position(kinds: &[&str], expected: &str) -> usize {
    kinds
        .iter()
        .position(|kind| *kind == expected)
        .unwrap_or_else(|| panic!("missing canonical trace record {expected}"))
}

#[test]
fn mapped_downstream_widget_preserves_semantics_action_runtime_and_trace_authority() {
    let mut harness = TestHarness::<MappedApp>::mount(MappedState::default());
    assert!(harness.publish().is_ok());

    let Ok(snapshot) = harness.semantic_snapshot() else {
        unreachable!("explicit downstream publication produces a semantic snapshot")
    };
    let baseline_surface = snapshot.surface_id().clone();
    let baseline_revision = snapshot.revision();

    let pulse = SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("Pulse")
        .with_supported_action(SemanticAction::Activate);
    let target = harness
        .unique_semantic_target(&pulse)
        .unwrap_or_else(|error| {
            unreachable!("mapped downstream semantic target is unique: {error:?}")
        });

    harness
        .submit_semantic_action(&target, SemanticAction::Activate)
        .unwrap_or_else(|error| {
            unreachable!("mapped downstream semantic action is accepted: {error:?}")
        });
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(
        harness.state(),
        &MappedState {
            pulses: 1,
            resets: 0
        }
    );

    assert!(harness.publish().is_ok());
    let Ok(snapshot) = harness.semantic_snapshot() else {
        unreachable!("republished downstream semantics remain available")
    };
    assert_eq!(snapshot.surface_id(), &baseline_surface);
    assert_eq!(
        snapshot.revision(),
        baseline_revision,
        "application-only state change must not invent a semantic update"
    );

    let replay = harness
        .trace_replay()
        .unwrap_or_else(|error| unreachable!("canonical downstream trace replays: {error}"));
    assert!(replay.is_complete());
    let kinds = replay
        .records()
        .map(|record| record.kind().as_str())
        .collect::<Vec<_>>();

    let bound = trace_position(&kinds, "semantic_action_bound");
    let accepted = trace_position(&kinds, "command_submission_accepted");
    let routed = trace_position(&kinds, "routed_event_started");
    let defaulted = trace_position(&kinds, "semantic_default_applied");
    let action = trace_position(&kinds, "action_submission_accepted");
    let updated = trace_position(&kinds, "application_state_updated");
    assert!(bound < accepted);
    assert!(accepted < routed);
    assert!(routed < defaulted);
    assert!(defaulted < action);
    assert!(action < updated);
}
