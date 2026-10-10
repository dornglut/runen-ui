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
            if phase == KeyboardPhase::Up { None } else { Some(device_a) },
        );
    }
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
    let receipt = app
        .start_composition(None)
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
