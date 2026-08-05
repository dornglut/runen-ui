#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceDeliveryOutcome,
    TraceEventFamily, TracePointerRejection, TraceRecord, TraceRecordKind, TraceTarget,
    WorkSequence,
};

#[derive(Clone)]
struct State {
    callbacks: Rc<RefCell<Vec<Observation>>>,
    activations: Rc<Cell<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Observation {
    Pointer(PointerPhase),
    Capture(PointerCaptureKind),
}

#[derive(Clone, Copy)]
enum Action {
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
        match action {
            Action::Activated => state.activations.set(state.activations.get() + 1),
        }
    }
}

#[derive(Debug)]
struct Probe {
    callbacks: Rc<RefCell<Vec<Observation>>>,
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
        match event {
            UiEvent::Pointer(pointer) => self
                .callbacks
                .borrow_mut()
                .push(Observation::Pointer(pointer.phase())),
            UiEvent::PointerCapture(capture) => self
                .callbacks
                .borrow_mut()
                .push(Observation::Capture(capture.kind())),
            _ => {}
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

fn pointer_event(
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(79).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        phase,
        point,
        context.clone(),
    );
    match phase {
        PointerPhase::Down => event
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
        _ => event,
    }
}

fn pump_all(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
}

fn mandatory_record<'a>(
    records: &[&'a TraceRecord],
    sequence: WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| record.work_sequence() == Some(sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("the mandatory cleanup trace fact is retained"))
}

fn assert_causal_ancestor(
    records: &[&TraceRecord],
    descendant: &TraceRecord,
    ancestor: &TraceRecord,
) {
    let mut parent = descendant.causal_parent();
    while parent != Some(ancestor.sequence()) {
        let sequence = parent.unwrap_or_else(|| {
            unreachable!("delivered capture loss must descend from stream closure")
        });
        parent = records
            .iter()
            .copied()
            .find(|record| record.sequence() == sequence)
            .unwrap_or_else(|| unreachable!("every retained parent is present in this trace"))
            .causal_parent();
    }
}

fn assert_prior_stream_surface(record: &TraceRecord, current: &SurfaceInputContext) {
    let surface = record
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("cleanup owns prior stream surface identity"));
    assert_eq!(surface.surface_id(), current.surface_id());
    assert_eq!(surface.coordinate_revision(), current.coordinate_revision());
    assert_eq!(surface.hit_test_generation(), current.hit_test_generation());
    assert_eq!(surface.snapshot(), None);
}

fn assert_integrity_cleanup(
    cleanup: &TraceRecord,
    current: &SurfaceInputContext,
    target: &runenui_core::MountedNodeId,
) {
    assert!(matches!(
        cleanup.kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted
    ));
    let context = cleanup.context();
    assert_eq!(context.event(), None);
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("cleanup owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id().get(), 79);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    assert_prior_stream_surface(cleanup, current);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("cleanup owns the prior physical path"));
    assert_eq!(path.targets().len(), 1);
    assert_eq!(path.targets()[0].mounted_node_id(), target);
    let facts = context
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("cleanup owns exact owner transitions"));
    let pressed = facts
        .pressed_owner()
        .unwrap_or_else(|| unreachable!("pressed ownership is cleared"));
    assert_eq!(
        pressed.previous().map(TraceTarget::mounted_node_id),
        Some(target)
    );
    assert_eq!(pressed.current(), None);
    let capture = facts
        .capture_owner()
        .unwrap_or_else(|| unreachable!("capture ownership is cleared"));
    assert_eq!(
        capture.previous().map(TraceTarget::mounted_node_id),
        Some(target)
    );
    assert_eq!(capture.current(), None);
    assert!(facts.physical_path_cleared());
    assert_eq!(context.route(), None);
    assert_eq!(context.delivery(), None);
}

fn assert_delivered_capture_loss(
    capture_lost: &TraceRecord,
    current: &SurfaceInputContext,
    target: &runenui_core::MountedNodeId,
) {
    assert!(matches!(
        capture_lost.kind(),
        TraceRecordKind::PointerCaptureNotificationResolved {
            kind: PointerCaptureKind::Lost,
        }
    ));
    assert_eq!(
        capture_lost.target().map(TraceTarget::mounted_node_id),
        Some(target)
    );
    let context = capture_lost.context();
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("capture loss owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::PointerCapture);
    assert!(!event.is_cancelable());
    assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Delivered));
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("capture loss owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id().get(), 79);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    assert_prior_stream_surface(capture_lost, current);
    let route = context
        .route()
        .unwrap_or_else(|| unreachable!("capture loss owns its target-only route"));
    assert_eq!(route.targets().len(), 1);
    assert_eq!(route.targets()[0].mounted_node_id(), target);
    assert_eq!(route.related_target(), None);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("capture loss owns the prior physical path"));
    assert_eq!(path.targets().len(), 1);
    assert_eq!(path.targets()[0].mounted_node_id(), target);
    let transition = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("capture loss owns exact capture endpoints"));
    assert_eq!(
        transition.previous().map(TraceTarget::mounted_node_id),
        Some(target)
    );
    assert_eq!(transition.current(), None);
}

#[test]
fn missing_context_up_commits_integrity_only_cleanup_with_causal_trace() {
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        callbacks: Rc::clone(&callbacks),
        activations: Rc::clone(&activations),
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
    let bounds = node.bounds();
    let target = node.id().clone();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let current = publication.input_context().clone();

    runtime
        .submit_pointer(pointer_event(PointerPhase::Down, &current, point))
        .unwrap_or_else(|_| unreachable!("the down event is accepted"));
    pump_all(&mut runtime);
    callbacks.borrow_mut().clear();
    activations.set(0);

    let missing = runtime.__surface_context_for_test(
        0,
        1,
        current.coordinate_revision(),
        current.hit_test_generation() + 100,
    );
    let submission = runtime
        .submit_pointer(pointer_event(PointerPhase::Up, &missing, point))
        .unwrap_or_else(|_| unreachable!("the up event is accepted before processing"));
    let sequence = submission.sequence();
    pump_all(&mut runtime);

    assert_eq!(
        callbacks.borrow().as_slice(),
        [Observation::Capture(PointerCaptureKind::Lost)]
    );
    assert_eq!(activations.get(), 0);
    let records = runtime.trace().records().collect::<Vec<_>>();
    let rejected = mandatory_record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerIngressRejected {
                pointer_id,
                phase: PointerPhase::Up,
                outcome: TracePointerRejection::MissingGeneration,
            } if pointer_id.get() == 79
        )
    });
    let cleanup = mandatory_record(&records, sequence, |kind| {
        matches!(kind, TraceRecordKind::PointerIntegrityCleanupCommitted)
    });
    let closed = mandatory_record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 79
        )
    });
    let capture_lost = mandatory_record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerCaptureNotificationResolved {
                kind: PointerCaptureKind::Lost,
            }
        )
    });

    assert_integrity_cleanup(cleanup, &current, &target);
    assert_delivered_capture_loss(capture_lost, &current, &target);
    assert_eq!(cleanup.causal_parent(), Some(rejected.sequence()));
    assert_eq!(closed.causal_parent(), Some(cleanup.sequence()));
    assert_causal_ancestor(&records, capture_lost, closed);
    assert_eq!(cleanup.instant(), capture_lost.instant());
    assert!(cleanup.instant().is_some());
    assert_ne!(
        capture_lost
            .context()
            .surface()
            .map(|surface| surface.hit_test_generation()),
        Some(missing.hit_test_generation())
    );
}
