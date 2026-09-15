#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Effects, Element, EventContext, HitContribution, HitContributionContext, IntoEffects,
    LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TracePointerRejection,
    TraceRecordKind,
};

#[derive(Clone)]
struct State {
    observed: Rc<RefCell<Vec<PointerPhase>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            observed: Rc::clone(&state.observed),
        })
    }

    fn update(
        _state: &mut Self::State,
        _action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::redraw()
    }
}

#[derive(Debug)]
struct Probe {
    observed: Rc<RefCell<Vec<PointerPhase>>>,
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
        if let UiEvent::Pointer(pointer) = event {
            self.observed.borrow_mut().push(pointer.phase());
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
        .unwrap_or_else(|_| unreachable!("fixture pointer ingress is accepted for processing"));
    pump_all(runtime);
}

fn fixture_runtime() -> (AppRuntime<App>, Rc<RefCell<Vec<PointerPhase>>>) {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observed: Rc::clone(&observed),
    });
    pump_all(&mut runtime);
    (runtime, observed)
}

#[test]
fn displayed_context_survives_one_unpresented_publication_but_not_two() {
    let (mut runtime, observed) = fixture_runtime();

    let style = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("fixture surface size is finite"));
    let build = SurfaceBuildContext::tight(&style, size);

    let displayed = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| unreachable!("displayed publication is valid: {error:?}"));
    let displayed_context = displayed.input_context().clone();
    let bounds = displayed
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("fixture root is published"))
        .bounds();
    let inside = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));

    let pending = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| unreachable!("one pending publication is valid: {error:?}"));
    assert_eq!(
        pending.input_context().hit_test_generation(),
        displayed_context.hit_test_generation() + 1
    );

    submit_and_pump(
        &mut runtime,
        pointer_event(1, PointerPhase::Down, &displayed_context, inside),
    );
    submit_and_pump(
        &mut runtime,
        pointer_event(1, PointerPhase::Up, &displayed_context, inside),
    );
    assert_eq!(
        observed.borrow().as_slice(),
        [PointerPhase::Down, PointerPhase::Up]
    );

    observed.borrow_mut().clear();
    let _too_far_ahead = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| unreachable!("second pending publication is valid: {error:?}"));
    let trace_start = runtime.trace().len();

    submit_and_pump(
        &mut runtime,
        pointer_event(2, PointerPhase::Down, &displayed_context, inside),
    );

    assert!(observed.borrow().is_empty());
    assert!(runtime.trace().records().skip(trace_start).any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::PointerIngressRejected {
                pointer_id,
                phase: PointerPhase::Down,
                outcome: TracePointerRejection::RetiredGeneration,
            } if pointer_id.get() == 2
        )
    }));
}

#[test]
fn latest_redraw_token_acknowledges_coalesced_dirty_revisions() {
    let (mut runtime, _observed) = fixture_runtime();
    let initial = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("mount is initially publication-dirty"));
    runtime
        .acknowledge_redraw(&initial)
        .unwrap_or_else(|_| unreachable!("initial redraw request belongs to this runtime"));
    assert!(runtime.take_redraw_request().is_none());

    runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!("first redraw action is admitted"));
    pump_all(&mut runtime);
    let older = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("first redraw action arms a request"));

    runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!("second redraw action is admitted"));
    pump_all(&mut runtime);
    let latest = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("second redraw action advances the request"));
    assert!(latest.revision() > older.revision());

    runtime
        .acknowledge_redraw(&latest)
        .unwrap_or_else(|_| unreachable!("latest redraw request belongs to this runtime"));
    assert!(
        runtime.take_redraw_request().is_none(),
        "acknowledging the newest coalesced request must not force an unchanged republish"
    );
}
