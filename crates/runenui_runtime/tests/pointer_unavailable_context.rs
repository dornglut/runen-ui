#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, HitContribution, HitContributionContext, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerBoundaryKind, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticCommand, StyleEnvironment,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePhase, TracePointerRejection,
    TraceRecordKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Observation {
    Pointer(PointerPhase),
    Boundary(PointerBoundaryKind),
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            observations: Rc::clone(&state.observations),
        })
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

#[derive(Debug)]
struct Probe {
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<()> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        _context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::Pointer(pointer) => self
                .observations
                .borrow_mut()
                .push(Observation::Pointer(pointer.phase())),
            UiEvent::PointerBoundary(boundary) => self
                .observations
                .borrow_mut()
                .push(Observation::Boundary(boundary.kind())),
            _ => {}
        }
        WidgetEventOutput::none()
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
    context: SurfaceInputContext,
    inside: LogicalPoint,
    outside: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
}

fn harness() -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
    });
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&style_environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"));
    let bounds = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the root is published"))
        .bounds();
    let inside = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let outside = LogicalPoint::new(bounds.max_x() + 1.0, bounds.max_y() + 1.0)
        .unwrap_or_else(|_| unreachable!("the outside point is finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        inside,
        outside,
        observations,
    }
}

fn pointer_event(
    pointer_id: u64,
    device_kind: PointerDeviceKind,
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    let event = PointerEvent::new(
        PointerId::new(pointer_id).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        device_kind,
        phase,
        point,
        context.clone(),
    );
    if phase == PointerPhase::Down {
        event
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary)
    } else {
        event
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

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the event is accepted before processing"));
    pump_all(runtime);
}

fn assert_rejection_since(
    runtime: &AppRuntime<App>,
    start: usize,
    pointer_id: u64,
    phase: PointerPhase,
    expected: TracePointerRejection,
) {
    let records = runtime.trace().records().skip(start).collect::<Vec<_>>();
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(
                record.kind(),
                TraceRecordKind::PointerIngressRejected {
                    pointer_id: actual,
                    phase: actual_phase,
                    outcome,
                } if actual.get() == pointer_id
                    && *actual_phase == phase
                    && outcome == &expected
            ))
            .count(),
        1
    );
    assert!(!records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerStreamRegistered { pointer_id: actual, .. }
            if actual.get() == pointer_id
    )));
    assert!(!records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RouteSnapshotCreated { .. }
            | TraceRecordKind::SemanticDefaultApplied { .. }
    )));
}

fn registration_count(runtime: &AppRuntime<App>, pointer_id: u64) -> usize {
    runtime
        .trace()
        .kinds()
        .filter(|kind| {
            matches!(
                kind,
                TraceRecordKind::PointerStreamRegistered { pointer_id: actual, .. }
                    if actual.get() == pointer_id
            )
        })
        .count()
}

#[test]
fn unavailable_down_and_wheel_consume_no_stream_or_default_authority() {
    let mut harness = harness();
    let missing = harness.runtime.__surface_context_for_test(
        0,
        1,
        harness.context.coordinate_revision(),
        harness.context.hit_test_generation() + 100,
    );

    for (pointer_id, phase) in [(81, PointerPhase::Down), (82, PointerPhase::Wheel)] {
        harness.observations.borrow_mut().clear();
        let start = harness.runtime.trace().len();
        submit_and_pump(
            &mut harness.runtime,
            pointer_event(
                pointer_id,
                PointerDeviceKind::Mouse,
                phase,
                &missing,
                harness.inside,
            ),
        );
        assert!(harness.observations.borrow().is_empty());
        assert_rejection_since(
            &harness.runtime,
            start,
            pointer_id,
            phase,
            TracePointerRejection::MissingGeneration,
        );
        assert_eq!(registration_count(&harness.runtime, pointer_id), 0);

        submit_and_pump(
            &mut harness.runtime,
            pointer_event(
                pointer_id,
                PointerDeviceKind::Mouse,
                phase,
                &harness.context,
                harness.inside,
            ),
        );
        assert_eq!(registration_count(&harness.runtime, pointer_id), 1);
        if phase == PointerPhase::Wheel {
            assert_eq!(
                harness
                    .runtime
                    .trace()
                    .kinds()
                    .filter(|kind| matches!(
                        kind,
                        TraceRecordKind::SemanticDefaultApplied {
                            command: SemanticCommand::LogicalScroll(_)
                        }
                    ))
                    .count(),
                1
            );
        }
        submit_and_pump(
            &mut harness.runtime,
            pointer_event(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Cancel,
                &harness.context,
                harness.inside,
            ),
        );
    }
}

#[test]
fn unavailable_move_preserves_the_retained_physical_path() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            83,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            &harness.context,
            harness.inside,
        ),
    );
    harness.observations.borrow_mut().clear();
    let missing = harness.runtime.__surface_context_for_test(
        0,
        1,
        harness.context.coordinate_revision(),
        harness.context.hit_test_generation() + 100,
    );
    let start = harness.runtime.trace().len();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            83,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            &missing,
            harness.outside,
        ),
    );
    assert!(harness.observations.borrow().is_empty());
    assert_rejection_since(
        &harness.runtime,
        start,
        83,
        PointerPhase::Move,
        TracePointerRejection::MissingGeneration,
    );

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            83,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            &harness.context,
            harness.inside,
        ),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation::Pointer(PointerPhase::Move)]
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            83,
            PointerDeviceKind::Mouse,
            PointerPhase::Cancel,
            &harness.context,
            harness.inside,
        ),
    );
}

#[test]
fn validation_order_rejects_surface_before_stream_and_stream_before_generation() {
    let mut local = harness();
    let foreign_runtime = harness();
    submit_and_pump(
        &mut local.runtime,
        pointer_event(
            84,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            &local.context,
            local.inside,
        ),
    );
    local.observations.borrow_mut().clear();

    let foreign_surface = local.runtime.__surface_context_for_test(
        1,
        1,
        local.context.coordinate_revision(),
        local.context.hit_test_generation(),
    );
    let missing = local.runtime.__surface_context_for_test(
        0,
        1,
        local.context.coordinate_revision(),
        local.context.hit_test_generation() + 100,
    );
    let cases = [
        (
            &foreign_runtime.context,
            TracePointerRejection::ForeignRuntime,
        ),
        (&foreign_surface, TracePointerRejection::ForeignSurface),
        (&missing, TracePointerRejection::DeviceKindMismatch),
    ];

    for (context, expected) in cases {
        let start = local.runtime.trace().len();
        submit_and_pump(
            &mut local.runtime,
            pointer_event(
                84,
                PointerDeviceKind::Pen,
                PointerPhase::Move,
                context,
                local.outside,
            ),
        );
        assert!(local.observations.borrow().is_empty());
        assert_rejection_since(&local.runtime, start, 84, PointerPhase::Move, expected);
    }

    submit_and_pump(
        &mut local.runtime,
        pointer_event(
            84,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            &local.context,
            local.inside,
        ),
    );
    assert_eq!(
        local.observations.borrow().as_slice(),
        [Observation::Pointer(PointerPhase::Move)]
    );
    submit_and_pump(
        &mut local.runtime,
        pointer_event(
            84,
            PointerDeviceKind::Mouse,
            PointerPhase::Cancel,
            &local.context,
            local.inside,
        ),
    );
}

#[test]
fn malformed_missing_context_up_rejects_before_integrity_settlement() {
    let mut harness = harness();
    let pointer_id =
        PointerId::new(85).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            pointer_id.get(),
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            &harness.context,
            harness.inside,
        ),
    );
    assert_eq!(registration_count(&harness.runtime, pointer_id.get()), 1);
    harness.observations.borrow_mut().clear();

    let missing = harness.runtime.__surface_context_for_test(
        0,
        1,
        harness.context.coordinate_revision(),
        harness.context.hit_test_generation() + 100,
    );
    let start = harness.runtime.trace().len();
    let malformed = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        harness.inside,
        missing,
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    submit_and_pump(&mut harness.runtime, malformed);

    assert!(harness.observations.borrow().is_empty());
    assert_rejection_since(
        &harness.runtime,
        start,
        pointer_id.get(),
        PointerPhase::Up,
        TracePointerRejection::ButtonTransitionMismatch,
    );
    let rejected = harness
        .runtime
        .trace()
        .records()
        .skip(start)
        .collect::<Vec<_>>();
    assert!(!rejected.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIngressRejected {
            pointer_id: actual,
            phase: PointerPhase::Up,
            outcome: TracePointerRejection::MissingGeneration,
        } if actual == &pointer_id
    )));
    assert!(!rejected.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id: actual } if actual == &pointer_id
    )));

    let valid = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        harness.inside,
        harness.context.clone(),
    )
    .with_changed_button(PointerButton::Primary);
    submit_and_pump(&mut harness.runtime, valid);

    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::PointerStreamClosed { pointer_id: actual }
                    if actual == &pointer_id
            ))
            .count(),
        1
    );
}

#[test]
fn unavailable_terminal_close_redraws_when_hover_projection_is_removed() {
    let mut harness = harness();
    let pointer_id =
        PointerId::new(86).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            pointer_id.get(),
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            &harness.context,
            harness.inside,
        ),
    );
    let hover_redraw = harness
        .runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("initial hover requests redraw"));
    harness
        .runtime
        .acknowledge_redraw(&hover_redraw)
        .unwrap_or_else(|_| unreachable!("runtime redraw token remains local"));

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            pointer_id.get(),
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            &harness.context,
            harness.inside,
        ),
    );
    assert!(
        harness.runtime.take_redraw_request().is_none(),
        "non-actionable button state alone does not alter style interaction facts"
    );

    let missing = harness.runtime.__surface_context_for_test(
        0,
        1,
        harness.context.coordinate_revision(),
        harness.context.hit_test_generation() + 100,
    );
    let start = harness.runtime.trace().len();
    let unavailable_up = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        harness.inside,
        missing,
    )
    .with_changed_button(PointerButton::Primary);
    submit_and_pump(&mut harness.runtime, unavailable_up);

    let redraw = harness
        .runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("closing the hovered stream requests redraw"));
    let records = harness
        .runtime
        .trace()
        .records()
        .skip(start)
        .collect::<Vec<_>>();
    let closed = records
        .iter()
        .copied()
        .find(|record| matches!(
            record.kind(),
            TraceRecordKind::PointerStreamClosed { pointer_id: actual } if actual == &pointer_id
        ))
        .unwrap_or_else(|| unreachable!("unavailable terminal cleanup closes the stream"));
    let requested = records
        .iter()
        .copied()
        .find(|record| matches!(record.kind(), TraceRecordKind::RedrawRequested { .. }))
        .unwrap_or_else(|| unreachable!("projection cleanup records its redraw request"));
    assert_eq!(requested.causal_parent(), Some(closed.sequence()));

    harness
        .runtime
        .acknowledge_redraw(&redraw)
        .unwrap_or_else(|_| unreachable!("runtime redraw token remains local"));
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    harness
        .runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("interaction-only publication is admitted"));
    assert_eq!(
        harness.runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Style]
    );
}
