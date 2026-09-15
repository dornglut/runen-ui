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
    WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeStatus, SurfaceBuildContext, TracePointerRejection,
    TraceRecordKind,
};

#[derive(Clone)]
struct State {
    show_old: bool,
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
        let children = if state.show_old {
            children![
                Element::new(Probe {
                    observations: Rc::clone(&state.old_observations),
                })
                .id("old.target")
                .key("old.target"),
            ]
        } else {
            children![
                Element::new(Probe {
                    observations: Rc::clone(&state.replacement_observations),
                })
                .id("replacement.target")
                .key("replacement.target"),
            ]
        };
        row(children).id("root").key("root")
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
    point: LogicalPoint,
    old_observations: Rc<RefCell<Vec<PointerPhase>>>,
    replacement_observations: Rc<RefCell<Vec<PointerPhase>>>,
    activations: Rc<Cell<usize>>,
}

fn harness() -> Harness {
    let old_observations = Rc::new(RefCell::new(Vec::new()));
    let replacement_observations = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_old: true,
        old_observations: Rc::clone(&old_observations),
        replacement_observations: Rc::clone(&replacement_observations),
        activations: Rc::clone(&activations),
    });
    pump_all(&mut runtime);

    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("fixture surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|error| unreachable!("fixture publication is valid: {error:?}"));
    let old_authored =
        ElementId::new("old.target").unwrap_or_else(|_| unreachable!("fixture id is valid"));
    let old = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&old_authored))
        .unwrap_or_else(|| unreachable!("old target is published"));
    let bounds = old.bounds();
    let point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));

    Harness {
        runtime,
        context: publication.input_context().clone(),
        point,
        old_observations,
        replacement_observations,
        activations,
    }
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

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("fixture pointer ingress is accepted"));
    pump_all(runtime);
}

fn replace_target_without_publishing(harness: &mut Harness) {
    harness
        .runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is accepted"));
    pump_all(&mut harness.runtime);
    assert!(!harness.runtime.state().show_old);
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
        pointer_event(81, PointerPhase::Down, &harness.context, harness.point),
    );

    assert_eq!(harness.runtime.status(), RuntimeStatus::Running);
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
    assert!(!harness.runtime.trace().records().skip(trace_start).any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
                if pointer_id.get() == 81
        )
    }));

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            81,
            PointerPhase::Cancel,
            &harness.context,
            harness.point,
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
        pointer_event(83, PointerPhase::Down, &harness.context, harness.point),
    );
    replace_target_without_publishing(&mut harness);
    let trace_start = harness.runtime.trace().len();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(83, PointerPhase::Up, &harness.context, harness.point),
    );

    assert_eq!(harness.runtime.status(), RuntimeStatus::Running);
    assert!(harness.old_observations.borrow().is_empty());
    assert!(harness.replacement_observations.borrow().is_empty());
    assert_eq!(harness.activations.get(), 0);
    assert!(has_pointer_rejection(
        &harness.runtime,
        trace_start,
        83,
        PointerPhase::Up,
        TracePointerRejection::NoTarget,
    ));
    assert!(harness.runtime.trace().records().skip(trace_start).any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::PointerIntegrityCleanupCommitted
        )
    }));
    assert!(harness.runtime.trace().records().skip(trace_start).any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 83
        )
    }));

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            83,
            PointerPhase::Cancel,
            &harness.context,
            harness.point,
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
