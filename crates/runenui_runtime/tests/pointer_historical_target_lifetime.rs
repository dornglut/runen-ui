#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, ElementId, EventContext, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerDeviceKind,
    PointerEvent, PointerId, PointerPhase, StyleEnvironment, SurfaceInputContext, UiApp, UiEvent,
    View, Widget, WidgetActivation, WidgetActivationContext, WidgetActivationOutput,
    WidgetEventOutput, WidgetMeasure, WorkSequence, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeStatus, SurfaceBuildContext, TracePointerRejection,
    TraceRecordKind,
};

#[derive(Clone)]
struct State {
    show_old: bool,
    capture_observations: Rc<RefCell<Vec<PointerPhase>>>,
    old_observations: Rc<RefCell<Vec<PointerPhase>>>,
    replacement_observations: Rc<RefCell<Vec<PointerPhase>>>,
    activations: Rc<Cell<usize>>,
}

#[derive(Clone, Copy)]
enum Action {
    Replace,
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let switching = if state.show_old {
            Element::new(Probe {
                observations: Rc::clone(&state.old_observations),
            })
            .id("old.target")
            .key("old.target")
        } else {
            Element::new(Probe {
                observations: Rc::clone(&state.replacement_observations),
            })
            .id("replacement.target")
            .key("replacement.target")
        };
        row(children![
            Element::new(Probe {
                observations: Rc::clone(&state.capture_observations),
            })
            .id("capture.target")
            .key("capture.target"),
            switching,
        ])
        .id("root")
        .key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Replace => state.show_old = false,
            Action::Activated => state.activations.set(state.activations.get() + 1),
        }
    }
}

#[derive(Debug)]
struct Probe {
    observations: Rc<RefCell<Vec<PointerPhase>>>,
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
        if let UiEvent::Pointer(pointer) = event {
            self.observations.borrow_mut().push(pointer.phase());
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

    fn measure(
        &self,
        _state: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::new(32.0).unwrap_or_default(),
            LogicalLength::new(32.0).unwrap_or_default(),
        )
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
    context: SurfaceInputContext,
    capture_point: LogicalPoint,
    old_point: LogicalPoint,
    capture_observations: Rc<RefCell<Vec<PointerPhase>>>,
    old_observations: Rc<RefCell<Vec<PointerPhase>>>,
    replacement_observations: Rc<RefCell<Vec<PointerPhase>>>,
    activations: Rc<Cell<usize>>,
}

fn harness() -> Harness {
    let capture_observations = Rc::new(RefCell::new(Vec::new()));
    let old_observations = Rc::new(RefCell::new(Vec::new()));
    let replacement_observations = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_old: true,
        capture_observations: Rc::clone(&capture_observations),
        old_observations: Rc::clone(&old_observations),
        replacement_observations: Rc::clone(&replacement_observations),
        activations: Rc::clone(&activations),
    });
    pump_all(&mut runtime);

    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 32.0)
        .unwrap_or_else(|_| unreachable!("fixture surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|error| unreachable!("fixture publication is valid: {error:?}"));
    let capture_point = authored_center(&publication, "capture.target");
    let old_point = authored_center(&publication, "old.target");

    Harness {
        runtime,
        context: publication.input_context().clone(),
        capture_point,
        old_point,
        capture_observations,
        old_observations,
        replacement_observations,
        activations,
    }
}

fn authored_center(
    publication: &runenui_runtime::SurfacePublication,
    authored_id: &str,
) -> LogicalPoint {
    let authored = ElementId::new(authored_id)
        .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture target is published"));
    let bounds = node.bounds();
    LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"))
}

fn pointer_event(
    pointer_id: u64,
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    let event = PointerEvent::new(
        PointerId::new(pointer_id).unwrap_or_else(|| unreachable!("pointer id is non-zero")),
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

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) -> WorkSequence {
    let submission = runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("fixture pointer ingress is accepted"));
    pump_all(runtime);
    submission.sequence()
}

fn replace_target_without_publishing(harness: &mut Harness) {
    harness
        .runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is accepted"));
    pump_all(&mut harness.runtime);
    assert!(!harness.runtime.state().show_old);
    clear_observations(harness);
}

fn clear_observations(harness: &Harness) {
    harness.capture_observations.borrow_mut().clear();
    harness.old_observations.borrow_mut().clear();
    harness.replacement_observations.borrow_mut().clear();
    harness.activations.set(0);
}

fn has_pointer_rejection(
    runtime: &AppRuntime<App>,
    trace_start: usize,
    pointer_id: u64,
    phase: PointerPhase,
    outcome: TracePointerRejection,
) -> bool {
    runtime.trace().records().skip(trace_start).any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::PointerIngressRejected {
                pointer_id: actual,
                phase: actual_phase,
                outcome: actual_outcome,
            } if actual.get() == pointer_id
                && *actual_phase == phase
                && *actual_outcome == outcome
        )
    })
}

#[test]
fn stale_displayed_hit_down_rejects_without_poison_or_current_geometry_retarget() {
    let mut harness = harness();
    replace_target_without_publishing(&mut harness);
    let trace_start = harness.runtime.trace().len();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(81, PointerPhase::Down, &harness.context, harness.old_point),
    );

    assert_eq!(harness.runtime.status(), RuntimeStatus::Running);
    assert!(harness.capture_observations.borrow().is_empty());
    assert!(harness.old_observations.borrow().is_empty());
    assert!(harness.replacement_observations.borrow().is_empty());
    assert_eq!(harness.activations.get(), 0);
    assert!(has_pointer_rejection(
        &harness.runtime,
        trace_start,
        81,
        PointerPhase::Down,
        TracePointerRejection::NoTarget,
    ));
    assert!(
        !harness
            .runtime
            .trace()
            .records()
            .skip(trace_start)
            .any(|record| {
                matches!(
                    record.kind(),
                    TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
                        if pointer_id.get() == 81
                )
            })
    );

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            81,
            PointerPhase::Cancel,
            &harness.context,
            harness.old_point,
        ),
    );
    assert!(has_pointer_rejection(
        &harness.runtime,
        trace_start,
        81,
        PointerPhase::Cancel,
        TracePointerRejection::MissingStream,
    ));
}

#[test]
fn stale_displayed_hit_up_closes_existing_stream_without_route_or_activation() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(83, PointerPhase::Down, &harness.context, harness.old_point),
    );
    replace_target_without_publishing(&mut harness);
    let trace_start = harness.runtime.trace().len();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(83, PointerPhase::Up, &harness.context, harness.old_point),
    );

    assert_eq!(harness.runtime.status(), RuntimeStatus::Running);
    assert!(harness.capture_observations.borrow().is_empty());
    assert!(harness.old_observations.borrow().is_empty());
    assert!(harness.replacement_observations.borrow().is_empty());
    assert_eq!(harness.activations.get(), 0);
    assert!(!has_pointer_rejection(
        &harness.runtime,
        trace_start,
        83,
        PointerPhase::Up,
        TracePointerRejection::NoTarget,
    ));
    assert!(
        harness
            .runtime
            .trace()
            .records()
            .skip(trace_start)
            .any(|record| {
                matches!(
                    record.kind(),
                    TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 83
                )
            })
    );

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            83,
            PointerPhase::Cancel,
            &harness.context,
            harness.old_point,
        ),
    );
    assert!(has_pointer_rejection(
        &harness.runtime,
        trace_start,
        83,
        PointerPhase::Cancel,
        TracePointerRejection::MissingStream,
    ));
}

#[test]
fn stale_physical_hit_preserves_distinct_live_capture_routing_without_retarget() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            89,
            PointerPhase::Down,
            &harness.context,
            harness.capture_point,
        ),
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(89, PointerPhase::Move, &harness.context, harness.old_point),
    );
    replace_target_without_publishing(&mut harness);
    let trace_start = harness.runtime.trace().len();

    let move_sequence = submit_and_pump(
        &mut harness.runtime,
        pointer_event(89, PointerPhase::Move, &harness.context, harness.old_point),
    );

    assert_eq!(harness.runtime.status(), RuntimeStatus::Running);
    assert_eq!(
        harness.capture_observations.borrow().as_slice(),
        [PointerPhase::Move]
    );
    assert!(harness.old_observations.borrow().is_empty());
    assert!(harness.replacement_observations.borrow().is_empty());
    assert_eq!(harness.activations.get(), 0);
    let physical = harness
        .runtime
        .trace()
        .records()
        .skip(trace_start)
        .find(|record| {
            record.work_sequence() == Some(move_sequence)
                && matches!(record.kind(), TraceRecordKind::PointerPhysicalTargetResolved)
        })
        .unwrap_or_else(|| unreachable!("stale captured move records physical resolution"));
    assert_eq!(physical.target(), None);
    assert!(
        physical
            .context()
            .physical_path()
            .is_some_and(|path| path.targets().is_empty())
    );
    assert!(!harness.runtime.trace().records().skip(trace_start).any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::PointerIngressRejected { pointer_id, .. } if pointer_id.get() == 89
        )
    }));
}
