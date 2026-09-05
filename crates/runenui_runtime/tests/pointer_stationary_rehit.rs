#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PointerBoundaryKind, PointerDeviceKind,
    PointerEvent, PointerId, PointerPhase, StyleEnvironment, SurfaceInputContext, UiApp, UiEvent,
    View, Widget, WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, PumpReport, SurfaceBuildContext, SurfacePublication,
    TraceRecordKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Observation {
    Boundary {
        pointer_id: u64,
        widget: &'static str,
        kind: PointerBoundaryKind,
        hit_test_generation: u64,
    },
    Pointer {
        pointer_id: u64,
        widget: &'static str,
        hit_test_generation: u64,
    },
}

#[derive(Clone)]
struct State {
    swapped: bool,
    observations: Rc<RefCell<Vec<Observation>>>,
}

#[derive(Clone, Copy)]
enum Action {
    Swap,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let children = if state.swapped {
            children![
                probe("right", &state.observations),
                probe("left", &state.observations),
            ]
        } else {
            children![
                probe("left", &state.observations),
                probe("right", &state.observations),
            ]
        };
        row(children).id("root").key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Swap => state.swapped = !state.swapped,
        }
    }
}

fn probe(name: &'static str, observations: &Rc<RefCell<Vec<Observation>>>) -> Element<Action> {
    Element::new(BoundaryProbe {
        name,
        observations: Rc::clone(observations),
    })
    .id(name)
    .key(name)
}

#[derive(Debug)]
struct BoundaryProbe {
    name: &'static str,
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<Action> for BoundaryProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        _context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::PointerBoundary(boundary) => {
                self.observations.borrow_mut().push(Observation::Boundary {
                    pointer_id: boundary.pointer_id().get(),
                    widget: self.name,
                    kind: boundary.kind(),
                    hit_test_generation: boundary.surface_context().hit_test_generation(),
                });
            }
            UiEvent::Pointer(pointer) if pointer.phase() == PointerPhase::Move => {
                self.observations.borrow_mut().push(Observation::Pointer {
                    pointer_id: pointer.pointer_id().get(),
                    widget: self.name,
                    hit_test_generation: pointer.surface_context().hit_test_generation(),
                });
            }
            _ => {}
        }
        WidgetEventOutput::none()
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
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
}

fn harness() -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        swapped: false,
        observations: Rc::clone(&observations),
    });
    let publication = publish(&mut runtime);
    let left_authored =
        ElementId::new("left").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let right_authored =
        ElementId::new("right").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let left_node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&left_authored))
        .unwrap_or_else(|| unreachable!("the left node is published"));
    let right_node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&right_authored))
        .unwrap_or_else(|| unreachable!("the right node is published"));
    let left_point = center(left_node.bounds());
    let right_point = center(right_node.bounds());
    assert!(runtime.pump(full_budget()).is_quiescent());
    Harness {
        runtime,
        context: publication.input_context().clone(),
        left_point,
        right_point,
        observations,
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(96.0, 48.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    runtime
        .publish_surface(&SurfaceBuildContext::tight(&style_environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"))
}

fn center(bounds: runenui_runtime::LogicalRect) -> LogicalPoint {
    LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"))
}

fn pointer_move(
    pointer_id: u64,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    PointerEvent::new(
        PointerId::new(pointer_id).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        context.clone(),
    )
}

const fn full_budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn pump(runtime: &mut AppRuntime<App>) -> PumpReport {
    runtime.pump(full_budget())
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    assert!(pump(runtime).is_quiescent());
}

#[test]
fn publication_rehits_stationary_streams_in_registration_order_without_move_callbacks() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_move(9, &harness.context, harness.left_point),
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_move(2, &harness.context, harness.right_point),
    );
    harness.observations.borrow_mut().clear();

    harness
        .runtime
        .submit_action(Action::Swap)
        .unwrap_or_else(|_| unreachable!("the swap action is accepted"));
    assert!(pump(&mut harness.runtime).is_quiescent());
    let publication = publish(&mut harness.runtime);
    let generation = publication.input_context().hit_test_generation();
    let report = pump(&mut harness.runtime);

    assert!(report.is_quiescent());
    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [
            Observation::Boundary {
                pointer_id: 9,
                widget: "left",
                kind: PointerBoundaryKind::Leave,
                hit_test_generation: generation,
            },
            Observation::Boundary {
                pointer_id: 9,
                widget: "right",
                kind: PointerBoundaryKind::Enter,
                hit_test_generation: generation,
            },
            Observation::Boundary {
                pointer_id: 2,
                widget: "right",
                kind: PointerBoundaryKind::Leave,
                hit_test_generation: generation,
            },
            Observation::Boundary {
                pointer_id: 2,
                widget: "left",
                kind: PointerBoundaryKind::Enter,
                hit_test_generation: generation,
            },
        ]
    );
    let records = harness.runtime.trace().records().collect::<Vec<_>>();
    let queued = records
        .iter()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerStationaryRehitQueued {
                    hit_test_generation,
                    ..
                } if *hit_test_generation == generation
            )
        })
        .unwrap_or_else(|| unreachable!("publication queues one causally traced re-hit"));
    let validated = |pointer: u64| {
        records
            .iter()
            .copied()
            .find(|record| {
                record.sequence() > queued.sequence()
                    && matches!(
                        record.kind(),
                        TraceRecordKind::PointerIngressValidated {
                            pointer_id,
                            phase: PointerPhase::Move,
                        } if pointer_id.get() == pointer
                    )
            })
            .unwrap_or_else(|| unreachable!("each retained pointer has a validation lineage"))
    };
    let first = validated(9);
    let second = validated(2);
    assert_eq!(first.causal_parent(), Some(queued.sequence()));
    assert_eq!(second.causal_parent(), Some(queued.sequence()));
    assert!(first.sequence() < second.sequence());
}

#[test]
fn publication_does_not_rebind_an_older_accepted_pointer_event() {
    let mut harness = harness();
    let old_generation = harness.context.hit_test_generation();
    harness
        .runtime
        .submit_action(Action::Swap)
        .unwrap_or_else(|_| unreachable!("the swap action is accepted"));
    harness
        .runtime
        .submit_pointer(pointer_move(7, &harness.context, harness.left_point))
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));

    let partial = harness
        .runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(partial.processed_envelopes(), 1);
    let older_work = partial.remaining_queued_envelopes();
    assert!(older_work >= 1);

    let publication = publish(&mut harness.runtime);
    let new_generation = publication.input_context().hit_test_generation();
    assert_ne!(old_generation, new_generation);
    let report = pump(&mut harness.runtime);

    assert!(report.is_quiescent());
    assert_eq!(report.processed_envelopes(), older_work + 1);
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [
            Observation::Boundary {
                pointer_id: 7,
                widget: "left",
                kind: PointerBoundaryKind::Enter,
                hit_test_generation: old_generation,
            },
            Observation::Pointer {
                pointer_id: 7,
                widget: "left",
                hit_test_generation: old_generation,
            },
            Observation::Boundary {
                pointer_id: 7,
                widget: "left",
                kind: PointerBoundaryKind::Leave,
                hit_test_generation: new_generation,
            },
            Observation::Boundary {
                pointer_id: 7,
                widget: "right",
                kind: PointerBoundaryKind::Enter,
                hit_test_generation: new_generation,
            },
        ]
    );
}
