#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Element, EventContext, HitContribution, HitContributionContext, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent,
    PointerId, PointerPhase, StyleTokens, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    RuntimeTerminalReason, SubmitPointerErrorKind, SurfaceBuildContext, TraceConfig,
    TracePointerRejection, TraceRecord, TraceRecordKind,
};

#[derive(Clone)]
struct State {
    callbacks: Rc<Cell<usize>>,
    activations: Rc<Cell<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Noop,
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            callbacks: Rc::clone(&state.callbacks),
        })
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if action == Action::Activated {
            state.activations.set(state.activations.get() + 1);
        }
    }
}

#[derive(Debug)]
struct Probe {
    callbacks: Rc<Cell<usize>>,
}

impl Widget<Action> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        _context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if matches!(event, UiEvent::Pointer(_)) {
            self.callbacks.set(self.callbacks.get() + 1);
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _state: &mut Self::State,
        _context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Activated)
    }

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(32.0).unwrap_or_default(),
            height: LogicalLength::new(32.0).unwrap_or_default(),
        }
    }

    fn hit_test(&self, _state: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rectangle"));
        HitContribution::single_rect(rect)
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    context: runenui_core::SurfaceInputContext,
    point: LogicalPoint,
    callbacks: Rc<Cell<usize>>,
    activations: Rc<Cell<usize>>,
}

fn harness(config: RuntimeConfig) -> Harness {
    let callbacks = Rc::new(Cell::new(0));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            callbacks: Rc::clone(&callbacks),
            activations: Rc::clone(&activations),
        },
        config,
    );
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&tokens, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"));
    let bounds = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the root is published"))
        .bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        point,
        callbacks,
        activations,
    }
}

fn pointer_event(
    harness: &Harness,
    pointer_id: u64,
    phase: PointerPhase,
    primary_pressed: bool,
) -> PointerEvent {
    let buttons = if primary_pressed {
        PointerButtons::new([PointerButton::Primary])
    } else {
        PointerButtons::default()
    };
    let event = PointerEvent::new(
        PointerId::new(pointer_id).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        phase,
        harness.point,
        harness.context.clone(),
    )
    .with_buttons(buttons);
    match phase {
        PointerPhase::Down | PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
        _ => event,
    }
}

fn pump_all(runtime: &mut AppRuntime<App>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn assert_causal_ancestor(
    records: &[&TraceRecord],
    descendant: &TraceRecord,
    ancestor: &TraceRecord,
) {
    let mut parent = descendant.causal_parent();
    while parent != Some(ancestor.sequence()) {
        let sequence = parent
            .unwrap_or_else(|| unreachable!("descendant must retain the expected causal ancestor"));
        parent = records
            .iter()
            .copied()
            .find(|record| record.sequence() == sequence)
            .unwrap_or_else(|| unreachable!("every retained causal parent is present"))
            .causal_parent();
    }
}

#[test]
fn queue_full_rejection_recovers_event_and_consumes_no_sequence_or_trace() {
    let mut harness = harness(RuntimeConfig::default().with_queue_capacity(1));
    pump_all(&mut harness.runtime);
    let (Some(expected_first_sequence), _) = harness.runtime.__routed_sequence_state_for_test()
    else {
        unreachable!("the drained runtime retains work-sequence authority");
    };
    let first_sequence = harness
        .runtime
        .submit_action(Action::Noop)
        .unwrap_or_else(|_| unreachable!("the drained single queue slot is available"));
    assert_eq!(first_sequence.get(), expected_first_sequence);
    let event = pointer_event(&harness, 101, PointerPhase::Move, false);
    let trace_len = harness.runtime.trace().len();

    let Err(error) = harness.runtime.submit_pointer(event.clone()) else {
        unreachable!("the occupied queue rejects pointer submission");
    };
    assert_eq!(error.kind(), SubmitPointerErrorKind::Full);
    assert_eq!(error.into_event(), event);
    assert_eq!(harness.runtime.trace().len(), trace_len);
    assert_eq!(harness.runtime.focus().modality(), None);

    pump_all(&mut harness.runtime);
    let accepted = harness
        .runtime
        .submit_pointer(pointer_event(&harness, 101, PointerPhase::Move, false))
        .unwrap_or_else(|_| unreachable!("the queue is available after pumping"));
    assert_eq!(
        accepted.sequence().get(),
        expected_first_sequence
            .checked_add(1)
            .unwrap_or_else(|| unreachable!("the test sequence is not exhausted")),
    );
}

#[test]
fn closed_and_terminal_rejections_recover_the_exact_event() {
    let mut closed = harness(RuntimeConfig::default());
    closed.runtime.shutdown();
    let closed_event = pointer_event(&closed, 102, PointerPhase::Move, false);
    let closed_trace_len = closed.runtime.trace().len();
    let Err(closed_error) = closed.runtime.submit_pointer(closed_event.clone()) else {
        unreachable!("closed runtime rejects pointer submission");
    };
    assert_eq!(closed_error.kind(), SubmitPointerErrorKind::Closed);
    assert_eq!(closed_error.into_event(), closed_event);
    assert_eq!(closed.runtime.trace().len(), closed_trace_len);

    let mut terminal = harness(RuntimeConfig::default());
    terminal.runtime.__fail_routed_commit_for_test();
    terminal
        .runtime
        .submit_pointer(pointer_event(&terminal, 103, PointerPhase::Down, true))
        .unwrap_or_else(|_| unreachable!("the first event is accepted before poisoning"));
    pump_all(&mut terminal.runtime);
    assert_eq!(
        terminal.runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    let terminal_event = pointer_event(&terminal, 104, PointerPhase::Move, false);
    let terminal_trace_len = terminal.runtime.trace().len();
    let Err(terminal_error) = terminal.runtime.submit_pointer(terminal_event.clone()) else {
        unreachable!("terminal runtime rejects pointer submission");
    };
    assert_eq!(
        terminal_error.kind(),
        SubmitPointerErrorKind::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(terminal_error.into_event(), terminal_event);
    assert_eq!(terminal.runtime.trace().len(), terminal_trace_len);
}

#[test]
fn sequence_exhaustion_rejections_consume_no_other_authority() {
    let mut work_exhausted = harness(RuntimeConfig::default());
    pump_all(&mut work_exhausted.runtime);
    work_exhausted.runtime.__seed_next_work_sequence_for_test(0);
    let work_event = pointer_event(&work_exhausted, 105, PointerPhase::Move, false);
    let work_trace_len = work_exhausted.runtime.trace().len();
    let Err(work_error) = work_exhausted.runtime.submit_pointer(work_event.clone()) else {
        unreachable!("missing next work sequence rejects submission");
    };
    assert_eq!(
        work_error.kind(),
        SubmitPointerErrorKind::WorkSequenceExhausted
    );
    assert_eq!(work_error.into_event(), work_event);
    assert_eq!(work_exhausted.runtime.trace().len(), work_trace_len);

    let mut trace_exhausted = harness(RuntimeConfig::default());
    pump_all(&mut trace_exhausted.runtime);
    let (Some(expected_work_sequence), Some(next_trace_sequence)) =
        trace_exhausted.runtime.__routed_sequence_state_for_test()
    else {
        unreachable!("the drained runtime retains work and trace sequence authority");
    };
    trace_exhausted
        .runtime
        .__seed_next_trace_sequence_for_test(0);
    let trace_event = pointer_event(&trace_exhausted, 106, PointerPhase::Move, false);
    let trace_len = trace_exhausted.runtime.trace().len();
    let Err(trace_error) = trace_exhausted.runtime.submit_pointer(trace_event.clone()) else {
        unreachable!("missing next trace sequence rejects submission");
    };
    assert_eq!(
        trace_error.kind(),
        SubmitPointerErrorKind::TraceSequenceExhausted
    );
    assert_eq!(trace_error.into_event(), trace_event);
    assert_eq!(trace_exhausted.runtime.trace().len(), trace_len);
    trace_exhausted
        .runtime
        .__seed_next_trace_sequence_for_test(next_trace_sequence);
    let accepted = trace_exhausted
        .runtime
        .submit_pointer(pointer_event(
            &trace_exhausted,
            106,
            PointerPhase::Move,
            false,
        ))
        .unwrap_or_else(|_| unreachable!("work sequence was not consumed by trace rejection"));
    assert_eq!(accepted.sequence().get(), expected_work_sequence);
}

#[test]
fn pointer_registry_saturation_rejects_before_callback_or_state_commit() {
    let limits = RuntimeLimits::default().with_pointer_streams(0);
    let mut harness = harness(RuntimeConfig::default().with_limits(limits));
    harness
        .runtime
        .submit_pointer(pointer_event(&harness, 107, PointerPhase::Move, false))
        .unwrap_or_else(|_| unreachable!("submission precedes processing saturation"));
    pump_all(&mut harness.runtime);

    assert_eq!(harness.callbacks.get(), 0);
    assert_eq!(harness.activations.get(), 0);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Move,
            outcome: TracePointerRejection::RegistryFull,
        } if pointer_id.get() == 107
    )));
    assert!(!harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
            if pointer_id.get() == 107
    )));
    assert_eq!(harness.runtime.status(), RuntimeStatus::Running);
}

#[test]
fn repeated_button_down_uses_the_single_transition_mismatch_outcome() {
    let mut harness = harness(RuntimeConfig::default());
    harness
        .runtime
        .submit_pointer(pointer_event(&harness, 109, PointerPhase::Down, true))
        .unwrap_or_else(|_| unreachable!("initial primary down is accepted"));
    pump_all(&mut harness.runtime);
    assert_eq!(harness.callbacks.get(), 1);
    let start = harness.runtime.trace().len();

    harness
        .runtime
        .submit_pointer(pointer_event(&harness, 109, PointerPhase::Down, true))
        .unwrap_or_else(|_| unreachable!("repeated down is accepted before processing"));
    pump_all(&mut harness.runtime);

    assert_eq!(harness.callbacks.get(), 1);
    let records = harness
        .runtime
        .trace()
        .records()
        .skip(start)
        .collect::<Vec<_>>();
    assert!(records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Down,
            outcome: TracePointerRejection::ButtonTransitionMismatch,
        } if pointer_id.get() == 109
    )));
    assert!(!records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Down,
            outcome: TracePointerRejection::DuplicateStream,
        } if pointer_id.get() == 109
    )));
    assert!(!records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 109
    )));
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
                    if pointer_id.get() == 109
            ))
            .count(),
        1
    );
}

#[test]
fn primary_partial_release_records_exact_cleanup_before_capture_loss() {
    let mut harness = harness(RuntimeConfig::default());
    harness
        .runtime
        .submit_pointer(pointer_event(&harness, 110, PointerPhase::Down, true))
        .unwrap_or_else(|_| unreachable!("initial primary down is accepted"));
    pump_all(&mut harness.runtime);

    let pointer_id = PointerId::new(110).unwrap_or_else(|| unreachable!("pointer id is non-zero"));
    let secondary_down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        harness.point,
        harness.context.clone(),
    )
    .with_buttons(PointerButtons::new([
        PointerButton::Primary,
        PointerButton::Secondary,
    ]))
    .with_changed_button(PointerButton::Secondary);
    harness
        .runtime
        .submit_pointer(secondary_down)
        .unwrap_or_else(|_| unreachable!("secondary chord down is accepted"));
    pump_all(&mut harness.runtime);

    let primary_up = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        harness.point,
        harness.context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Secondary]))
    .with_changed_button(PointerButton::Primary);
    let accepted = harness
        .runtime
        .submit_pointer(primary_up)
        .unwrap_or_else(|_| unreachable!("primary partial release is accepted"));
    let sequence = accepted.sequence();
    pump_all(&mut harness.runtime);

    let records = harness.runtime.trace().records().collect::<Vec<_>>();
    let cleanup_record = records
        .iter()
        .copied()
        .find(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerIntegrityCleanupCommitted
                )
        })
        .unwrap_or_else(|| unreachable!("primary release records exact pointer cleanup"));
    let pointer = cleanup_record
        .context()
        .pointer()
        .unwrap_or_else(|| unreachable!("cleanup retains pointer context"));
    assert_eq!(pointer.pointer_id(), &pointer_id);
    assert_eq!(pointer.phase(), Some(PointerPhase::Up));
    let cleanup = cleanup_record
        .context()
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("cleanup record retains exact owner transitions"));
    let pressed = cleanup
        .pressed_owner()
        .unwrap_or_else(|| unreachable!("primary release clears pressed ownership"));
    let capture = cleanup
        .capture_owner()
        .unwrap_or_else(|| unreachable!("primary release clears capture ownership"));
    assert!(pressed.previous().is_some());
    assert_eq!(pressed.current(), None);
    assert_eq!(capture.current(), None);
    assert_eq!(
        pressed
            .previous()
            .map(runenui_runtime::TraceTarget::mounted_node_id),
        capture
            .previous()
            .map(runenui_runtime::TraceTarget::mounted_node_id)
    );

    let capture_loss = records
        .iter()
        .copied()
        .find(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerCaptureNotificationResolved {
                        kind: runenui_core::PointerCaptureKind::Lost,
                    }
                )
        })
        .unwrap_or_else(|| unreachable!("primary release records capture loss"));
    assert_causal_ancestor(&records, capture_loss, cleanup_record);
    assert!(!records.iter().any(|record| {
        record.work_sequence() == Some(sequence)
            && matches!(record.kind(), TraceRecordKind::PointerStreamClosed { .. })
    }));
}

#[test]
fn trace_capacity_zero_preserves_pointer_behavior() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(0));
    let mut harness = harness(config);
    harness
        .runtime
        .submit_pointer(pointer_event(&harness, 108, PointerPhase::Down, true))
        .unwrap_or_else(|_| unreachable!("trace-disabled down is accepted"));
    pump_all(&mut harness.runtime);
    harness
        .runtime
        .submit_pointer(pointer_event(&harness, 108, PointerPhase::Up, false))
        .unwrap_or_else(|_| unreachable!("trace-disabled up is accepted"));
    pump_all(&mut harness.runtime);

    assert_eq!(harness.callbacks.get(), 2);
    assert_eq!(harness.activations.get(), 1);
    assert_eq!(harness.runtime.trace().len(), 0);
}
