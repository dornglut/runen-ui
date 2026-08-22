#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, HitContribution, HitContributionContext, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerBoundaryKind, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget,
    WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TracePointerRejection,
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
    point: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
}

fn harness() -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
    });
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
        observations,
    }
}

fn pointer_event(
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    PointerEvent::new(
        PointerId::new(83).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        phase,
        point,
        context.clone(),
    )
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted before processing"));
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

fn registration_count(runtime: &AppRuntime<App>) -> usize {
    runtime
        .trace()
        .kinds()
        .filter(|kind| {
            matches!(
                kind,
                TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
                    if pointer_id.get() == 83
            )
        })
        .count()
}

fn assert_rejected_cancel_preserves_local_stream(
    harness: &mut Harness,
    rejected_context: &SurfaceInputContext,
    expected: TracePointerRejection,
) {
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(PointerPhase::Move, &harness.context, harness.point),
    );
    assert_eq!(registration_count(&harness.runtime), 1);
    harness.observations.borrow_mut().clear();
    let rejection_start = harness.runtime.trace().len();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(PointerPhase::Cancel, rejected_context, harness.point),
    );
    assert!(harness.observations.borrow().is_empty());
    let rejected_records = harness
        .runtime
        .trace()
        .records()
        .skip(rejection_start)
        .collect::<Vec<_>>();
    assert!(rejected_records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Cancel,
            outcome,
        } if pointer_id.get() == 83 && outcome == &expected
    )));
    assert!(!rejected_records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 83
    )));
    assert_eq!(registration_count(&harness.runtime), 1);

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(PointerPhase::Move, &harness.context, harness.point),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation::Pointer(PointerPhase::Move)]
    );
    assert_eq!(registration_count(&harness.runtime), 1);
    harness.observations.borrow_mut().clear();
    let close_start = harness.runtime.trace().len();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(PointerPhase::Cancel, &harness.context, harness.point),
    );
    assert!(harness.observations.borrow().is_empty());
    let close_records = harness
        .runtime
        .trace()
        .records()
        .skip(close_start)
        .collect::<Vec<_>>();
    assert_eq!(
        close_records
            .iter()
            .filter(|record| matches!(
                record.kind(),
                TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 83
            ))
            .count(),
        1
    );
    assert!(!close_records.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Cancel,
            outcome: TracePointerRejection::MissingStream,
        } if pointer_id.get() == 83
    )));
}

#[test]
fn foreign_runtime_cancel_cannot_mutate_a_local_stream() {
    let mut local = harness();
    let foreign = harness();
    assert_rejected_cancel_preserves_local_stream(
        &mut local,
        &foreign.context,
        TracePointerRejection::ForeignRuntime,
    );
}

#[test]
fn foreign_surface_cancel_cannot_mutate_a_local_stream() {
    let mut harness = harness();
    let foreign_surface = harness.runtime.__surface_context_for_test(
        1,
        1,
        harness.context.coordinate_revision(),
        harness.context.hit_test_generation(),
    );
    assert_rejected_cancel_preserves_local_stream(
        &mut harness,
        &foreign_surface,
        TracePointerRejection::ForeignSurface,
    );
}
