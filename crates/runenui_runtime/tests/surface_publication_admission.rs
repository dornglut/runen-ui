#![allow(refining_impl_trait)]
#![cfg(feature = "internal-test-seams")]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Element, LogicalLength, LogicalPoint, NoHostProtocol, PointerDeviceKind, PointerEvent,
    PointerId, PointerPhase, StyleEnvironment, SurfaceInputContext, UiApp, Widget, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PublishSurfaceError, PumpBudget, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, SurfaceBuildContext, SurfacePublication, SurfacePublicationCounter,
    TraceRecordKind,
};

#[derive(Clone, Copy)]
struct Replace;

#[derive(Clone)]
struct State {
    replaced: bool,
    measure_calls: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Replace;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(MeasureProbe {
            calls: Rc::clone(&state.measure_calls),
            width: if state.replaced { 32 } else { 16 },
        })
        .key("probe")
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.replaced = true;
    }
}

#[derive(Debug)]
struct MeasureProbe {
    calls: Rc<Cell<usize>>,
    width: u16,
}

impl Widget<Replace> for MeasureProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        self.calls.set(self.calls.get() + 1);
        WidgetMeasure::measured(LogicalLength::from(self.width), LogicalLength::from(16_u16))
    }
}

const fn full_budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn published_count(runtime: &AppRuntime<App>) -> usize {
    runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::SurfacePublished))
        .count()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PublicationTraceState {
    published: usize,
    stationary_rehit_queued: usize,
    redraw_taken: usize,
    redraw_acknowledged: usize,
    latest_redraw_requested: Option<u64>,
    latest_redraw_acknowledged: Option<u64>,
}

fn publication_trace_state(runtime: &AppRuntime<App>) -> PublicationTraceState {
    let mut state = PublicationTraceState::default();
    for record in runtime.trace().records() {
        match record.kind() {
            TraceRecordKind::SurfacePublished => state.published += 1,
            TraceRecordKind::PointerStationaryRehitQueued { .. } => {
                state.stationary_rehit_queued += 1;
            }
            TraceRecordKind::RedrawRequested { revision } => {
                state.latest_redraw_requested = Some(*revision);
            }
            TraceRecordKind::RedrawTaken { .. } => state.redraw_taken += 1,
            TraceRecordKind::RedrawAcknowledged { revision } => {
                state.redraw_acknowledged += 1;
                state.latest_redraw_acknowledged = Some(*revision);
            }
            _ => {}
        }
    }
    state
}

const fn has_pending_redraw(state: PublicationTraceState) -> bool {
    match (
        state.latest_redraw_requested,
        state.latest_redraw_acknowledged,
    ) {
        (Some(requested), Some(acknowledged)) => requested > acknowledged,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

fn expect_counter_exhaustion(
    result: &Result<SurfacePublication, PublishSurfaceError>,
    expected: SurfacePublicationCounter,
) {
    let expected_error = PublishSurfaceError::Terminal(
        RuntimeTerminalReason::SurfacePublicationCounterExhausted(expected),
    );
    assert_eq!(result.as_ref().err(), Some(&expected_error));
}

fn prepared_runtime() -> (
    AppRuntime<App>,
    Rc<Cell<usize>>,
    runenui_core::SurfaceInputContext,
) {
    let measure_calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        replaced: false,
        measure_calls: Rc::clone(&measure_calls),
    });
    let style_environment = StyleEnvironment::default();
    let first = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("initial publication is admitted"));
    assert!(measure_calls.get() > 0);
    assert!(runtime.pump(full_budget()).is_quiescent());
    runtime
        .submit_action(Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted"));
    let calls_before_replacement = measure_calls.get();
    assert!(runtime.pump(full_budget()).is_quiescent());
    assert!(runtime.state().replaced);
    assert_eq!(measure_calls.get(), calls_before_replacement);
    (runtime, measure_calls, first.input_context().clone())
}

fn center(publication: &SurfacePublication) -> LogicalPoint {
    let bounds = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the probe is published"))
        .bounds();
    LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds have a finite center"))
}

fn pointer_move(context: &SurfaceInputContext, point: LogicalPoint) -> PointerEvent {
    PointerEvent::new(
        PointerId::new(1).unwrap_or_else(|| unreachable!("the pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        context.clone(),
    )
}

#[test]
fn hit_test_generation_exhaustion_terminalizes_before_surface_callbacks() {
    let (mut runtime, measure_calls, previous) = prepared_runtime();
    let calls_before = measure_calls.get();
    let published_before = published_count(&runtime);
    let next_coordinate = previous
        .coordinate_revision()
        .checked_add(1)
        .unwrap_or_else(|| unreachable!("first coordinate revision has a successor"));
    runtime.__seed_next_surface_publication_counters_for_test(None, Some(next_coordinate));

    let style_environment = StyleEnvironment::default();
    let result = runtime.publish_surface(&SurfaceBuildContext::new(
        &style_environment,
        LayoutConstraints::unbounded(),
    ));

    expect_counter_exhaustion(&result, SurfacePublicationCounter::HitTestGeneration);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::SurfacePublicationCounterExhausted(
            SurfacePublicationCounter::HitTestGeneration,
        ))
    );
    assert_eq!(measure_calls.get(), calls_before);
    assert_eq!(published_count(&runtime), published_before);
}

#[test]
fn coordinate_revision_exhaustion_terminalizes_before_surface_callbacks() {
    let (mut runtime, measure_calls, previous) = prepared_runtime();
    let calls_before = measure_calls.get();
    let published_before = published_count(&runtime);
    let next_hit_test = previous
        .hit_test_generation()
        .checked_add(1)
        .unwrap_or_else(|| unreachable!("first hit-test generation has a successor"));
    runtime.__seed_next_surface_publication_counters_for_test(Some(next_hit_test), None);

    let style_environment = StyleEnvironment::default();
    let result = runtime.publish_surface(&SurfaceBuildContext::new(
        &style_environment,
        LayoutConstraints::unbounded(),
    ));

    expect_counter_exhaustion(&result, SurfacePublicationCounter::CoordinateRevision);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::SurfacePublicationCounterExhausted(
            SurfacePublicationCounter::CoordinateRevision,
        ))
    );
    assert_eq!(measure_calls.get(), calls_before);
    assert_eq!(published_count(&runtime), published_before);
}

#[test]
fn queued_pointer_rehit_backpressure_refuses_without_commit_and_retries_exactly() {
    let measure_calls = Rc::new(Cell::new(0));
    let config = RuntimeConfig::default().with_queue_capacity(1);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            replaced: false,
            measure_calls: Rc::clone(&measure_calls),
        },
        config,
    );
    let style_environment = StyleEnvironment::default();
    let build_context =
        SurfaceBuildContext::new(&style_environment, LayoutConstraints::unbounded());
    let first = runtime
        .publish_surface(&build_context)
        .unwrap_or_else(|_| unreachable!("initial publication is admitted"));
    assert!(runtime.pump(full_budget()).is_quiescent());

    runtime
        .submit_action(Replace)
        .unwrap_or_else(|_| unreachable!("the dirtying action is accepted"));
    let calls_before_update = measure_calls.get();
    assert!(runtime.pump(full_budget()).is_quiescent());
    assert!(runtime.state().replaced);
    assert_eq!(measure_calls.get(), calls_before_update);

    let point = center(&first);
    runtime
        .submit_pointer(pointer_move(first.input_context(), point))
        .unwrap_or_else(|_| unreachable!("the queue-filling pointer move is accepted"));
    let calls_before_refusal = measure_calls.get();
    let trace_before_refusal = publication_trace_state(&runtime);
    assert!(has_pending_redraw(trace_before_refusal));

    let refused = runtime.publish_surface(&build_context);

    assert_eq!(refused.as_ref().err(), Some(&PublishSurfaceError::Full));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(measure_calls.get(), calls_before_refusal);
    assert_eq!(publication_trace_state(&runtime), trace_before_refusal);
    assert!(has_pending_redraw(publication_trace_state(&runtime)));

    let filler = runtime.pump(full_budget());
    assert!(filler.is_quiescent());
    assert_eq!(filler.processed_envelopes(), 1);
    let calls_before_retry = measure_calls.get();
    let trace_before_retry = publication_trace_state(&runtime);
    assert_eq!(calls_before_retry, calls_before_refusal);
    assert!(has_pending_redraw(trace_before_retry));

    let expected_hit_test_generation = first
        .input_context()
        .hit_test_generation()
        .checked_add(1)
        .unwrap_or_else(|| unreachable!("the first hit-test generation has a successor"));
    let expected_coordinate_revision = first
        .input_context()
        .coordinate_revision()
        .checked_add(1)
        .unwrap_or_else(|| unreachable!("the first coordinate revision has a successor"));
    let retry = runtime
        .publish_surface(&build_context)
        .unwrap_or_else(|_| unreachable!("publication retries after queue capacity is freed"));

    assert_eq!(
        retry.input_context().hit_test_generation(),
        expected_hit_test_generation
    );
    assert_eq!(
        retry.input_context().coordinate_revision(),
        expected_coordinate_revision
    );
    assert!(measure_calls.get() > calls_before_retry);
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let trace_after_retry = publication_trace_state(&runtime);
    assert_eq!(
        trace_after_retry.published,
        trace_before_retry.published + 1
    );
    assert_eq!(
        trace_after_retry.redraw_taken,
        trace_before_retry.redraw_taken + 1
    );
    assert_eq!(
        trace_after_retry.redraw_acknowledged,
        trace_before_retry.redraw_acknowledged + 1
    );
    assert_eq!(
        trace_after_retry.latest_redraw_acknowledged,
        trace_after_retry.latest_redraw_requested
    );

    assert!(runtime.pump(full_budget()).is_quiescent());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}
