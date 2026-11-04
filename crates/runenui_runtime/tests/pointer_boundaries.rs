#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, EventPhase, HitContribution, HitContributionContext,
    LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PointerBoundaryKind,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetMeasure, children,
    row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext, TraceRecordKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Observation {
    Boundary {
        widget: &'static str,
        kind: PointerBoundaryKind,
        phase: EventPhase,
        original: MountedNodeId,
        current: MountedNodeId,
        related: Option<MountedNodeId>,
        event_related: Option<MountedNodeId>,
        cancelable: bool,
    },
    Pointer {
        widget: &'static str,
        phase: EventPhase,
        pointer_phase: PointerPhase,
    },
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
}

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The fixture has at most 3 mounted owners and no initial output.
        runenui_core::ProducerBudget::new(3, 0)
            .with_reconciliation(3, 0, 3)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 0)
            .with_reconciliation(3, 3, 3)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Event output budgets sum to 0 across the mounted owners. Bound
        // ordinary/boundary route passes and four focus passes, plus four
        // activation/default outputs. Owner reconciliation and the cursor
        // service outcome/cancel pair stay inside the same finite frontier.
        runenui_core::ProducerBudget::new(10, 4)
            .with_application_outputs(0)
            .with_reconciliation(3, 3, 3)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 1))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 1))
    }

    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        row(children![
            Element::new(BoundaryProbe {
                name: "left",
                observations: Rc::clone(&state.observations),
            })
            .id("left")
            .key("left"),
            Element::new(BoundaryProbe {
                name: "right",
                observations: Rc::clone(&state.observations),
            })
            .id("right")
            .key("right"),
        ])
        .id("root")
        .key("root")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

#[derive(Debug)]
struct BoundaryProbe {
    name: &'static str,
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<()> for BoundaryProbe {
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

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::PointerBoundary(boundary) => {
                self.observations.borrow_mut().push(Observation::Boundary {
                    widget: self.name,
                    kind: boundary.kind(),
                    phase: context.phase(),
                    original: context.original_target().clone(),
                    current: context.current_target().clone(),
                    related: context.related_target().cloned(),
                    event_related: boundary.related_target().cloned(),
                    cancelable: context.default_is_cancelable(),
                });
                context.stop_propagation();
                context.prevent_default();
            }
            UiEvent::Pointer(pointer) if pointer.phase() == PointerPhase::Move => {
                self.observations.borrow_mut().push(Observation::Pointer {
                    widget: self.name,
                    phase: context.phase(),
                    pointer_phase: pointer.phase(),
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
    left: MountedNodeId,
    right: MountedNodeId,
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    outside_point: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
}

fn harness() -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(96.0, 48.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&style_environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"));
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
    let left_bounds = left_node.bounds();
    let right_bounds = right_node.bounds();
    let left_point = LogicalPoint::new(
        left_bounds.x() + left_bounds.width() / 2.0,
        left_bounds.y() + left_bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let right_point = LogicalPoint::new(
        right_bounds.x() + right_bounds.width() / 2.0,
        right_bounds.y() + right_bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let outside_point = LogicalPoint::new(size.width() + 1.0, size.height() + 1.0)
        .unwrap_or_else(|_| unreachable!("the outside point is finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        left: left_node.id().clone(),
        right: right_node.id().clone(),
        left_point,
        right_point,
        outside_point,
        observations,
    }
}

fn pointer_move(context: &SurfaceInputContext, point: LogicalPoint) -> PointerEvent {
    PointerEvent::new(
        PointerId::new(41).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        context.clone(),
    )
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    let report = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(report.is_quiescent());
}

#[test]
fn boundary_bundle_is_target_only_ordered_and_precedes_the_ordinary_move() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_move(&harness.context, harness.left_point),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [
            Observation::Boundary {
                widget: "left",
                kind: PointerBoundaryKind::Enter,
                phase: EventPhase::Target,
                original: harness.left.clone(),
                current: harness.left.clone(),
                related: None,
                event_related: None,
                cancelable: false,
            },
            Observation::Pointer {
                widget: "left",
                phase: EventPhase::Target,
                pointer_phase: PointerPhase::Move,
            },
        ]
    );

    harness.observations.borrow_mut().clear();
    submit_and_pump(
        &mut harness.runtime,
        pointer_move(&harness.context, harness.right_point),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [
            Observation::Boundary {
                widget: "left",
                kind: PointerBoundaryKind::Leave,
                phase: EventPhase::Target,
                original: harness.left.clone(),
                current: harness.left.clone(),
                related: Some(harness.right.clone()),
                event_related: Some(harness.right.clone()),
                cancelable: false,
            },
            Observation::Boundary {
                widget: "right",
                kind: PointerBoundaryKind::Enter,
                phase: EventPhase::Target,
                original: harness.right.clone(),
                current: harness.right.clone(),
                related: Some(harness.left.clone()),
                event_related: Some(harness.left.clone()),
                cancelable: false,
            },
            Observation::Pointer {
                widget: "right",
                phase: EventPhase::Target,
                pointer_phase: PointerPhase::Move,
            },
        ]
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RouteSnapshotCreated { invocations: 5 }
    )));
}

#[test]
#[allow(clippy::assert_is_empty)]
fn leaving_the_surface_delivers_leaves_without_a_fake_ordinary_route() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_move(&harness.context, harness.right_point),
    );
    harness.observations.borrow_mut().clear();

    submit_and_pump(
        &mut harness.runtime,
        pointer_move(&harness.context, harness.outside_point),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation::Boundary {
            widget: "right",
            kind: PointerBoundaryKind::Leave,
            phase: EventPhase::Target,
            original: harness.right.clone(),
            current: harness.right.clone(),
            related: None,
            event_related: None,
            cancelable: false,
        }]
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RouteSnapshotCreated { invocations: 2 }
    )));

    harness.observations.borrow_mut().clear();
    submit_and_pump(
        &mut harness.runtime,
        pointer_move(&harness.context, harness.outside_point),
    );
    assert!(harness.observations.borrow().is_empty());
}
