//! Internal constructor access, exercising the same actual public widget contract.
#![allow(refining_impl_trait)]

use super::AppRuntime;
use crate::{
    InputArbitrationRecord, PumpBudget, UiInputClaimReason, UiInputConflict, UiInputFinality, pump,
};
use runenui_core::{
    CommandOrigin, Element, ElementId, EventContext, EventPhase, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput,
};

#[derive(Debug)]
struct HostClaimProbe {
    claim: bool,
}

enum ProbeAction {}

impl Widget<ProbeAction> for HostClaimProbe {
    type State = ();
    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ProbeAction>,
    ) -> WidgetEventOutput {
        if matches!(event, UiEvent::Keyboard(_)) && context.phase() == EventPhase::Target {
            if self.claim {
                context.claim_host_input();
            } else {
                context.prevent_default();
                context.stop_propagation();
            }
        }
        WidgetEventOutput::none()
    }
}

struct ProbeApp;
impl UiApp for ProbeApp {
    type State = bool;
    type Action = ProbeAction;
    type HostProtocol = NoHostProtocol;

    fn root(claim: &Self::State) -> impl View<Self::Action> {
        Element::new(HostClaimProbe { claim: *claim })
            .id("probe")
            .key("probe")
            .focusable(true)
    }

    fn update(_: &mut Self::State, never: Self::Action) {
        match never {}
    }
}

fn focused_runtime(claim: bool) -> AppRuntime<ProbeApp> {
    let mut app = AppRuntime::<ProbeApp>::mount(claim);
    let budget = PumpBudget::new(16, 16, 16, 16);
    let _ = app.pump(budget);
    let id = ElementId::new("probe").unwrap_or_else(|| unreachable!("fixture id"));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("mounted fixture"))
        .id()
        .clone();
    app.submit_command(
        target.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus admission"));
    let _ = app.pump(budget);
    assert_eq!(app.focus().focused_node(), Some(&target));
    app
}

fn key() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyW")),
        LogicalKey::Character(String::from("w")),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn only_explicit_widget_claim_is_exclusive_not_propagation_or_default_control() {
    for claim in [false, true] {
        let mut app = focused_runtime(claim);
        let receipt = app
            .submit_keyboard(key())
            .unwrap_or_else(|_| unreachable!("focused admission"))
            .sequence();
        let batch = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("canonical pump observation"));
        assert_eq!(batch.processed_through(), Some(receipt));
        let settled = batch
            .ordered_records()
            .iter()
            .find_map(|record| match record {
                InputArbitrationRecord::InputSettled(settled) => Some(settled),
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("one exact reached input settlement"));
        assert_eq!(settled.sequence(), receipt);
        assert_eq!(settled.scope(), batch.final_ownership().scope());
        assert_eq!(
            settled.ownership_revision(),
            batch.final_ownership().revision()
        );
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("successful callback transaction must commit");
        };
        assert_eq!(
            facts.conflict(),
            if claim {
                UiInputConflict::ExclusiveUi
            } else {
                UiInputConflict::ObservedNonexclusive
            },
        );
        assert_eq!(
            facts
                .reasons()
                .contains(&UiInputClaimReason::ExplicitWidgetClaim),
            claim,
        );
        assert_eq!(facts.default_prevented(), !claim);
        assert_eq!(facts.propagation_stopped(), !claim);
    }
}

#[test]
fn terminal_scope_invalidates_unprocessed_native_receipts_without_fake_settlement() {
    let mut app = focused_runtime(false);
    let pending = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("receipt is admitted"))
        .sequence();
    let _ = app.shutdown();
    let batch = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("closed runtime still reports its retirement"));
    assert_eq!(batch.processed_through(), None);
    assert!(
        batch
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
    assert!(!batch.ordered_records().iter().any(|record| matches!(
        record, InputArbitrationRecord::InputSettled(settled) if settled.sequence() == pending
    )));
    let second = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("repeated observation is allowed"));
    assert!(
        !second
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
}

#[test]
fn explicit_shutdown_reports_final_closed_ownership_and_one_scope_retirement() {
    let mut app = focused_runtime(false);
    let receipt = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("pending input admitted before shutdown"))
        .sequence();
    let first = app
        .runtime
        .shutdown_observed()
        .unwrap_or_else(|_| unreachable!("preflighted canonical shutdown observation"));
    assert_eq!(
        first.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    assert_eq!(first.ordered_records().len(), 2);
    assert!(matches!(
        first.ordered_records()[0],
        InputArbitrationRecord::OwnershipChanged(_)
    ));
    assert!(matches!(
        first.ordered_records()[1],
        InputArbitrationRecord::ScopeRetired(_)
    ));
    assert!(!first.ordered_records().iter().any(|record| matches!(
        record, InputArbitrationRecord::InputSettled(settled) if settled.sequence() == receipt
    )));
    let repeated = app
        .runtime
        .shutdown_observed()
        .unwrap_or_else(|_| unreachable!("repeated close is idempotent"));
    assert_eq!(
        repeated.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    assert!(repeated.ordered_records().is_empty());
}

#[test]
fn failed_observation_preflight_does_not_pop_or_settle_input() {
    let mut app = focused_runtime(false);
    let receipt = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("focused key admission"))
        .sequence();
    app.runtime.inject_input_reservation_failure_after(0);
    let outcome = pump::pump_recorded::<ProbeApp>(
        &mut app.runtime,
        PumpBudget::new(1, 0, 0, 0),
    );
    assert!(matches!(outcome, Err(crate::InputObservationError::Capacity)));
    assert_eq!(app.runtime.status(), crate::RuntimeStatus::Running);

    let resumed = pump::pump_recorded::<ProbeApp>(
        &mut app.runtime,
        PumpBudget::new(1, 0, 0, 0),
    )
    .unwrap_or_else(|_| unreachable!("preflight fault is one-shot"));
    assert_eq!(resumed.processed_through(), Some(receipt));
    let settled = resumed.ordered_records().iter().filter_map(|record| {
        match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.sequence()),
            _ => None,
        }
    }).collect::<Vec<_>>();
    assert_eq!(settled, vec![receipt]);
}

#[test]
fn mid_pump_snapshot_capacity_failure_returns_lossless_partial_batch() {
    let mut app = focused_runtime(false);
    let first = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("first admitted"))
        .sequence();
    let second = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("second admitted"))
        .sequence();

    // One reservation for the initial read, one for the initial readiness
    // checkpoint, and one for the first canonical FIFO envelope. Refuse the
    // next checkpoint before it can mutate or dequeue the next envelope.
    app.runtime.inject_input_reservation_failure_after(3);
    let partial = pump::pump_recorded::<ProbeApp>(
        &mut app.runtime,
        PumpBudget::new(2, 0, 0, 0),
    )
    .unwrap_or_else(|_| unreachable!("after progress the result is a partial success"));
    assert_eq!(
        partial.pause_reason(),
        Some(crate::InputPumpPauseReason::ObservationCapacity)
    );
    assert_eq!(partial.processed_through(), Some(first));
    let first_settled = partial.ordered_records().iter().filter_map(|record| {
        match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.sequence()),
            _ => None,
        }
    }).collect::<Vec<_>>();
    assert_eq!(first_settled, vec![first]);

    let resumed = pump::pump_recorded::<ProbeApp>(
        &mut app.runtime,
        PumpBudget::new(1, 0, 0, 0),
    )
    .unwrap_or_else(|_| unreachable!("next retained receipt settles once"));
    assert_eq!(resumed.processed_through(), Some(second));
    let second_settled = resumed.ordered_records().iter().filter_map(|record| {
        match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.sequence()),
            _ => None,
        }
    }).collect::<Vec<_>>();
    assert_eq!(second_settled, vec![second]);
}
