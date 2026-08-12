#![allow(refining_impl_trait)]
#![cfg(feature = "internal-test-seams")]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Element, LogicalLength, NoHostProtocol, StyleTokens, UiApp, Widget, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PublishSurfaceError, PumpBudget, RuntimeStatus,
    RuntimeTerminalReason, SurfaceBuildContext, SurfacePublicationCounter, TraceRecordKind,
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
        })
        .key(if state.replaced {
            "replacement"
        } else {
            "initial"
        })
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.replaced = true;
    }
}

#[derive(Debug)]
struct MeasureProbe {
    calls: Rc<Cell<usize>>,
}

impl Widget<Replace> for MeasureProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        self.calls.set(self.calls.get() + 1);
        WidgetMeasure::Fixed {
            width: LogicalLength::from(16_u16),
            height: LogicalLength::from(16_u16),
        }
    }
}

fn full_budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn published_count(runtime: &AppRuntime<App>) -> usize {
    runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::SurfacePublished))
        .count()
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
    let tokens = StyleTokens::default();
    let first = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("initial publication is admitted"));
    assert!(measure_calls.get() > 0);
    assert!(runtime.pump(full_budget()).is_quiescent());
    runtime
        .submit_action(Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted"));
    assert_eq!(runtime.pump(full_budget()).processed_envelopes(), 1);
    (runtime, measure_calls, first.input_context().clone())
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

    let tokens = StyleTokens::default();
    let result = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::unbounded(),
    ));

    assert!(matches!(
        result,
        Err(PublishSurfaceError::Terminal(
            RuntimeTerminalReason::SurfacePublicationCounterExhausted(
                SurfacePublicationCounter::HitTestGeneration,
            )
        ))
    ));
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

    let tokens = StyleTokens::default();
    let result = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::unbounded(),
    ));

    assert!(matches!(
        result,
        Err(PublishSurfaceError::Terminal(
            RuntimeTerminalReason::SurfacePublicationCounterExhausted(
                SurfacePublicationCounter::CoordinateRevision,
            )
        ))
    ));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::SurfacePublicationCounterExhausted(
            SurfacePublicationCounter::CoordinateRevision,
        ))
    );
    assert_eq!(measure_calls.get(), calls_before);
    assert_eq!(published_count(&runtime), published_before);
}
