#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, LogicalDelta, LogicalLength, LogicalPoint, NoHostProtocol,
    PointerButton, PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, StyleTokens, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceRecordKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Active,
    Disabled,
    NonActionable,
}

struct State {
    mode: Mode,
    activations: usize,
    pointer_phases: Rc<RefCell<Vec<PointerPhase>>>,
    capture_kinds: Rc<RefCell<Vec<PointerCaptureKind>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    SetMode(Mode),
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            mode: state.mode,
            pointer_phases: Rc::clone(&state.pointer_phases),
            capture_kinds: Rc::clone(&state.capture_kinds),
        })
        .key("probe")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::SetMode(mode) => state.mode = mode,
            Action::Activated => state.activations += 1,
        }
    }
}

#[derive(Debug)]
struct Probe {
    mode: Mode,
    pointer_phases: Rc<RefCell<Vec<PointerPhase>>>,
    capture_kinds: Rc<RefCell<Vec<PointerCaptureKind>>>,
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
            UiEvent::Pointer(pointer) => {
                self.pointer_phases.borrow_mut().push(pointer.phase());
            }
            UiEvent::PointerCapture(capture) => {
                self.capture_kinds.borrow_mut().push(capture.kind());
            }
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        match self.mode {
            Mode::Active => WidgetActivation::actionable(true),
            Mode::Disabled => WidgetActivation::actionable(false),
            Mode::NonActionable => WidgetActivation::NONE,
        }
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
    outside: LogicalPoint,
    pointer_phases: Rc<RefCell<Vec<PointerPhase>>>,
    capture_kinds: Rc<RefCell<Vec<PointerCaptureKind>>>,
}

fn harness() -> Harness {
    let pointer_phases = Rc::new(RefCell::new(Vec::new()));
    let capture_kinds = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        mode: Mode::Active,
        activations: 0,
        pointer_phases: Rc::clone(&pointer_phases),
        capture_kinds: Rc::clone(&capture_kinds),
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
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let outside = LogicalPoint::new(bounds.max_x() + 1.0, bounds.max_y() + 1.0)
        .unwrap_or_else(|_| unreachable!("the outside point is finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        target: node.id().clone(),
        point,
        outside,
        pointer_phases,
        capture_kinds,
    }
}

fn pointer_event(harness: &Harness, phase: PointerPhase, primary_pressed: bool) -> PointerEvent {
    let buttons = if primary_pressed {
        PointerButtons::new([PointerButton::Primary])
    } else {
        PointerButtons::default()
    };
    PointerEvent::new(
        PointerId::new(71).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        phase,
        harness.point,
        harness.context.clone(),
    )
    .with_buttons(buttons)
    .with_changed_button(PointerButton::Primary)
    .with_movement_delta(LogicalDelta::ZERO)
}

fn pointer_event_at(
    harness: &Harness,
    phase: PointerPhase,
    position: LogicalPoint,
    primary_pressed: bool,
) -> PointerEvent {
    let buttons = if primary_pressed {
        PointerButtons::new([PointerButton::Primary])
    } else {
        PointerButtons::default()
    };
    let event = PointerEvent::new(
        PointerId::new(71).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        phase,
        position,
        harness.context.clone(),
    )
    .with_buttons(buttons);
    match phase {
        PointerPhase::Down | PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
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

fn assert_capability_transition_clears_interaction(mode: Mode) {
    let mut harness = harness();
    let down = pointer_event(&harness, PointerPhase::Down, true);
    harness
        .runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("the down event is accepted"));
    pump_all(&mut harness.runtime);
    assert_eq!(
        harness.pointer_phases.borrow().as_slice(),
        [PointerPhase::Down]
    );
    assert_eq!(
        harness.capture_kinds.borrow().as_slice(),
        [PointerCaptureKind::Gained]
    );
    harness.pointer_phases.borrow_mut().clear();
    harness.capture_kinds.borrow_mut().clear();

    let cleanup_start = harness.runtime.trace().len();
    harness
        .runtime
        .submit_action(Action::SetMode(mode))
        .unwrap_or_else(|_| unreachable!("the application action is accepted"));
    pump_all(&mut harness.runtime);

    assert_eq!(harness.runtime.index().nodes()[0].id(), &harness.target);
    let cleanup_records = harness
        .runtime
        .trace()
        .records()
        .skip(cleanup_start)
        .collect::<Vec<_>>();
    assert!(cleanup_records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted {
            pointer_id,
            pressed: true,
            capture: true,
            physical_path: false,
        } if pointer_id.get() == 71
    )));
    assert!(cleanup_records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerCaptureTransitionQueued {
            pointer_id,
            kind: PointerCaptureKind::Lost,
        } if pointer_id.get() == 71
    )));
    assert!(!cleanup_records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerCaptureNotificationSuppressed {
            pointer_id,
            kind: PointerCaptureKind::Lost,
        } if pointer_id.get() == 71
    )));
    assert!(harness.pointer_phases.borrow().is_empty());
    assert_eq!(
        harness.capture_kinds.borrow().as_slice(),
        [PointerCaptureKind::Lost]
    );

    let up = pointer_event(&harness, PointerPhase::Up, false);
    harness
        .runtime
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("the up event is accepted"));
    pump_all(&mut harness.runtime);

    assert_eq!(harness.runtime.state().activations, 0);
    assert_eq!(
        harness.pointer_phases.borrow().as_slice(),
        [PointerPhase::Up]
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 71
    )));
}

#[test]
fn disablement_clears_pressed_and_capture_before_later_input() {
    assert_capability_transition_clears_interaction(Mode::Disabled);
}

#[test]
fn non_actionable_transition_clears_pressed_and_capture_before_later_input() {
    assert_capability_transition_clears_interaction(Mode::NonActionable);
}

#[test]
fn release_outside_routes_to_capture_but_never_activates() {
    let mut harness = harness();
    let down = pointer_event_at(&harness, PointerPhase::Down, harness.point, true);
    harness
        .runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("the down event is accepted"));
    pump_all(&mut harness.runtime);
    harness.pointer_phases.borrow_mut().clear();

    let moved = pointer_event_at(&harness, PointerPhase::Move, harness.outside, true);
    harness
        .runtime
        .submit_pointer(moved)
        .unwrap_or_else(|_| unreachable!("the move event is accepted"));
    pump_all(&mut harness.runtime);
    let up = pointer_event_at(&harness, PointerPhase::Up, harness.outside, false);
    harness
        .runtime
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("the up event is accepted"));
    pump_all(&mut harness.runtime);

    assert_eq!(
        harness.pointer_phases.borrow().as_slice(),
        [PointerPhase::Move, PointerPhase::Up]
    );
    assert_eq!(harness.runtime.state().activations, 0);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 71
    )));
}
