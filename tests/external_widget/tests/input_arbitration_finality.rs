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
    RuntimeStatus, RuntimeTerminalReason, TraceConfig, TraceRecordKind, UiFocusRetirementKind,
    UiInputAbortReason, UiInputClaimReason, UiInputConflict, UiInputFamily, UiInputFinality,
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
    let mut runtime = AppRuntime::<App>::mount_with_config(state, config)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
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
        .unwrap_or_else(|error| unreachable!("focus admission: {:?}", error.kind()));
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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

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
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 0)
            .with_reconciliation(3, 0, 3)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Source and lifetime continuation coexist.
        runenui_core::ProducerBudget::new(2, 0)
    }

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

#[test]
fn public_host_pair_has_two_ordered_exact_input_receipts_with_trace_on_or_off() {
    use runenui_core::CommittedTextEvent;

    for trace_capacity in [0, 512] {
        let config =
            RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let observations = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = focused::<CompositionApp>(observations, config);
        let before = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("original focused input scope"));
        let scope = before.scope().clone();

        let (keyboard, text) = runtime
            .submit_keyboard_with_text(
                key(),
                CommittedTextEvent::new("é", None)
                    .unwrap_or_else(|_| unreachable!("original committed Unicode text")),
            )
            .unwrap_or_else(|error| unreachable!("public paired ingress: {error}"));
        assert!(
            keyboard.sequence() < text.sequence(),
            "both original members must get ordered canonical work identities"
        );
        runtime
            .submit_action(())
            .unwrap_or_else(|_| unreachable!("later independent application work"));
        let batch = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("one canonical public pump"));
        let emitted = settlements(&batch);
        let keyboard_receipts: Vec<_> = emitted
            .iter()
            .filter(|record| record.sequence() == keyboard.sequence())
            .collect();
        let text_receipts: Vec<_> = emitted
            .iter()
            .filter(|record| record.sequence() == text.sequence())
            .collect();
        assert_eq!(keyboard_receipts.len(), 1);
        assert_eq!(text_receipts.len(), 1);
        exact_receipt(
            keyboard_receipts[0],
            &scope,
            keyboard.sequence(),
            UiInputFamily::Keyboard,
        );
        exact_receipt(
            text_receipts[0],
            &scope,
            text.sequence(),
            UiInputFamily::CommittedText,
        );
        assert!(matches!(
            text_receipts[0].finality(),
            UiInputFinality::Committed(_)
        ));
        assert_eq!(
            batch.processed_through().is_some_and(|last| last >= text.sequence()),
            true,
            "an exact text terminal receipt cannot be inferred from keyboard alone"
        );
        let later = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("later pending-work observation"));
        assert!(
            !settlements(&later).iter().any(|record| {
                record.sequence() == keyboard.sequence() || record.sequence() == text.sequence()
            }),
            "canonical M13 input finality is once-only with and without trace"
        );
        trace_bound(&runtime, trace_capacity);
    }
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
            .unwrap_or_else(|error| unreachable!("later focus admission: {:?}", error.kind()));
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
    event_budget: usize,
}
impl Widget<u8> for ActionProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event => {
                runenui_core::WidgetProducerBudget::new(self.event_budget)
            }
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();
    fn create_state(&self) -> Self::State {}
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, u8>,
    ) -> WidgetEventOutput {
        if matches!(event, UiEvent::Keyboard(_)) && context.phase() == EventPhase::Target {
            let _ = context.try_emit(1);
            if self.overflow {
                context.claim_host_input();
                let _ = context.try_emit(2);
            }
        }
        WidgetEventOutput::none()
    }
}

struct CausalApp;
impl UiApp for CausalApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(10, 10).with_reconciliation(1, 0, 1)
    }

    type State = usize;
    type Action = u8;
    type HostProtocol = NoHostProtocol;
    fn root(state: &usize) -> impl View<u8> {
        if *state == 0 {
            Element::new(ActionProbe {
                overflow: false,
                event_budget: 2,
            })
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

fn causal_action_trace(
    runtime: &AppRuntime<CausalApp>,
    native: WorkSequence,
    action: WorkSequence,
) {
    let records = runtime.trace().records().collect::<Vec<_>>();
    let native_commit = records
        .iter()
        .find(|record| {
            record.work_sequence() == Some(native)
                && matches!(record.kind(), TraceRecordKind::RoutedEventCommitted)
        })
        .unwrap_or_else(|| unreachable!("native routing diagnostic"));
    let action_start = records
        .iter()
        .find(|record| {
            record.work_sequence() == Some(action)
                && matches!(
                    record.kind(),
                    TraceRecordKind::ApplicationActionTransactionStarted
                )
        })
        .unwrap_or_else(|| unreachable!("derived action diagnostic"));
    assert!(native_commit.sequence() < action_start.sequence());
    let accepted = records
        .iter()
        .find(|record| Some(record.sequence()) == action_start.causal_parent())
        .unwrap_or_else(|| unreachable!("exact action-admission causal parent"));
    assert!(matches!(
        accepted.kind(),
        TraceRecordKind::ActionSubmissionAccepted
    ));
    assert_eq!(accepted.work_sequence(), Some(action));
    let collected = records
        .iter()
        .find(|record| Some(record.sequence()) == accepted.causal_parent())
        .unwrap_or_else(|| unreachable!("exact native action-collection causal parent"));
    assert!(matches!(
        collected.kind(),
        TraceRecordKind::RoutedActionCollected
    ));
    assert_eq!(collected.work_sequence(), Some(native));
}

#[test]
fn exact_focus_retirement_generation_does_not_alias_aba_refocus() {
    for trace_capacity in [0, 512] {
        let state = Rc::new(RefCell::new(Vec::new()));
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let mut runtime = focused::<CompositionApp>(state, config);
        let first = target(&mut runtime, "probe");
        let second = target(&mut runtime, "second");
        let original_focus_generation = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("exact initial focus lifetime"))
            .keyboard()
            .focus_generation()
            .unwrap_or_else(|| unreachable!("focused widget has a generation"));
        let scope = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("initial scope"))
            .scope()
            .clone();

        let to_second = runtime
            .submit_command(
                second.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("second focus admitted"));
        let outgoing_first = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("second focus committed"));
        let first_retirement = outgoing_first
            .ordered_records()
            .iter()
            .find_map(|record| match record {
                InputArbitrationRecord::FocusRecipientRetired(value) => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("old focus callback delivered"));
        assert_eq!(first_retirement.scope(), &scope);
        assert_eq!(first_retirement.recipient(), &first);
        assert_eq!(
            first_retirement.focus_generation(),
            original_focus_generation
        );
        assert_eq!(first_retirement.sequence(), to_second.sequence());
        assert_eq!(
            first_retirement.kind(),
            UiFocusRetirementKind::DeliveredFocusOut
        );
        assert_eq!(
            outgoing_first.final_ownership().keyboard().focused_node(),
            Some(&second)
        );

        let to_first = runtime
            .submit_command(
                first.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("return focus admitted"));
        let outgoing_second = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("return focus committed"));
        let second_retirement = outgoing_second
            .ordered_records()
            .iter()
            .find_map(|record| match record {
                InputArbitrationRecord::FocusRecipientRetired(value) => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("second focus callback delivered"));
        assert_eq!(second_retirement.scope(), &scope);
        assert_eq!(second_retirement.recipient(), &second);
        assert_eq!(second_retirement.sequence(), to_first.sequence());
        assert_eq!(
            second_retirement.kind(),
            UiFocusRetirementKind::DeliveredFocusOut
        );
        assert!(second_retirement.focus_generation() > first_retirement.focus_generation());
        assert!(second_retirement.sequence() > first_retirement.sequence());
        assert_ne!(
            outgoing_second
                .final_ownership()
                .keyboard()
                .focus_generation(),
            Some(original_focus_generation),
            "focus A->B->A must never reuse A's original lifetime"
        );
        assert_eq!(
            outgoing_second.final_ownership().keyboard().focused_node(),
            Some(&first)
        );
        assert!(
            !outgoing_second
                .ordered_records()
                .iter()
                .any(|record| matches!(
                    record,
                    InputArbitrationRecord::FocusRecipientRetired(value)
                        if value.recipient() == &first
                ))
        );
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
        assert!(matches!(action.ordered_records(), [
                InputArbitrationRecord::OwnershipChanged(change),
                InputArbitrationRecord::FocusRecipientRetired(retired)
            ]
            if change.before_revision() == input.final_ownership().revision()
                && change.after().revision() == action.final_ownership().revision()
                && change.after().scope() == before.scope()
                && retired.scope() == before.scope()
                && Some(retired.recipient()) == before.keyboard().focused_node()
                && retired.kind() == UiFocusRetirementKind::Unmounted
                && Some(retired.focus_generation()) == before.keyboard().focus_generation()
                && Some(retired.sequence()) == action.processed_through()));
        assert_eq!(action.final_ownership().keyboard().focused_node(), None);
        assert!(action.final_ownership().revision() > input.final_ownership().revision());
        let no_replay = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("a repeated observation succeeds"));
        assert!(
            !no_replay
                .ordered_records()
                .iter()
                .any(|record| matches!(record, InputArbitrationRecord::FocusRecipientRetired(_)))
        );
        assert_eq!(
            input.final_ownership(),
            &before,
            "earlier snapshots remain immutable"
        );
        if trace_capacity == 512 {
            causal_action_trace(
                &runtime,
                receipt.sequence(),
                action
                    .processed_through()
                    .unwrap_or_else(|| unreachable!("action was processed")),
            );
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

const fn callback_overrun_reason() -> RuntimeTerminalReason {
    RuntimeTerminalReason::ProducerContractViolation(runenui_core::ProducerContractViolation {
        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
        phase: runenui_core::ProducerPhase::Widget(runenui_core::WidgetProducerPhase::Event),
        component: runenui_core::BudgetComponent::Outputs,
        declared: 1,
        attempted: Some(2),
    })
}

struct OverflowApp;
impl UiApp for OverflowApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        // Focus is admitted while this widget has no event output. A bounded
        // app action then raises its declared event allowance to one; the
        // keyboard callback deliberately emits two to exercise fail-closed
        // overrun handling under the one-output transaction limit.
        runenui_core::ProducerBudget::new(1, usize::from(*state != 0))
    }

    type State = usize;
    type Action = u8;
    type HostProtocol = NoHostProtocol;
    fn root(state: &usize) -> impl View<u8> {
        Element::new(ActionProbe {
            overflow: *state != 0,
            event_budget: usize::from(*state != 0),
        })
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
        runtime.submit_action(1).unwrap_or_else(|_| {
            unreachable!("bounded action enables the declared-overrun callback")
        });
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("fixture event profile transition"));
        assert_eq!(
            runtime.state(),
            &1,
            "overrun behavior is enabled before the poisoned input"
        );
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
        assert_eq!(
            runtime.state(),
            &1,
            "the overrun input action does not commit"
        );
        assert!(
            matches!(batch.ordered_records(), [InputArbitrationRecord::OwnershipChanged(_), InputArbitrationRecord::InputSettled(input), InputArbitrationRecord::ScopeRetired(retired)]
                        if input.sequence() == reached.sequence() && retired.scope() == &scope
                            && retired.reason() == InputScopeRetirementReason::Terminal(callback_overrun_reason()))
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
            RuntimeStatus::Terminal(callback_overrun_reason())
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
