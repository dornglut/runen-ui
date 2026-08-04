#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, EventContext, LogicalDelta, LogicalLength, LogicalPoint, NoHostProtocol,
    PointerButton, PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, StyleTokens, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
    WorkSequence,
};
use runenui_runtime::{
    AppRuntime, FocusReason, InputModality, LogicalSize, PumpBudget, SurfaceBuildContext,
    TraceEventFamily, TraceRecord, TraceRecordKind, TraceSurfaceSnapshotKind, TraceTarget,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct PointerObservation {
    pointer_id: PointerId,
    phase: PointerPhase,
    physical_target: Option<runenui_core::MountedNodeId>,
    physical_path: Vec<runenui_core::MountedNodeId>,
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<PointerObservation>>>,
    activations: Rc<Cell<usize>>,
    prevent_up: bool,
    prevent_wheel: bool,
}

#[derive(Debug)]
enum Action {
    Observed(PointerObservation),
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(ProbeWidget {
            prevent_up: state.prevent_up,
            prevent_wheel: state.prevent_wheel,
        })
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Observed(observation) => state.observations.borrow_mut().push(observation),
            Action::Activated => state.activations.set(state.activations.get() + 1),
        }
    }
}

#[derive(Debug)]
struct ProbeWidget {
    prevent_up: bool,
    prevent_wheel: bool,
}

impl Widget<Action> for ProbeWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let UiEvent::Pointer(pointer) = event {
            context.emit(Action::Observed(PointerObservation {
                pointer_id: context
                    .pointer_id()
                    .unwrap_or_else(|| unreachable!("pointer callbacks carry an identity")),
                phase: pointer.phase(),
                physical_target: context.physical_target().cloned(),
                physical_path: context.physical_path().to_vec(),
            }));
            if (self.prevent_up && pointer.phase() == PointerPhase::Up)
                || (self.prevent_wheel && pointer.phase() == PointerPhase::Wheel)
            {
                context.prevent_default();
            }
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
}

struct Harness {
    runtime: AppRuntime<App>,
    context: runenui_core::SurfaceInputContext,
    target: runenui_core::MountedNodeId,
    point: LogicalPoint,
    observations: Rc<RefCell<Vec<PointerObservation>>>,
    activations: Rc<Cell<usize>>,
}

fn harness(prevent_up: bool, prevent_wheel: bool) -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
        activations: Rc::clone(&activations),
        prevent_up,
        prevent_wheel,
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let node = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the root is published"));
    let target = node.id().clone();
    let bounds = node.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        target,
        point,
        observations,
        activations,
    }
}

fn pointer_event(
    harness: &Harness,
    pointer_id: u64,
    phase: PointerPhase,
    changed_button: Option<PointerButton>,
    primary_pressed: bool,
    scroll_delta: LogicalDelta,
) -> PointerEvent {
    let pointer_id = PointerId::new(pointer_id)
        .unwrap_or_else(|| unreachable!("test pointer identities are non-zero"));
    let buttons = if primary_pressed {
        PointerButtons::new([PointerButton::Primary])
    } else {
        PointerButtons::default()
    };
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        phase,
        harness.point,
        harness.context.clone(),
    )
    .with_buttons(buttons)
    .with_scroll_delta(scroll_delta);
    match changed_button {
        Some(button) => event.with_changed_button(button),
        None => event,
    }
}

fn pump_all(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

fn assert_pointer_focus(harness: &Harness) {
    assert_eq!(
        harness.runtime.focus().focused_node(),
        Some(&harness.target)
    );
    assert_eq!(harness.runtime.focus().reason(), Some(FocusReason::Pointer));
    assert_eq!(
        harness.runtime.focus().modality(),
        Some(InputModality::Pointer)
    );
}

fn mandatory_pointer_record<'a>(
    records: &[&'a TraceRecord],
    work_sequence: WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| record.work_sequence() == Some(work_sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("the mandatory pointer trace fact is retained"))
}

fn assert_physical_pointer_observation(physical: &TraceRecord, harness: &Harness) {
    let event = physical
        .context()
        .event()
        .unwrap_or_else(|| unreachable!("physical observation owns its event context"));
    assert_eq!(event.family(), TraceEventFamily::Pointer);
    assert!(event.is_cancelable());

    let pointer = physical
        .context()
        .pointer()
        .unwrap_or_else(|| unreachable!("physical observation owns pointer identity"));
    assert_eq!(pointer.pointer_id().get(), 6);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), Some(PointerPhase::Down));

    let surface = physical
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("physical observation owns displayed surface identity"));
    assert_eq!(surface.surface_id(), harness.context.surface_id());
    assert_eq!(
        surface.coordinate_revision(),
        harness.context.coordinate_revision()
    );
    assert_eq!(
        surface.hit_test_generation(),
        harness.context.hit_test_generation()
    );
    assert_eq!(surface.snapshot(), Some(TraceSurfaceSnapshotKind::Current));

    let physical_path = physical
        .context()
        .physical_path()
        .unwrap_or_else(|| unreachable!("physical observation owns the exact physical path"));
    assert_eq!(physical_path.targets().len(), 1);
    assert_eq!(
        physical_path.targets()[0].mounted_node_id(),
        &harness.target
    );
    assert_eq!(
        physical.target().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
}

#[test]
fn pointer_submission_is_non_reentrant_and_exposes_physical_facts() {
    let mut harness = harness(false, false);
    let down = pointer_event(
        &harness,
        1,
        PointerPhase::Down,
        Some(PointerButton::Primary),
        true,
        LogicalDelta::ZERO,
    );

    harness
        .runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("the canonical queue accepts the event"));
    assert!(harness.observations.borrow().is_empty());

    pump_all(&mut harness.runtime);

    assert_pointer_focus(&harness);

    let observations = harness.observations.borrow();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].pointer_id.get(), 1);
    assert_eq!(observations[0].phase, PointerPhase::Down);
    assert_eq!(
        observations[0].physical_target.as_ref(),
        Some(&harness.target)
    );
    assert_eq!(observations[0].physical_path, [harness.target.clone()]);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamRegistered { pointer_id, .. } if pointer_id.get() == 1
    )));
}

#[test]
fn primary_release_inside_emits_exactly_one_activation_and_closes_the_stream() {
    let mut harness = harness(false, false);
    let down = pointer_event(
        &harness,
        2,
        PointerPhase::Down,
        Some(PointerButton::Primary),
        true,
        LogicalDelta::ZERO,
    );
    let up = pointer_event(
        &harness,
        2,
        PointerPhase::Up,
        Some(PointerButton::Primary),
        false,
        LogicalDelta::ZERO,
    );

    harness
        .runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!());
    pump_all(&mut harness.runtime);
    harness
        .runtime
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!());
    pump_all(&mut harness.runtime);

    assert_eq!(harness.activations.get(), 1);
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::SemanticDefaultApplied {
                    command: runenui_core::SemanticCommand::Activate
                }
            ))
            .count(),
        1
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 2
    )));
}

#[test]
fn prevent_default_suppresses_release_activation_and_wheel_scroll() {
    let mut harness = harness(true, true);
    let down = pointer_event(
        &harness,
        3,
        PointerPhase::Down,
        Some(PointerButton::Primary),
        true,
        LogicalDelta::ZERO,
    );
    let up = pointer_event(
        &harness,
        3,
        PointerPhase::Up,
        Some(PointerButton::Primary),
        false,
        LogicalDelta::ZERO,
    );
    harness
        .runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!());
    pump_all(&mut harness.runtime);
    harness
        .runtime
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!());
    pump_all(&mut harness.runtime);

    let wheel = pointer_event(
        &harness,
        4,
        PointerPhase::Wheel,
        None,
        false,
        LogicalDelta::new(0.0, 3.0).unwrap_or_else(|_| unreachable!("the wheel delta is finite")),
    );
    harness
        .runtime
        .submit_pointer(wheel)
        .unwrap_or_else(|_| unreachable!());
    pump_all(&mut harness.runtime);

    assert_eq!(harness.activations.get(), 0);
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::SemanticDefaultApplied {
                    command: runenui_core::SemanticCommand::Activate
                        | runenui_core::SemanticCommand::LogicalScroll(_)
                }
            ))
            .count(),
        0
    );
    assert!(
        harness
            .runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::DefaultPrevented))
    );
}

#[test]
fn wheel_derives_exactly_one_logical_scroll_command() {
    let mut harness = harness(false, false);
    let wheel = pointer_event(
        &harness,
        5,
        PointerPhase::Wheel,
        None,
        false,
        LogicalDelta::new(2.0, -4.0).unwrap_or_else(|_| unreachable!("the wheel delta is finite")),
    );
    harness
        .runtime
        .submit_pointer(wheel)
        .unwrap_or_else(|_| unreachable!());
    pump_all(&mut harness.runtime);

    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::SemanticDefaultApplied {
                    command: runenui_core::SemanticCommand::LogicalScroll(_)
                }
            ))
            .count(),
        1
    );
}

#[test]
fn pointer_trace_reconstructs_validation_routing_default_and_commit_lineage() {
    let mut harness = harness(false, false);
    let submission = harness
        .runtime
        .submit_pointer(pointer_event(
            &harness,
            6,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            true,
            LogicalDelta::ZERO,
        ))
        .unwrap_or_else(|_| unreachable!("the canonical queue accepts the event"));
    pump_all(&mut harness.runtime);

    let records = harness.runtime.trace().records().collect::<Vec<_>>();
    let record = |predicate: &dyn Fn(&TraceRecordKind) -> bool| {
        mandatory_pointer_record(&records, submission.sequence(), predicate)
    };
    let accepted = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerSubmissionAccepted { pointer_id, phase: PointerPhase::Down }
                if pointer_id.get() == 6
        )
    });
    let validated = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerIngressValidated { pointer_id, phase: PointerPhase::Down }
                if pointer_id.get() == 6
        )
    });
    let stream = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerStreamResolved { pointer_id, new_stream: true }
                if pointer_id.get() == 6
        )
    });
    let physical = record(&|kind| matches!(kind, TraceRecordKind::PointerPhysicalTargetResolved));
    let boundary_bundle = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryBundlePlanned { notifications: 1 }
        )
    });
    let routed = record(&|kind| matches!(kind, TraceRecordKind::RoutedEventStarted));
    let default = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerDefaultApplied { pointer_id, phase: PointerPhase::Down }
                if pointer_id.get() == 6
        )
    });
    let modality = record(&|kind| matches!(kind, TraceRecordKind::ModalityChanged { .. }));
    let registered = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
                if pointer_id.get() == 6
        )
    });
    let committed = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerInteractionCommitted { pointer_id }
                if pointer_id.get() == 6
        )
    });
    let capture = record(&|kind| {
        matches!(
            kind,
            TraceRecordKind::PointerCaptureTransitionQueued {
                pointer_id,
                kind: PointerCaptureKind::Gained,
            } if pointer_id.get() == 6
        )
    });

    assert_physical_pointer_observation(physical, &harness);
    assert_eq!(
        boundary_bundle
            .context()
            .pointer()
            .map(|pointer| pointer.pointer_id().get()),
        Some(6)
    );
    assert_eq!(validated.causal_parent(), Some(accepted.sequence()));
    assert_eq!(stream.causal_parent(), Some(validated.sequence()));
    assert_eq!(physical.causal_parent(), Some(stream.sequence()));
    assert_eq!(physical.instant(), routed.instant());
    assert_eq!(boundary_bundle.causal_parent(), Some(physical.sequence()));
    assert_eq!(routed.causal_parent(), Some(boundary_bundle.sequence()));
    assert!(default.sequence() > routed.sequence());
    assert_eq!(modality.causal_parent(), Some(default.sequence()));
    assert_eq!(registered.causal_parent(), Some(modality.sequence()));
    assert_eq!(committed.causal_parent(), Some(registered.sequence()));
    assert_eq!(capture.causal_parent(), Some(committed.sequence()));
}
