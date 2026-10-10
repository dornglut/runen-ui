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
fn observed_shutdown_rejects_before_mutation_if_final_revision_is_exhausted() {
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
    assert!(matches!(
        app.runtime.shutdown_observed(),
        Err(crate::InputObservationError::RevisionExhausted)
    ));
    // The rejected close has not hidden a successfully terminalized scope.
    assert_eq!(
        app.runtime.status(),
        crate::RuntimeStatus::Terminal(crate::RuntimeTerminalReason::Poisoned)
    );
    let batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("one scope retirement remains observable"));
    assert!(
        batch
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::ScopeRetired(_)))
    );
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

#[test]
fn unavailable_context_pointer_up_preserves_exclusive_press_without_activation() {
    use crate::{LogicalSize, RuntimeConfig, SurfaceBuildContext, UiInputRoute};
    use core::num::NonZeroUsize;
    use runenui_core::{
        LogicalPoint, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
        PointerPhase, StyleEnvironment,
    };
    let retention = NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("positive retention"));
    let mut app = AppRuntime::<SpaceApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_surface_snapshot_retention(retention),
    );
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
    let up = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        point,
        old_context,
    )
    .with_changed_button(PointerButton::Primary);
    let up_receipt = app
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("stale-context Up admitted"))
        .sequence();
    // Republication may insert stationary rehit/focus work ahead of the native
    // Up. Correlate by the exact receipt rather than assuming the next pop.
    let up_batch = pump::pump_recorded::<SpaceApp>(&mut app.runtime, PumpBudget::new(16, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("integrity cleanup settled"));
    assert!(up_batch.processed_through().is_some());
    let observed = up_batch
        .ordered_records()
        .iter()
        .find_map(|r| match r {
            InputArbitrationRecord::InputSettled(s) if s.sequence() == up_receipt => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("exact reached Up settlement"));
    let UiInputFinality::Committed(facts) = observed.finality() else {
        unreachable!("unavailable-context Up is committed integrity cleanup");
    };
    assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
    assert!(facts.reasons().contains(&UiInputClaimReason::PointerPress));
    assert!(matches!(facts.route(), UiInputRoute::Unrouted));
    let _ = app.pump(PumpBudget::new(32, 32, 32, 32));
    assert_eq!(
        *app.state(),
        0,
        "integrity-only release may not activate the widget"
    );
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
