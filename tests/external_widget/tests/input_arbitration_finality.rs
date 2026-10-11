//! Independent public-only proof of pending, causal, rejected and aborted input.

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, CompositionEvent, Element, ElementId, EventContext, EventPhase, KeyLocation,
    KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetTextInput, WorkSequence, column, text,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputArbitrationScope, InputPumpBatch,
    InputScopeRetirementReason, MountedNodeId, PumpBudget, RuntimeConfig, RuntimeLimits,
    RuntimeStatus, RuntimeTerminalReason, TraceConfig, TraceRecordKind, UiInputAbortReason,
    UiInputClaimReason, UiInputConflict, UiInputFamily, UiInputFinality,
    UiInputProcessingRejection, UiInputSettlement,
};

const FULL: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

fn trace_bound<App: UiApp>(runtime: &AppRuntime<App>, capacity: usize) {
    match capacity {
        0 => assert_eq!(runtime.trace().len(), 0),
        1 => assert_eq!(runtime.trace().len(), 1, "trace really was truncated"),
        _ => assert!(runtime.trace().len() <= capacity),
    }
}

fn target<App: UiApp>(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!("valid fixture ID"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted fixture"))
        .id()
        .clone()
}

fn focused<App: UiApp>(state: App::State, config: RuntimeConfig) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(state, config);
    runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("initial work"));
    let target = target(&mut runtime, "probe");
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus admission"));
    runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("focus commit"));
    runtime
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

fn settlements(batch: &InputPumpBatch) -> Vec<&UiInputSettlement> {
    batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(value) => Some(value),
            _ => None,
        })
        .collect()
}

fn exact_receipt(
    settled: &UiInputSettlement,
    scope: &InputArbitrationScope,
    sequence: WorkSequence,
    family: UiInputFamily,
) {
    assert_eq!(settled.scope(), scope);
    assert_eq!(settled.sequence(), sequence);
    assert_eq!(settled.family(), family);
    assert_eq!(settled.device_id(), None);
    assert_eq!(settled.pointer_id(), None);
    assert_eq!(settled.surface_id(), None);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompositionObservation {
    Start,
    Update,
    Cancel,
}

#[derive(Debug)]
struct CompositionProbe(Rc<RefCell<Vec<CompositionObservation>>>);

impl Widget<()> for CompositionProbe {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && let UiEvent::Composition(event) = event
        {
            let observation = match event {
                CompositionEvent::Start(_) => Some(CompositionObservation::Start),
                CompositionEvent::Update(_) => Some(CompositionObservation::Update),
                CompositionEvent::Cancel(_) => Some(CompositionObservation::Cancel),
                _ => None,
            };
            if let Some(observation) = observation {
                self.0.borrow_mut().push(observation);
            }
        }
        WidgetEventOutput::none()
    }
}

struct CompositionApp;
impl UiApp for CompositionApp {
    type State = Rc<RefCell<Vec<CompositionObservation>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(observations: &Self::State) -> impl View<()> {
        column(vec![
            Element::new(CompositionProbe(Rc::clone(observations)))
                .id("probe")
                .key("probe")
                .focusable(true),
            Element::new(CompositionProbe(Rc::clone(observations)))
                .id("second")
                .key("second")
                .focusable(true),
        ])
    }
    fn update(_: &mut Self::State, (): ()) {}
}

fn stale_composition_trace(runtime: &AppRuntime<CompositionApp>, sequence: WorkSequence) {
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::CompositionProcessingStaleGeneration
                ))
    );
    assert!(!runtime.trace().export_jsonl().contains("private preedit"));
}

#[test]
fn admitted_composition_update_becomes_stale_without_native_cancellation_or_retarget() {
    for trace_capacity in [0, 1, 512] {
        let observations = Rc::new(RefCell::new(Vec::new()));
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let mut runtime = focused::<CompositionApp>(Rc::clone(&observations), config);
        let second = target(&mut runtime, "second");
        let start = runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("start admission"));
        let pending = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("pending generation"));
        assert_eq!(
            pending.keyboard().composition_generation(),
            Some(start.generation())
        );
        let focus = runtime
            .submit_command(
                second.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("later focus admission"));
        let update = runtime
            .submit_composition_update(
                start.generation().clone(),
                String::from("private preedit"),
                None,
            )
            .unwrap_or_else(|_| unreachable!("pending update admitted before focus changes"));
        assert!(start.sequence() < focus.sequence() && focus.sequence() < update.sequence());
        let batch = runtime
            .pump(PumpBudget::new(3, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("three canonical boundaries"));
        assert_eq!(batch.processed_through(), Some(update.sequence()));
        let settled = settlements(&batch);
        assert_eq!(
            settled.len(),
            2,
            "internal cancellation and focus never create native receipts"
        );
        exact_receipt(
            settled[0],
            pending.scope(),
            start.sequence(),
            UiInputFamily::Composition,
        );
        exact_receipt(
            settled[1],
            pending.scope(),
            update.sequence(),
            UiInputFamily::Composition,
        );
        assert!(
            matches!(settled[0].finality(), UiInputFinality::Committed(facts)
            if facts.conflict() == UiInputConflict::ExclusiveUi && facts.reasons() == [UiInputClaimReason::CompositionOwner])
        );
        assert_eq!(
            settled[1].finality(),
            &UiInputFinality::ProcessingRejected(
                UiInputProcessingRejection::StaleCompositionGeneration
            )
        );
        assert_eq!(
            settled[1].ownership_revision(),
            batch.final_ownership().revision()
        );
        assert_eq!(
            batch.final_ownership().keyboard().composition_generation(),
            None
        );
        assert_eq!(
            batch.final_ownership().keyboard().focused_node(),
            Some(&second)
        );
        assert_eq!(
            *observations.borrow(),
            [
                CompositionObservation::Start,
                CompositionObservation::Cancel
            ]
        );
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        if trace_capacity == 512 {
            stale_composition_trace(&runtime, update.sequence());
        }
        let later = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("no replay"));
        assert_eq!(settlements(&later), Vec::<&UiInputSettlement>::new());
        trace_bound(&runtime, trace_capacity);
    }
}

#[derive(Debug)]
struct ActionProbe {
    overflow: bool,
}
impl Widget<u8> for ActionProbe {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, u8>,
    ) -> WidgetEventOutput {
        if matches!(event, UiEvent::Keyboard(_)) && context.phase() == EventPhase::Target {
            context.emit(1);
            if self.overflow {
                context.claim_host_input();
                context.emit(2);
            }
        }
        WidgetEventOutput::none()
    }
}

struct CausalApp;
impl UiApp for CausalApp {
    type State = usize;
    type Action = u8;
    type HostProtocol = NoHostProtocol;
    fn root(state: &usize) -> impl View<u8> {
        if *state == 0 {
            Element::new(ActionProbe { overflow: false })
                .id("probe")
                .key("probe")
                .focusable(true)
        } else {
            text("application removed the focused widget")
                .id("replacement")
                .key("replacement")
                .into_element()
        }
    }
    fn update(state: &mut usize, action: u8) {
        *state += usize::from(action);
    }
}

#[test]
fn native_settlement_precedes_application_action_and_focus_revocation_across_bounded_pumps() {
    for trace_capacity in [0, 1, 512] {
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let mut runtime = focused::<CausalApp>(0, config);
        let before = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("initial ownership"));
        let receipt = runtime
            .submit_keyboard(key())
            .unwrap_or_else(|_| unreachable!("keyboard admission"));
        let pending = runtime
            .pump(PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("pending checkpoint"));
        assert_eq!(pending.processed_through(), None);
        assert_eq!(settlements(&pending), Vec::<&UiInputSettlement>::new());
        let input = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("input boundary"));
        let settled = settlements(&input);
        assert_eq!(settled.len(), 1);
        exact_receipt(
            settled[0],
            before.scope(),
            receipt.sequence(),
            UiInputFamily::Keyboard,
        );
        assert!(
            matches!(settled[0].finality(), UiInputFinality::Committed(facts)
            if facts.conflict() == UiInputConflict::ObservedNonexclusive && facts.reasons().is_empty())
        );
        assert_eq!(input.processed_through(), Some(receipt.sequence()));
        assert_eq!(input.final_ownership(), &before);
        assert_eq!(
            runtime.state(),
            &0,
            "a settlement does not complete its callback-derived application action"
        );
        assert_eq!(input.report().remaining_queued_envelopes(), 1);
        let action = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("later application action"));
        assert_eq!(runtime.state(), &1);
        assert!(
            action
                .processed_through()
                .is_some_and(|sequence| sequence > receipt.sequence())
        );
        assert!(
            matches!(action.ordered_records(), [InputArbitrationRecord::OwnershipChanged(change)]
            if change.before_revision() == input.final_ownership().revision()
                && change.after().revision() == action.final_ownership().revision()
                && change.after().scope() == before.scope())
        );
        assert_eq!(action.final_ownership().keyboard().focused_node(), None);
        assert!(action.final_ownership().revision() > input.final_ownership().revision());
        assert_eq!(
            input.final_ownership(),
            &before,
            "earlier snapshots remain immutable"
        );
        if trace_capacity == 512 {
            let records = runtime.trace().records().collect::<Vec<_>>();
            let native_commit = records
                .iter()
                .find(|record| {
                    record.work_sequence() == Some(receipt.sequence())
                        && matches!(record.kind(), TraceRecordKind::RoutedEventCommitted)
                })
                .unwrap_or_else(|| unreachable!("native routing diagnostic"));
            let action_start = records
                .iter()
                .find(|record| {
                    record.work_sequence() == action.processed_through()
                        && matches!(
                            record.kind(),
                            TraceRecordKind::ApplicationActionTransactionStarted
                        )
                })
                .unwrap_or_else(|| unreachable!("derived action diagnostic"));
            assert!(native_commit.sequence() < action_start.sequence());
            assert!(action_start.causal_parent().is_some());
        }
        assert_eq!(
            runtime
                .pump(FULL)
                .unwrap_or_else(|_| unreachable!("no replay"))
                .ordered_records(),
            []
        );
        trace_bound(&runtime, trace_capacity);
    }
}

struct OverflowApp;
impl UiApp for OverflowApp {
    type State = usize;
    type Action = u8;
    type HostProtocol = NoHostProtocol;
    fn root(_: &usize) -> impl View<u8> {
        Element::new(ActionProbe { overflow: true })
            .id("probe")
            .key("probe")
            .focusable(true)
    }
    fn update(state: &mut usize, action: u8) {
        *state += usize::from(action);
    }
}

#[test]
fn callback_poison_aborts_only_reached_input_and_retires_pending_receipts_once() {
    for trace_capacity in [0, 1, 512] {
        let config = RuntimeConfig::default()
            .with_limits(RuntimeLimits::default().with_transaction_outputs(1))
            .with_trace_config(TraceConfig::new(trace_capacity));
        let mut runtime = focused::<OverflowApp>(0, config);
        let scope = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("scope"))
            .scope()
            .clone();
        let reached = runtime
            .submit_keyboard(key())
            .unwrap_or_else(|_| unreachable!("first admission"));
        let pending = runtime
            .submit_keyboard(key())
            .unwrap_or_else(|_| unreachable!("second admission"));
        let batch = runtime
            .pump(PumpBudget::new(2, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("poison remains observable"));
        assert_eq!(batch.processed_through(), Some(reached.sequence()));
        assert_eq!(batch.report().processed_envelopes(), 1);
        assert_eq!(batch.report().cancelled_by_terminal_transition(), 1);
        assert_eq!(runtime.state(), &0, "neither provisional action commits");
        assert!(
            matches!(batch.ordered_records(), [InputArbitrationRecord::OwnershipChanged(_), InputArbitrationRecord::InputSettled(input), InputArbitrationRecord::ScopeRetired(retired)]
            if input.sequence() == reached.sequence() && retired.scope() == &scope
                && retired.reason() == InputScopeRetirementReason::Terminal(RuntimeTerminalReason::Poisoned))
        );
        let settled = settlements(&batch);
        assert_eq!(settled.len(), 1);
        exact_receipt(
            settled[0],
            &scope,
            reached.sequence(),
            UiInputFamily::Keyboard,
        );
        assert_eq!(
            settled[0].finality(),
            &UiInputFinality::Aborted(UiInputAbortReason::RuntimeIntegrity)
        );
        assert_eq!(
            settled[0].ownership_revision(),
            batch.final_ownership().revision()
        );
        assert_ne!(settled[0].sequence(), pending.sequence());
        assert_eq!(
            batch.final_ownership().status(),
            RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
        );
        if trace_capacity == 512 {
            assert!(
                runtime
                    .trace()
                    .records()
                    .any(|record| record.work_sequence() == Some(reached.sequence())
                        && matches!(record.kind(), TraceRecordKind::RoutedIntegrityFailed { .. }))
            );
        }
        let repeated = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("repeat observation"));
        assert_eq!(repeated.ordered_records(), []);
        assert_eq!(repeated.report().cancelled_by_terminal_transition(), 0);
        let closed = runtime
            .shutdown()
            .unwrap_or_else(|_| unreachable!("final cleanup"));
        assert_eq!(closed.final_ownership().status(), RuntimeStatus::Closed);
        assert_eq!(
            closed.final_ownership().revision(),
            batch.final_ownership().revision()
        );
        assert_eq!(closed.ordered_records(), []);
        assert_eq!(
            runtime
                .shutdown()
                .unwrap_or_else(|_| unreachable!("idempotent close"))
                .ordered_records(),
            []
        );
        trace_bound(&runtime, trace_capacity);
    }
}
