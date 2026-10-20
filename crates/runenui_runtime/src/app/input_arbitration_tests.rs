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
    focused_runtime_with_config(claim, crate::RuntimeConfig::default())
}

fn focused_runtime_with_config(claim: bool, config: crate::RuntimeConfig) -> AppRuntime<ProbeApp> {
    let mut app = AppRuntime::<ProbeApp>::mount_with_config(claim, config);
    let budget = PumpBudget::new(16, 16, 16, 16);
    let _ = app.pump(budget);
    let id = ElementId::new("probe").unwrap_or_else(|_| unreachable!("fixture id"));
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
fn intervening_non_input_work_does_not_create_or_reorder_native_receipts() {
    let mut app = focused_runtime(true);
    let target = app
        .focus()
        .focused_node()
        .unwrap_or_else(|| unreachable!("fixture is focused"))
        .clone();
    let first = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("first native input accepted"))
        .sequence();
    let action = app
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("unrelated semantic work accepted"))
        .sequence();
    let second = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("second native input accepted"))
        .sequence();

    assert!(first < action && action < second);
    let batch = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(3, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("canonical mixed FIFO settles"));
    assert_eq!(batch.processed_through(), Some(second));
    assert_eq!(batch.report().processed_envelopes(), 3);
    let settlements = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settlements.len(), 2);
    assert_eq!(settlements[0].sequence(), first);
    assert_eq!(settlements[1].sequence(), second);
    assert_eq!(settlements[0].scope(), settlements[1].scope());
    for settled in settlements {
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("explicit host claims committed");
        };
        assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
        assert!(
            facts
                .reasons()
                .contains(&UiInputClaimReason::ExplicitWidgetClaim)
        );
    }
}

#[test]
fn coalesced_direct_publications_expose_a_revision_gap_without_claiming_present() {
    use crate::{LogicalSize, SurfaceBuildContext};
    use runenui_core::StyleEnvironment;

    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(16, 16, 16, 16));
    let before = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("initial ownership projection"));
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("finite logical viewport"));
    let build = SurfaceBuildContext::tight(&environment, size);
    let first = app
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("first logical publication"));
    let second = app
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("second logical publication"));
    assert_ne!(
        first.input_context().hit_test_generation(),
        second.input_context().hit_test_generation(),
    );
    // Neither publication was followed by a pump or intermediate host query.
    // The gap tells a host it cannot infer uninterrupted input eligibility.
    let observed = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("post-publication ownership projection"));
    assert_eq!(observed.revision().get(), before.revision().get() + 2);
    assert_eq!(
        observed.surfaces()[0]
            .latest_retained_context()
            .unwrap_or_else(|| unreachable!("latest logical retained context")),
        second.input_context(),
    );
    assert_eq!(
        app.input_ownership()
            .unwrap_or_else(|_| unreachable!("repeat snapshot is unchanged"))
            .revision(),
        observed.revision(),
    );
}

#[test]
fn trace_disabled_keeps_exact_host_receipt_and_explicit_widget_conflict() {
    use crate::{RuntimeConfig, TraceConfig};

    let mut app = AppRuntime::<ProbeApp>::mount_with_config(
        true,
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    let budget = PumpBudget::new(16, 16, 16, 16);
    let _ = app.pump(budget);
    let id = ElementId::new("probe").unwrap_or_else(|_| unreachable!("fixture identity"));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("mounted fixture"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus accepted without trace"));
    let _ = app.pump(budget);
    let receipt = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("native keyboard admission without trace"))
        .sequence();
    let batch = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("trace-independent canonical settlement"));
    assert_eq!(batch.processed_through(), Some(receipt));
    let facts = batch
        .ordered_records()
        .iter()
        .find_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) if settled.sequence() == receipt => {
                Some(settled.finality())
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("exact native receipt exists without trace"));
    let UiInputFinality::Committed(facts) = facts else {
        unreachable!("routed trace-disabled event committed");
    };
    assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
    assert!(
        facts
            .reasons()
            .contains(&UiInputClaimReason::ExplicitWidgetClaim)
    );
}

#[test]
fn terminal_scope_invalidates_unprocessed_native_receipts_without_fake_settlement() {
    let mut app = focused_runtime(false);
    let pending = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("receipt is admitted"))
        .sequence();
    let batch = app
        .shutdown()
        .unwrap_or_else(|_| unreachable!("public shutdown returns the final retirement"));
    assert_eq!(batch.report().cancelled_queued_envelopes(), 1);
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
    assert_eq!(repeated.ordered_records(), []);
}

#[test]
fn failed_observation_preflight_does_not_pop_or_settle_input() {
    let mut app = focused_runtime(false);
    let receipt = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("focused key admission"))
        .sequence();
    app.runtime.inject_input_reservation_failure_after(0);
    let outcome = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0));
    assert!(matches!(
        outcome,
        Err(crate::InputObservationError::Capacity)
    ));
    assert_eq!(app.runtime.status(), crate::RuntimeStatus::Running);

    let resumed = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("preflight fault is one-shot"));
    assert_eq!(resumed.processed_through(), Some(receipt));
    let settled = resumed
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.sequence()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settled, vec![receipt]);
}

#[test]
fn initial_checkpoint_capacity_shortage_preserves_prior_ownership_observation_and_pending_input() {
    let mut app = focused_runtime(false);
    let pending = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("pending input accepted"))
        .sequence();
    // The first reservation belongs to the initial ownership snapshot. The
    // second would prepare the initial zero-work readiness checkpoint.
    app.runtime.inject_input_reservation_failure_after(1);
    let partial = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("snapshot after initial progress must be returned"));
    assert_eq!(
        partial.pause_reason(),
        Some(crate::InputPumpPauseReason::ObservationCapacity)
    );
    assert_eq!(partial.processed_through(), None);
    assert_eq!(partial.report().processed_envelopes(), 0);
    assert_eq!(
        partial.final_ownership().status(),
        crate::RuntimeStatus::Running
    );
    assert_eq!(partial.ordered_records(), []);

    let resumed = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pending input remains queued"));
    assert_eq!(resumed.processed_through(), Some(pending));
    assert_eq!(
        resumed
            .ordered_records()
            .iter()
            .filter(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
            .count(),
        1
    );
}

#[test]
fn initial_snapshot_revision_after_direct_composition_admission_survives_capacity_pause() {
    let mut app = AppRuntime::<TextOwnershipApp>::mount(());
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(&ElementId::new("editor").unwrap_or_else(|_| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!("editable owner mounted"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus admission"));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let before = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("initial observed focus owner"));
    let receipt = app
        .start_composition(Some(device(43)))
        .unwrap_or_else(|_| unreachable!("pending native composition admitted"));
    // First reservation publishes the already-committed synchronous admission
    // revision. The following initial checkpoint reservation fails. Returning
    // Err here would lose the ownership observation, even at zero FIFO work.
    app.runtime.inject_input_reservation_failure_after(1);
    let partial =
        pump::pump_recorded::<TextOwnershipApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("revision is preserved as partial success"));
    assert_eq!(
        partial.pause_reason(),
        Some(crate::InputPumpPauseReason::ObservationCapacity)
    );
    assert_eq!(partial.processed_through(), None);
    assert_eq!(partial.ordered_records(), []);
    assert!(partial.final_ownership().revision() > before.revision());
    assert_eq!(
        partial
            .final_ownership()
            .keyboard()
            .composition_generation(),
        Some(receipt.generation())
    );
    assert_eq!(
        partial.final_ownership().keyboard().composition_device_id(),
        Some(device(43))
    );
    let resumed =
        pump::pump_recorded::<TextOwnershipApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("pending input resumes"));
    assert_eq!(resumed.processed_through(), Some(receipt.sequence()));
    assert_eq!(
        resumed
            .ordered_records()
            .iter()
            .filter(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
            .count(),
        1
    );
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
    let partial = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("after progress the result is a partial success"));
    assert_eq!(
        partial.pause_reason(),
        Some(crate::InputPumpPauseReason::ObservationCapacity)
    );
    assert_eq!(partial.processed_through(), Some(first));
    let first_settled = partial
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.sequence()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(first_settled, vec![first]);

    let resumed = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("next retained receipt settles once"));
    assert_eq!(resumed.processed_through(), Some(second));
    let second_settled = resumed
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.sequence()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(second_settled, vec![second]);
}

#[test]
fn every_observation_reservation_boundary_preserves_fifo_finality_and_bounded_records() {
    // A three-envelope pump has the initial read, at most nine readiness
    // checkpoints, and three envelope reservations. Sweep that entire range,
    // including reservations reached only by subsequent bounded pumps.
    for trace_capacity in [0, 1, 512] {
        for failed_reservation in 0..=13 {
            let config = crate::RuntimeConfig::default()
                .with_trace_config(crate::TraceConfig::new(trace_capacity));
            let mut app = focused_runtime_with_config(false, config);
            let before = app
                .input_ownership()
                .unwrap_or_else(|_| unreachable!("ownership"));
            let receipts = (0..3)
                .map(|_| {
                    app.submit_keyboard(key())
                        .unwrap_or_else(|_| unreachable!("admitted"))
                        .sequence()
                })
                .collect::<Vec<_>>();
            app.runtime
                .inject_input_reservation_failure_after(failed_reservation);
            let mut settled = Vec::new();
            let mut last_fence = None;
            for budget in [0, 3, 1, 1, 1, 1, 1] {
                match app.pump(PumpBudget::new(budget, 0, 0, 0)) {
                    Err(crate::InputObservationError::Capacity) => {
                        assert_eq!(app.runtime.queue.len(), receipts.len() - settled.len());
                    }
                    Err(other) => unreachable!("unexpected observation failure: {other:?}"),
                    Ok(batch) => {
                        assert!(batch.ordered_records().len() <= 4 * budget + 4);
                        assert_eq!(batch.final_ownership(), &before);
                        assert_eq!(batch.final_ownership().pointers(), []);
                        assert!(batch.final_ownership().surfaces().len() <= 1);
                        assert!(batch.report().processed_envelopes() <= budget);
                        if batch.pause_reason().is_some() {
                            assert_eq!(
                                batch.pause_reason(),
                                Some(crate::InputPumpPauseReason::ObservationCapacity)
                            );
                            assert_ne!(batch.report().outcome(), crate::PumpOutcome::Quiescent);
                        }
                        let mut processed = 0;
                        for record in batch.ordered_records() {
                            let InputArbitrationRecord::InputSettled(input) = record else {
                                unreachable!("fixture has no ownership transition or retirement");
                            };
                            assert_eq!(input.scope(), before.scope());
                            assert_eq!(input.ownership_revision(), before.revision());
                            assert!(matches!(input.finality(), UiInputFinality::Committed(facts)
                                if facts.conflict() == UiInputConflict::ObservedNonexclusive));
                            assert_eq!(input.sequence(), receipts[settled.len()]);
                            settled.push(input.sequence());
                            processed += 1;
                        }
                        assert_eq!(processed, batch.report().processed_envelopes());
                        if let Some(fence) = batch.processed_through() {
                            assert!(last_fence.is_none_or(|previous| fence > previous));
                            assert_eq!(Some(&fence), settled.last());
                            last_fence = Some(fence);
                        } else {
                            assert_eq!(processed, 0);
                        }
                        assert_eq!(
                            batch.report().remaining_queued_envelopes(),
                            receipts.len() - settled.len()
                        );
                    }
                }
            }
            assert_eq!(settled, receipts);
            assert_eq!(app.status(), crate::RuntimeStatus::Running);
            assert!(app.trace().len() <= trace_capacity);
        }
    }
}

#[test]
fn terminal_reason_is_not_rewritten_as_shutdown_when_retirement_is_observed_late() {
    let mut app = focused_runtime(false);
    let pending = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("pending keyboard receipt"))
        .sequence();
    let _ = app
        .runtime
        .enter_terminal(crate::RuntimeTerminalReason::Poisoned, 0);
    let batch = app
        .runtime
        .shutdown_observed()
        .unwrap_or_else(|_| unreachable!("terminal and shutdown observations remain available"));
    assert_eq!(
        batch.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    let reasons = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::ScopeRetired(retired) => Some(retired.reason()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        vec![crate::InputScopeRetirementReason::Terminal(
            crate::RuntimeTerminalReason::Poisoned,
        )]
    );
    assert!(!batch.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::InputSettled(settled) if settled.sequence() == pending
    )));
    assert_eq!(
        app.runtime
            .shutdown_observed()
            .unwrap_or_else(|_| unreachable!("shutdown is idempotent"))
            .ordered_records(),
        []
    );
}

struct SpaceApp;

impl UiApp for SpaceApp {
    type State = usize;
    type Action = usize;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        runenui_core::button("Space")
            .id("space")
            .key("space")
            .on_activate(|| 1)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state += action;
    }
}

fn device(value: u64) -> runenui_core::InputDeviceId {
    runenui_core::InputDeviceId::new(value)
        .unwrap_or_else(|| unreachable!("nonzero fixture device"))
}

fn space_key(phase: KeyboardPhase, device_id: runenui_core::InputDeviceId) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        PhysicalKey::Space,
        LogicalKey::Space,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        Some(device_id),
    )
}

#[test]
fn keyboard_default_claim_reasons_distinguish_navigation_and_activation() {
    for (physical, logical, reason) in [
        (
            PhysicalKey::Tab,
            LogicalKey::Tab,
            UiInputClaimReason::FocusNavigation,
        ),
        (
            PhysicalKey::Enter,
            LogicalKey::Enter,
            UiInputClaimReason::ActivationDefault,
        ),
    ] {
        let mut app = AppRuntime::<SpaceApp>::mount(0);
        let budget = PumpBudget::new(32, 32, 32, 32);
        let _ = app
            .pump(budget)
            .unwrap_or_else(|_| unreachable!("mounted button is observable"));
        let id =
            ElementId::new("space").unwrap_or_else(|_| unreachable!("static button authored id"));
        let target = app
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&id))
            .unwrap_or_else(|| unreachable!("button is mounted"))
            .id()
            .clone();
        app.submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("button focus command admitted"));
        let _ = app
            .pump(budget)
            .unwrap_or_else(|_| unreachable!("button focus committed"));
        let receipt = app
            .submit_keyboard(KeyboardEvent::new(
                KeyboardPhase::Down,
                physical,
                logical,
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            ))
            .unwrap_or_else(|_| unreachable!("focused button keyboard admitted"))
            .sequence();
        let batch = app
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one native keyboard boundary observed"));
        assert_eq!(batch.processed_through(), Some(receipt));
        let settled = batch
            .ordered_records()
            .iter()
            .find_map(|record| match record {
                InputArbitrationRecord::InputSettled(entry) => Some(entry),
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("one native keyboard finality"));
        assert_eq!(settled.sequence(), receipt);
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("valid keyboard default commits");
        };
        assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
        assert_eq!(facts.reasons(), [reason]);
        assert_eq!(
            facts.default_disposition(),
            crate::UiDefaultDisposition::Queued
        );
        assert_eq!(
            *app.state(),
            0,
            "keyboard input settlement does not imply descendant action completion"
        );
    }
}

#[test]
fn space_press_ownership_is_source_qualified_through_other_device_and_own_release() {
    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(&ElementId::new("space").unwrap_or_else(|_| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!("space widget mounted"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus accepted"));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let device_a = device(11);
    let device_b = device(12);
    let cases = [
        (KeyboardPhase::Down, device_a, UiInputConflict::ExclusiveUi),
        (
            KeyboardPhase::Down,
            device_b,
            UiInputConflict::ObservedNonexclusive,
        ),
        (KeyboardPhase::Up, device_a, UiInputConflict::ExclusiveUi),
    ];
    for (phase, device_id, expected) in cases {
        let receipt = app
            .submit_keyboard(space_key(phase, device_id))
            .unwrap_or_else(|_| unreachable!("focused Space accepted"))
            .sequence();
        let batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("Space receipt is settled"));
        assert_eq!(batch.processed_through(), Some(receipt));
        let settled = batch
            .ordered_records()
            .iter()
            .find_map(|record| match record {
                InputArbitrationRecord::InputSettled(settled) => Some(settled),
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("one source-qualified receipt"));
        assert_eq!(settled.device_id(), Some(device_id));
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("Space callback commits");
        };
        assert_eq!(facts.conflict(), expected);
        if phase == KeyboardPhase::Down && device_id == device_a {
            assert_eq!(
                facts.default_disposition(),
                crate::UiDefaultDisposition::Applied
            );
        }
        let ownership = app
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("Space snapshot available"));
        assert_eq!(
            ownership.keyboard().space_activation_device_id(),
            if phase == KeyboardPhase::Up {
                None
            } else {
                Some(device_a)
            },
        );
    }
}

#[test]
fn exhausted_direct_publication_preflights_before_commit_and_exposes_last_terminal_revision() {
    use crate::{LogicalSize, PublishSurfaceError, SurfaceBuildContext};
    use runenui_core::StyleEnvironment;

    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(16, 16, 16, 16));
    let original = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("baseline observed"));
    assert_eq!(original.status(), crate::RuntimeStatus::Running);
    app.runtime.seed_input_revision_for_test(u64::MAX - 1);

    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0).unwrap_or_else(|_| unreachable!("finite viewport"));
    let build = SurfaceBuildContext::tight(&environment, size);
    // One publication would consume the sole remaining revision and leave
    // no final revision for invalidation. Reject before publication changes.
    let rejected = app.publish_surface(&build);
    assert!(matches!(
        rejected,
        Err(PublishSurfaceError::Terminal(
            crate::RuntimeTerminalReason::Poisoned
        ))
    ));
    let retired = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("final terminal snapshot is readable"));
    assert_eq!(
        retired.status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(retired.revision().get(), u64::MAX);
    assert!(retired.surfaces()[0].latest_retained_context().is_none());
    assert_eq!(
        app.input_ownership()
            .unwrap_or_else(|_| unreachable!("terminal snapshot remains readable"))
            .revision(),
        retired.revision(),
    );
    let final_batch =
        pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("terminal retirement is still observable"));
    assert!(final_batch.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::ScopeRetired(retired)
            if retired.reason()
                == crate::InputScopeRetirementReason::Terminal(
                    crate::RuntimeTerminalReason::Poisoned
                )
    )));
}

#[test]
fn rev20_unchanged_running_ownership_at_penultimate_revision_remains_observable() {
    use crate::{LogicalSize, SurfaceBuildContext};
    use runenui_core::StyleEnvironment;

    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(16, 16, 16, 16));
    let _ = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("baseline projection"));
    app.runtime.seed_input_revision_for_test(u64::MAX - 2);

    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0).unwrap_or_else(|_| unreachable!("finite viewport"));
    let build = SurfaceBuildContext::tight(&environment, size);
    let _ = app
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("one direct boundary with terminal headroom"));
    let advanced = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("direct publication revision"));
    assert_eq!(advanced.revision().get(), u64::MAX - 1);
    assert_eq!(advanced.status(), crate::RuntimeStatus::Running);

    // No further input/presentation ownership changed. A read cannot
    // consume the sole reserved terminal revision or strand the scope.
    let unchanged = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("unchanged ownership remains readable"));
    assert_eq!(unchanged.revision(), advanced.revision());
    assert_eq!(unchanged.status(), crate::RuntimeStatus::Running);
}

#[test]
fn rev20_pump_exhaustion_retires_scope_without_inventing_input_settlement() {
    use crate::{LogicalSize, SurfaceBuildContext};
    use runenui_core::StyleEnvironment;

    let mut app = focused_runtime(false);
    let _ = app.pump(PumpBudget::new(16, 16, 16, 16));
    let _ = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("mounted snapshot"));
    app.runtime.seed_input_revision_for_test(u64::MAX - 2);
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0).unwrap_or_else(|_| unreachable!("finite viewport"));
    let build = SurfaceBuildContext::tight(&environment, size);
    let _ = app
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("publication admitted"));
    let before = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("penultimate revision"));
    assert_eq!(before.revision().get(), u64::MAX - 1);
    // An admitted but unreached native occurrence must be invalidated by
    // one scope retirement, never fabricated as a processed settlement.
    let pending = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("native occurrence queued before retirement"))
        .sequence();

    let final_batch =
        pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| {
                unreachable!("checked exhaustion retires scope with an owned batch")
            });
    assert_eq!(final_batch.processed_through(), None);
    assert_eq!(final_batch.final_ownership().revision().get(), u64::MAX);
    assert_eq!(
        final_batch.final_ownership().status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned)
    );
    assert!(final_batch.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::ScopeRetired(retired)
            if retired.reason() == crate::InputScopeRetirementReason::Terminal(
                crate::RuntimeTerminalReason::Poisoned
            )
    )));
    assert!(!final_batch.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::InputSettled(settled) if settled.sequence() == pending
    )));
    let repeated = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("retired scope remains observable"));
    assert!(
        !repeated
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
    // Retirement already reached the host on the last valid revision.
    // Finishing cleanup cannot repeat the marker or require MAX+1.
    let closed = app
        .shutdown()
        .unwrap_or_else(|_| unreachable!("retired lifetime still closes"));
    assert_eq!(closed.final_ownership().revision().get(), u64::MAX);
    assert_eq!(
        closed.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    assert_eq!(closed.ordered_records(), []);
}

#[test]
fn rev20_explicit_shutdown_uses_last_revision_without_phantom_boundary() {
    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(16, 16, 16, 16));
    let _ = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("baseline observed"));
    app.runtime.seed_input_revision_for_test(u64::MAX - 1);
    let completed = app
        .runtime
        .shutdown_observed()
        .unwrap_or_else(|_| unreachable!("one remaining revision permits exact close"));
    assert_eq!(completed.final_ownership().revision().get(), u64::MAX);
    assert_eq!(
        completed.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    assert!(
        completed
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
}

#[test]
fn failed_second_shutdown_reservation_preserves_pending_composition_revision() {
    let mut app = AppRuntime::<TextOwnershipApp>::mount(());
    let budget = PumpBudget::new(32, 32, 32, 32);
    let _ = app.pump(budget);
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(&ElementId::new("editor").unwrap_or_else(|_| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!("editable owner is mounted"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus admitted"));
    let _ = app.pump(budget);
    let _ = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("focused owner observed"));
    // Only two revisions remain: pending IME ownership and terminal close.
    // If the failed attempt consumes the former, retry cannot complete.
    app.runtime.seed_input_revision_for_test(u64::MAX - 2);
    let before = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("seeded baseline is readable"));
    assert_eq!(before.revision().get(), u64::MAX - 2);
    let receipt = app
        .start_composition(Some(device(84)))
        .unwrap_or_else(|_| unreachable!("synchronous composition admission"));

    // The first snapshot allocation succeeds; the second fails. Neither
    // revision projection nor irreversible shutdown may have run yet.
    app.runtime.inject_input_reservation_failure_after(1);
    assert!(matches!(
        app.runtime.shutdown_observed(),
        Err(crate::InputObservationError::Capacity)
    ));
    assert_eq!(app.runtime.status(), crate::RuntimeStatus::Running);

    let completed = app
        .runtime
        .shutdown_observed()
        .unwrap_or_else(|_| unreachable!("retry preflights both snapshots"));
    assert_eq!(
        completed.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    assert_eq!(completed.ordered_records().len(), 2);
    let InputArbitrationRecord::OwnershipChanged(transition) = &completed.ordered_records()[0]
    else {
        unreachable!("close publishes its ownership transition");
    };
    assert_eq!(
        transition.before_revision().get(),
        before.revision().get() + 1
    );
    assert_eq!(transition.after().revision().get(), u64::MAX);
    assert!(matches!(
        completed.ordered_records()[1],
        InputArbitrationRecord::ScopeRetired(_)
    ));
    assert!(!completed.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::InputSettled(settled) if settled.sequence() == receipt.sequence()
    )));
}

#[test]
fn already_terminal_max_revision_can_shutdown_and_retire_without_wrap() {
    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(16, 16, 16, 16));
    let _ = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("initial snapshot"));
    app.runtime.seed_input_revision_for_test(u64::MAX - 1);
    let _ = app
        .runtime
        .enter_terminal(crate::RuntimeTerminalReason::Poisoned, 0);
    let terminal = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("last terminal revision is available"));
    assert_eq!(terminal.revision().get(), u64::MAX);
    assert_eq!(
        terminal.status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned)
    );
    // The scope has already reached terminal invalidation at MAX. A
    // subsequent explicit shutdown is real cleanup, not another arbitrable
    // ownership change, and cannot require a nonexistent MAX+1 revision.
    let closed = app
        .shutdown()
        .unwrap_or_else(|_| unreachable!("terminal cleanup needs no further revision"));
    assert_eq!(closed.final_ownership().revision().get(), u64::MAX);
    assert_eq!(
        closed.final_ownership().status(),
        crate::RuntimeStatus::Closed
    );
    assert!(closed.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::ScopeRetired(retired)
            if retired.reason()
                == crate::InputScopeRetirementReason::Terminal(
                    crate::RuntimeTerminalReason::Poisoned
                )
    )));
    assert!(
        !closed
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::OwnershipChanged(_)))
    );
    let repeated = app
        .shutdown()
        .unwrap_or_else(|_| unreachable!("repeated observed close is idempotent"));
    assert!(repeated.report().already_complete());
    assert_eq!(repeated.ordered_records(), []);
    assert_eq!(repeated.final_ownership().revision().get(), u64::MAX);
    let batch = app
        .pump(PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("closed max revision remains readable"));
    assert_eq!(batch.ordered_records(), []);
}

#[test]
fn revision_exhaustion_retains_committed_receipt_and_terminalizes_with_exact_final_snapshot() {
    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(&ElementId::new("space").unwrap_or_else(|_| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!("space widget mounted"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus accepted"));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    app.runtime.seed_input_revision_for_test(u64::MAX - 2);
    let receipt = app
        .submit_keyboard(space_key(KeyboardPhase::Down, device(21)))
        .unwrap_or_else(|_| unreachable!("Space admitted"))
        .sequence();
    let batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("overflow must preserve earlier successful records"));
    assert_eq!(batch.processed_through(), Some(receipt));
    assert_eq!(batch.pause_reason(), None);
    assert_eq!(
        batch.final_ownership().status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned),
    );
    assert_eq!(batch.final_ownership().revision().get(), u64::MAX);
    assert!(
        batch
            .final_ownership()
            .keyboard()
            .space_activation_owner()
            .is_none()
    );
    assert!(matches!(
        batch.ordered_records(),
        [
            InputArbitrationRecord::OwnershipChanged(_),
            InputArbitrationRecord::InputSettled(_),
            InputArbitrationRecord::OwnershipChanged(_),
            InputArbitrationRecord::ScopeRetired(_)
        ]
    ));
    let InputArbitrationRecord::InputSettled(settled) = &batch.ordered_records()[1] else {
        unreachable!("second record is reached keyboard finality");
    };
    assert_eq!(settled.sequence(), receipt);
    assert_eq!(settled.ownership_revision().get(), u64::MAX - 1);
    let InputArbitrationRecord::ScopeRetired(retired) = &batch.ordered_records()[3] else {
        unreachable!("terminal scope retirement concludes records");
    };
    assert_eq!(
        retired.reason(),
        crate::InputScopeRetirementReason::Terminal(crate::RuntimeTerminalReason::Poisoned),
    );
}

#[derive(Debug)]
struct OverflowProbe;

impl Widget<u8> for OverflowProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, u8>,
    ) -> WidgetEventOutput {
        if (matches!(event, UiEvent::Keyboard(_))
            || matches!(event, UiEvent::SemanticCommand(command)
                if command.command() == SemanticCommand::CancelOrBack))
            && context.phase() == EventPhase::Target
        {
            context.claim_host_input();
            context.emit(1);
            // A second output exceeds the admitted one-output transaction.
            // Even the explicit UI claim must not turn an aborted route
            // into a successful exclusive settlement.
            context.emit(2);
        }
        WidgetEventOutput::none()
    }
}

struct OverflowApp;

impl UiApp for OverflowApp {
    type State = usize;
    type Action = u8;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        Element::new(OverflowProbe)
            .id("overflow")
            .key("overflow")
            .focusable(true)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state += usize::from(action);
    }
}

fn assert_overflow_settlement_and_retirement(
    batch: &crate::InputPumpBatch,
    first: runenui_core::WorkSequence,
    pending: runenui_core::WorkSequence,
    scope: &crate::InputArbitrationScope,
) {
    assert_ne!(first, pending);
    assert_eq!(batch.processed_through(), Some(first));
    assert_eq!(
        batch.report().cancelled_by_terminal_transition(),
        1,
        "the queued second native receipt is cancelled by input-callback poison",
    );
    assert_eq!(batch.final_ownership().scope(), scope);
    assert_eq!(
        batch.final_ownership().status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned),
    );

    let settlements: Vec<_> = batch
        .ordered_records()
        .iter()
        .enumerate()
        .filter_map(|(index, record)| match record {
            InputArbitrationRecord::InputSettled(settled) => Some((index, settled)),
            _ => None,
        })
        .collect();
    assert_eq!(settlements.len(), 1);
    let (settlement_index, settled) = settlements[0];
    assert_eq!(settled.sequence(), first);
    assert_eq!(settled.scope(), scope);
    assert_eq!(settled.family(), crate::UiInputFamily::Keyboard);
    assert!(matches!(
        settled.finality(),
        UiInputFinality::Aborted(crate::UiInputAbortReason::RuntimeIntegrity)
    ));

    let retirements: Vec<_> = batch
        .ordered_records()
        .iter()
        .enumerate()
        .filter_map(|(index, record)| match record {
            InputArbitrationRecord::ScopeRetired(retirement) => Some((index, retirement)),
            _ => None,
        })
        .collect();
    assert_eq!(retirements.len(), 1);
    let (retirement_index, retirement) = retirements[0];
    assert!(retirement_index > settlement_index);
    assert_eq!(retirement.scope(), scope);
    assert_eq!(
        retirement.reason(),
        crate::InputScopeRetirementReason::Terminal(crate::RuntimeTerminalReason::Poisoned),
    );
}

fn assert_overflow_aborts_with_trace(trace_config: crate::TraceConfig) {
    let config = crate::RuntimeConfig::default()
        .with_limits(crate::RuntimeLimits::default().with_transaction_outputs(1))
        .with_trace_config(trace_config);
    let mut app = AppRuntime::<OverflowApp>::mount_with_config(0, config);
    let budget = PumpBudget::new(16, 16, 16, 16);
    app.pump(budget)
        .unwrap_or_else(|_| unreachable!("initial readiness must be observable"));
    let id = ElementId::new("overflow").unwrap_or_else(|_| unreachable!("static fixture ID"));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("the overflowing widget must be mounted"))
        .id()
        .clone();
    app.submit_command(
        target.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus request is admitted"));
    app.pump(budget)
        .unwrap_or_else(|_| unreachable!("focus request must be observed"));
    assert_eq!(app.focus().focused_node(), Some(&target));

    let first = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("first keyboard receipt is admitted"))
        .sequence();
    let pending = app
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("second keyboard receipt is admitted"))
        .sequence();
    let scope = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("mounted scope is observable"))
        .scope()
        .clone();

    let batch = app
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("poison must not discard reached finality"));
    assert_eq!(
        *app.state(),
        0,
        "overflow must not commit any staged action"
    );
    assert_overflow_settlement_and_retirement(&batch, first, pending, &scope);
    if trace_config.capacity() == 0 {
        assert_eq!(app.trace().len(), 0);
    }
    let repeated = app
        .pump(budget)
        .unwrap_or_else(|_| unreachable!("terminal observations remain available"));
    assert_eq!(repeated.ordered_records(), []);
    assert_eq!(
        repeated.report().cancelled_by_terminal_transition(),
        0,
        "no terminal cancellation may be reported twice",
    );
}

#[test]
fn overflowing_widget_callback_aborts_reached_input_and_retires_pending_receipts() {
    // Keep the same exact negative oracle for redacted canonical trace and
    // for completely disabled trace. Neither is a gameplay input authority.
    for trace_config in [crate::TraceConfig::default(), crate::TraceConfig::new(0)] {
        assert_overflow_aborts_with_trace(trace_config);
    }
}

#[test]
fn semantic_callback_poison_reports_cancelled_native_input_without_phantom_settlement() {
    for trace_config in [crate::TraceConfig::default(), crate::TraceConfig::new(0)] {
        let config = crate::RuntimeConfig::default()
            .with_limits(crate::RuntimeLimits::default().with_transaction_outputs(1))
            .with_trace_config(trace_config);
        let mut app = AppRuntime::<OverflowApp>::mount_with_config(0, config);
        let budget = PumpBudget::new(32, 32, 32, 32);
        app.pump(budget)
            .unwrap_or_else(|_| unreachable!("initial mount is observable"));
        let authored =
            ElementId::new("overflow").unwrap_or_else(|_| unreachable!("static probe identity"));
        let target = app
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored))
            .unwrap_or_else(|| unreachable!("mounted overflow probe"))
            .id()
            .clone();
        app.submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus admission succeeds"));
        app.pump(budget)
            .unwrap_or_else(|_| unreachable!("focus commits before negative oracle"));

        let failing_command = app
            .submit_command(
                target,
                SemanticCommand::CancelOrBack,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("semantic command enters FIFO"))
            .sequence();
        let pending = app
            .submit_keyboard(key())
            .unwrap_or_else(|_| unreachable!("later native receipt is admitted"))
            .sequence();
        assert!(failing_command < pending);

        let batch = app
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("poisoned semantic transaction is observed"));
        assert_eq!(batch.processed_through(), Some(failing_command));
        assert_eq!(
            batch.report().cancelled_by_terminal_transition(),
            1,
            "the native receipt was erased by the semantic poison transition",
        );
        assert_eq!(
            batch.final_ownership().status(),
            crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned)
        );
        assert_eq!(
            batch
                .ordered_records()
                .iter()
                .filter(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
                .count(),
            1
        );
        assert!(!batch.ordered_records().iter().any(|record| matches!(
            record,
            InputArbitrationRecord::InputSettled(settled) if settled.sequence() == pending
        )));
        assert_eq!(*app.state(), 0);
        let repeated = app
            .pump(budget)
            .unwrap_or_else(|_| unreachable!("terminal observation remains readable"));
        assert_eq!(repeated.report().cancelled_by_terminal_transition(), 0);
        assert_eq!(repeated.ordered_records(), []);
    }
}

#[test]
fn nonterminal_pointer_routed_capacity_refusal_is_processing_rejected_not_aborted() {
    use crate::{LogicalSize, RuntimeConfig, SurfaceBuildContext, TraceConfig};
    use runenui_core::{
        LogicalPoint, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
        PointerPhase, StyleEnvironment,
    };

    // Routed callback preflight reserves the accepted transaction's maximum
    // possible follow-up envelopes. Ingress still fits this smaller FIFO.
    // With trace disabled, arbitration cannot borrow an optional trace result.
    let mut app = AppRuntime::<SpaceApp>::mount_with_config(
        0,
        RuntimeConfig::default()
            .with_queue_capacity(64)
            .with_trace_config(TraceConfig::new(0)),
    );
    app.pump(PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("initial public mount settles"));
    let environment = StyleEnvironment::default();
    let extent =
        LogicalSize::try_new(64.0, 64.0).unwrap_or_else(|_| unreachable!("valid logical extent"));
    let publication = app
        .publish_surface(&SurfaceBuildContext::tight(&environment, extent))
        .unwrap_or_else(|_| unreachable!("retained surface accepted"));
    let bounds = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("mounted button has paint bounds"))
        .bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("button hit point valid"));
    app.pump(PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("publication work settles"));

    let pointer_id =
        PointerId::new(207).unwrap_or_else(|| unreachable!("positive pointer identity"));
    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    let receipt = app
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("host-native Down is queue-admitted"))
        .sequence();
    let batch = app
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pointer preflight refusal is observable"));
    assert_eq!(batch.processed_through(), Some(receipt));
    assert_eq!(
        batch.final_ownership().status(),
        crate::RuntimeStatus::Running
    );
    assert_eq!(batch.final_ownership().pointers(), []);
    assert_eq!(*app.state(), 0);
    let settlements: Vec<_> = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled),
            _ => None,
        })
        .collect();
    assert_eq!(settlements.len(), 1);
    let settled = settlements[0];
    assert_eq!(settled.sequence(), receipt);
    assert_eq!(settled.scope(), batch.final_ownership().scope());
    assert_eq!(settled.family(), crate::UiInputFamily::Pointer);
    assert_eq!(settled.pointer_id(), Some(pointer_id));
    assert!(matches!(
        settled.finality(),
        UiInputFinality::ProcessingRejected(
            crate::UiInputProcessingRejection::InsufficientTransactionCapacity
        )
    ));
    assert!(
        !batch
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
    let repeated = app
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("rejected receipt never replays"));
    assert!(
        !repeated
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
    );
}

#[test]
fn pointer_release_remains_ui_owned_after_press_state_is_cleared() {
    use crate::{LogicalSize, SurfaceBuildContext};
    use runenui_core::{
        LogicalPoint, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
        PointerPhase, StyleEnvironment,
    };

    let mut app = AppRuntime::<SpaceApp>::mount(0);
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0).unwrap_or_else(|_| unreachable!("finite size"));
    let publication = app
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("surface published"));
    let first = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("published root"));
    let bounds = first.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("point in bounds"));
    let context = publication.input_context().clone();
    let id = PointerId::new(51).unwrap_or_else(|| unreachable!("pointer identity"));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));

    let down = PointerEvent::new(
        id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    let down_receipt = app
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("Down admitted"))
        .sequence();
    let down_batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("Down committed"));
    assert_eq!(down_batch.processed_through(), Some(down_receipt));
    let down_facts = down_batch
        .ordered_records()
        .iter()
        .find_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) => Some(settled.finality()),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("Down settled"));
    let UiInputFinality::Committed(down) = down_facts else {
        unreachable!("valid button Down committed");
    };
    assert!(down.reasons().contains(&UiInputClaimReason::PointerPress));

    let up = PointerEvent::new(
        id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        point,
        context,
    )
    .with_changed_button(PointerButton::Primary);
    let up_receipt = app
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("Up admitted"))
        .sequence();
    // Down may have queued a derived action before this native Up. The one
    // canonical FIFO executes that work first; match exact native receipt,
    // never assume the next popped envelope belongs to this submission.
    let up_batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(32, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("bounded canonical FIFO settles Up"));
    let up_facts = up_batch
        .ordered_records()
        .iter()
        .find_map(|record| match record {
            InputArbitrationRecord::InputSettled(settled) if settled.sequence() == up_receipt => {
                Some(settled.finality())
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("exact Up receipt settled"));
    let UiInputFinality::Committed(up) = up_facts else {
        unreachable!("valid button Up committed");
    };
    assert_eq!(up.conflict(), UiInputConflict::ExclusiveUi);
    assert!(up.reasons().contains(&UiInputClaimReason::PointerPress));
}

#[derive(Debug)]
struct TextOwnershipProbe;

impl Widget<()> for TextOwnershipProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn text_input(&self, (): &Self::State) -> runenui_core::WidgetTextInput {
        runenui_core::WidgetTextInput::new(true, true)
    }
}

struct TextOwnershipApp;

impl UiApp for TextOwnershipApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(TextOwnershipProbe)
            .id("editor")
            .key("editor")
            .focusable(true)
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn direct_composition_start_preflights_last_revision_without_binding_a_pending_owner() {
    let mut app = AppRuntime::<TextOwnershipApp>::mount(());
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(&ElementId::new("editor").unwrap_or_else(|_| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!("editor mounted"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus accepted"));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let _ = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("focus owner observed"));
    app.runtime.seed_input_revision_for_test(u64::MAX - 1);
    let start = app.start_composition(Some(device(73)));
    assert!(matches!(
        start,
        Err(error)
            if error.kind()
                == crate::SubmitCompositionErrorKind::Terminal(
                    crate::RuntimeTerminalReason::Poisoned
                )
    ));
    let terminal = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("final terminal owner observed"));
    assert_eq!(terminal.revision().get(), u64::MAX);
    assert_eq!(
        terminal.status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(terminal.keyboard().composition_generation(), None);
    assert_eq!(terminal.keyboard().composition_device_id(), None);
    let batch =
        pump::pump_recorded::<TextOwnershipApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("retirement is observable without consumed input"));
    assert_eq!(batch.processed_through(), None);
    assert!(
        batch
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
}

#[test]
fn accepted_composition_start_publishes_pending_ownership_without_pumping() {
    let mut app = AppRuntime::<TextOwnershipApp>::mount(());
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(&ElementId::new("editor").unwrap_or_else(|_| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!("editor mounted"))
        .id()
        .clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus ingress accepted"));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let before = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("focused owner observable"));
    assert_eq!(before.keyboard().composition_generation(), None);
    assert_eq!(before.keyboard().composition_device_id(), None);
    let receipt = app
        .start_composition(Some(device(42)))
        .unwrap_or_else(|_| unreachable!("composition start admitted"));
    // Admission installs a pending generation synchronously, even though
    // the accepted native receipt has not reached the canonical FIFO head.
    let pending = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("pending owner observed without pump"));
    assert_eq!(
        pending.keyboard().composition_generation(),
        Some(receipt.generation())
    );
    assert_eq!(pending.keyboard().composition_device_id(), Some(device(42)));
    assert!(pending.revision().get() > before.revision().get());
    let unchanged = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("no-op query remains stable"));
    assert_eq!(unchanged.revision(), pending.revision());
    let settled =
        pump::pump_recorded::<TextOwnershipApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("accepted start processes exactly once"));
    assert_eq!(settled.processed_through(), Some(receipt.sequence()));
    assert!(settled.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::InputSettled(entry)
            if entry.sequence() == receipt.sequence()
    )));
}

// The same externally accepted release integrity must survive an evicted
// displayed context for both Up and Cancel, without a fresh hit or activation.
fn prepare_unavailable_context_pressed_pointer(
    phase: runenui_core::PointerPhase,
) -> (
    AppRuntime<SpaceApp>,
    runenui_core::PointerId,
    runenui_core::LogicalPoint,
    runenui_core::SurfaceInputContext,
) {
    use crate::{LogicalSize, RuntimeConfig, SurfaceBuildContext};
    use core::num::NonZeroUsize;
    use runenui_core::{
        LogicalPoint, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
        PointerPhase, StyleEnvironment,
    };
    let retention = NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("positive retention"));
    let mut config = RuntimeConfig::default().with_surface_snapshot_retention(retention);
    if phase == PointerPhase::Cancel {
        config = config.with_trace_config(crate::TraceConfig::new(0));
    }
    let mut app = AppRuntime::<SpaceApp>::mount_with_config(0, config);
    let environment = StyleEnvironment::default();
    let first_size =
        LogicalSize::try_new(64.0, 64.0).unwrap_or_else(|_| unreachable!("finite first size"));
    let first_pub = app
        .publish_surface(&SurfaceBuildContext::tight(&environment, first_size))
        .unwrap_or_else(|_| unreachable!("first surface accepted"));
    let root = first_pub
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("root"));
    let bounds = root.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("point inside root"));
    let old_context = first_pub.input_context().clone();
    let pointer_id = PointerId::new(91).unwrap_or_else(|| unreachable!("positive pointer"));
    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        old_context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    let down_receipt = app
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("Down admitted"))
        .sequence();
    let down_batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("Down settled"));
    assert_eq!(down_batch.processed_through(), Some(down_receipt));
    assert!(down_batch.ordered_records().iter().any(|r| matches!(
        r, InputArbitrationRecord::InputSettled(s) if
            matches!(s.finality(), UiInputFinality::Committed(f)
                if f.reasons().contains(&UiInputClaimReason::PointerPress))
    )));
    let second_size =
        LogicalSize::try_new(96.0, 96.0).unwrap_or_else(|_| unreachable!("finite second size"));
    let newer = app
        .publish_surface(&SurfaceBuildContext::tight(&environment, second_size))
        .unwrap_or_else(|_| unreachable!("republication accepted"));
    assert_ne!(
        old_context.hit_test_generation(),
        newer.input_context().hit_test_generation(),
        "republication must create a new generation to evict the old context"
    );
    (app, pointer_id, point, old_context)
}

fn assert_unavailable_context_pointer_release(phase: runenui_core::PointerPhase) {
    use crate::UiInputRoute;
    use runenui_core::{PointerButton, PointerDeviceKind, PointerEvent, PointerPhase};

    let (mut app, pointer_id, point, old_context) =
        prepare_unavailable_context_pressed_pointer(phase);
    let mut release = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        phase,
        point,
        old_context,
    );
    if phase == PointerPhase::Up {
        release = release.with_changed_button(PointerButton::Primary);
    }
    let release_receipt = app
        .submit_pointer(release)
        .unwrap_or_else(|_| unreachable!("stale-context release admitted"))
        .sequence();
    // Republication may insert stationary rehit/focus work ahead of the
    // native release. Match its exact FIFO receipt, never the next pop.
    let release_batch =
        pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(16, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("integrity cleanup settled"));
    assert!(release_batch.processed_through().is_some());
    let observed = release_batch
        .ordered_records()
        .iter()
        .find_map(|r| match r {
            InputArbitrationRecord::InputSettled(s) if s.sequence() == release_receipt => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("exact reached release settlement"));
    let UiInputFinality::Committed(facts) = observed.finality() else {
        unreachable!("unavailable-context release is committed integrity cleanup");
    };
    assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
    assert!(facts.reasons().contains(&UiInputClaimReason::PointerPress));
    if phase == PointerPhase::Up {
        assert!(matches!(facts.route(), UiInputRoute::Unrouted));
    }
    assert_eq!(observed.scope(), release_batch.final_ownership().scope());
    assert!(
        release_batch
            .final_ownership()
            .pointers()
            .iter()
            .all(|pointer| pointer.pointer_id() != pointer_id)
    );
    if phase == PointerPhase::Cancel {
        assert!(app.runtime.trace().records().next().is_none());
    }
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    assert_eq!(
        *app.state(),
        0,
        "integrity-only release may not activate the widget"
    );
}

#[test]
fn unavailable_context_pointer_up_preserves_exclusive_press_without_activation() {
    assert_unavailable_context_pointer_release(runenui_core::PointerPhase::Up);
}

#[test]
fn unavailable_context_pointer_cancel_preserves_exclusive_press_without_activation() {
    assert_unavailable_context_pointer_release(runenui_core::PointerPhase::Cancel);
}

#[test]
fn observation_capacity_pause_requests_retry_wake_even_with_empty_fifo() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let mut app = focused_runtime(false);
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&calls);
    app.set_wake_transport(move || {
        callback_calls.fetch_add(1, Ordering::SeqCst);
    });
    app.runtime.acknowledge_wake();
    // Initial snapshot reservation, then one zero-work readiness checkpoint.
    // Fail the mandatory terminal checkpoint even though the FIFO is empty.
    app.runtime.inject_input_reservation_failure_after(2);
    let partial = pump::pump_recorded::<ProbeApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("after a checkpoint, capacity produces a partial batch"));
    assert_eq!(
        partial.pause_reason(),
        Some(crate::InputPumpPauseReason::ObservationCapacity)
    );
    assert_eq!(partial.processed_through(), None);
    assert_ne!(partial.report().outcome(), crate::PumpOutcome::Quiescent);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn initial_observation_capacity_error_rearms_wake_without_fifo_work() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let mut app = focused_runtime(false);
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = Arc::clone(&calls);
    app.set_wake_transport(move || {
        callback_calls.fetch_add(1, Ordering::SeqCst);
    });
    app.runtime.acknowledge_wake();
    app.runtime.inject_input_reservation_failure_after(0);

    assert!(matches!(
        app.pump(PumpBudget::new(0, 0, 0, 0)),
        Err(crate::InputObservationError::Capacity)
    ));
    assert_eq!(app.status(), crate::RuntimeStatus::Running);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "initial preflight failure must rearm even without serviceable FIFO work"
    );

    let retried = app
        .pump(PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("one-time capacity fault is recoverable"));
    assert_eq!(retried.ordered_records(), []);
    assert_eq!(retried.pause_reason(), None);
    assert_eq!(
        retried.final_ownership().status(),
        crate::RuntimeStatus::Running
    );
}

#[test]
fn a1_composition_active_space_release_retires_ownership_without_activating() {
    for (active_down, active_up) in [(true, true), (false, true)] {
        let mut app = AppRuntime::<SpaceApp>::mount(0);
        let full = PumpBudget::new(32, 32, 32, 32);
        app.pump(full).unwrap_or_else(|_| unreachable!("mount"));
        let authored = ElementId::new("space").unwrap_or_else(|_| unreachable!());
        let target = app
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored))
            .unwrap_or_else(|| unreachable!("space button"))
            .id()
            .clone();
        app.submit_command(target.clone(), SemanticCommand::RequestFocus, CommandOrigin::programmatic())
            .unwrap_or_else(|_| unreachable!("focus"));
        app.pump(full).unwrap_or_else(|_| unreachable!("focus settles"));
        let device_id = device(97);
        for (phase, active) in [
            (KeyboardPhase::Down, active_down),
            (KeyboardPhase::Up, active_up),
        ] {
            let receipt = app
                .submit_keyboard(KeyboardEvent::new(
                    phase,
                    PhysicalKey::Space,
                    LogicalKey::Space,
                    KeyModifiers::NONE,
                    false,
                    KeyLocation::Standard,
                    if active {
                        KeyboardCompositionState::Active
                    } else {
                        KeyboardCompositionState::Inactive
                    },
                    Some(device_id),
                ))
                .unwrap_or_else(|_| unreachable!("focused Space ingress"))
                .sequence();
            let batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(1, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!("native settlement"));
            assert_eq!(batch.processed_through(), Some(receipt));
            let facts = batch.ordered_records().iter().find_map(|record| match record {
                InputArbitrationRecord::InputSettled(settled) if settled.sequence() == receipt =>
                    Some(settled.finality()),
                _ => None,
            }).unwrap_or_else(|| unreachable!("exact Space receipt"));
            if active {
                let UiInputFinality::Committed(facts) = facts else {
                    unreachable!("active Space input committed")
                };
                assert_eq!(facts.conflict(), UiInputConflict::ObservedNonexclusive);
                assert_eq!(facts.reasons(), []);
                assert_eq!(facts.default_disposition(), crate::UiDefaultDisposition::None);
            }
        }
        app.pump(full).unwrap_or_else(|_| unreachable!("Space cleanup"));
        assert_eq!(*app.state(), 0, "IME Space cannot activate a button");
        // No stale Space owner may activate a later unrelated release.
        app.submit_keyboard(space_key(KeyboardPhase::Up, device_id))
            .unwrap_or_else(|_| unreachable!("subsequent release"));
        app.pump(full).unwrap_or_else(|_| unreachable!("stale release"));
        assert_eq!(*app.state(), 0);
        // Normal keyboard activation still works after an IME-owned release.
        app.submit_keyboard(space_key(KeyboardPhase::Down, device_id))
            .unwrap_or_else(|_| unreachable!("new Down"));
        app.submit_keyboard(space_key(KeyboardPhase::Up, device_id))
            .unwrap_or_else(|_| unreachable!("new Up"));
        app.pump(full).unwrap_or_else(|_| unreachable!("new activation"));
        assert_eq!(*app.state(), 1);
        assert_eq!(app.focus().focused_node(), Some(&target));
    }
}
