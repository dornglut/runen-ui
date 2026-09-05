#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, Element, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PaintContribution, PaintContributionContext,
    SemanticContribution, SemanticContributionContext, StyleEnvironment, UiApp, View, Widget,
    WidgetActivation, WidgetDiagnostic, WidgetInvalidation, WidgetMeasure, WidgetUpdateContext,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePhase,
};

fn process_one<App: UiApp>(runtime: &mut AppRuntime<App>, action: App::Action) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

const fn context(environment: &StyleEnvironment) -> SurfaceBuildContext<'_> {
    SurfaceBuildContext::new(environment, LayoutConstraints::unbounded())
}

#[derive(Default, Debug)]
struct Calls {
    activation: Cell<usize>,
    measure: Cell<usize>,
    paint: Cell<usize>,
    hit_test: Cell<usize>,
    semantics: Cell<usize>,
    diagnostics: Cell<usize>,
}

#[derive(Debug)]
struct ProbeState {
    hit_enabled: bool,
}

#[derive(Debug)]
struct InvalidationProbe {
    calls: Rc<Calls>,
    invalidation: WidgetInvalidation,
    hit_enabled: bool,
}

impl Widget<()> for InvalidationProbe {
    type State = ProbeState;

    fn create_state(&self) -> Self::State {
        ProbeState {
            hit_enabled: self.hit_enabled,
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext) {
        context.invalidate(self.invalidation);
        if state.hit_enabled != self.hit_enabled {
            state.hit_enabled = self.hit_enabled;
            context.invalidate(WidgetInvalidation::HIT_TEST);
        }
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        self.calls.activation.set(self.calls.activation.get() + 1);
        WidgetActivation::NONE
    }

    fn measure(&self, _: &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        self.calls.measure.set(self.calls.measure.get() + 1);
        WidgetMeasure::measured(LogicalLength::from(10_u16), LogicalLength::from(10_u16))
    }

    fn paint(&self, _: &Self::State, _: PaintContributionContext) -> PaintContribution {
        self.calls.paint.set(self.calls.paint.get() + 1);
        PaintContribution::empty()
    }

    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        self.calls.hit_test.set(self.calls.hit_test.get() + 1);
        if state.hit_enabled {
            let size = context.local_size();
            let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rect"));
            HitContribution::single_rect(rect)
        } else {
            HitContribution::empty()
        }
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        self.calls.semantics.set(self.calls.semantics.get() + 1);
        SemanticContribution::empty()
    }

    fn diagnostics(&self, _: &Self::State) -> Vec<WidgetDiagnostic> {
        self.calls.diagnostics.set(self.calls.diagnostics.get() + 1);
        Vec::new()
    }
}

impl ChildBearingWidget<()> for InvalidationProbe {}

#[derive(Debug)]
struct CacheState {
    invalidation: WidgetInvalidation,
    hit_enabled: bool,
    calls: Rc<Calls>,
}

#[derive(Clone, Copy, Debug)]
enum CacheAction {
    Invalidate(WidgetInvalidation),
    SetHit(bool),
}

struct CacheApp;

impl UiApp for CacheApp {
    type State = CacheState;
    type Action = CacheAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        runenui_core::container(
            InvalidationProbe {
                calls: Rc::clone(&state.calls),
                invalidation: state.invalidation,
                hit_enabled: state.hit_enabled,
            },
            Vec::<Element<()>>::new(),
        )
        .key("cache")
        .into_element()
        .map_action(|()| unreachable!())
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CacheAction::Invalidate(value) => state.invalidation = value,
            CacheAction::SetHit(enabled) => {
                state.invalidation = WidgetInvalidation::NONE;
                state.hit_enabled = enabled;
            }
        }
    }
}

fn mounted_cache() -> (Rc<Calls>, AppRuntime<CacheApp>, StyleEnvironment) {
    let calls = Rc::new(Calls::default());
    let mut runtime = AppRuntime::<CacheApp>::mount(CacheState {
        invalidation: WidgetInvalidation::NONE,
        hit_enabled: false,
        calls: Rc::clone(&calls),
    });
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    (calls, runtime, StyleEnvironment::default())
}

fn publish(
    runtime: &mut AppRuntime<CacheApp>,
    environment: &StyleEnvironment,
) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&context(environment))
        .unwrap_or_else(|_| unreachable!("cache publication is admitted"))
}

#[test]
fn clean_and_paint_only_publication_skip_unrelated_work() {
    let (calls, mut runtime, environment) = mounted_cache();
    let _ = publish(&mut runtime, &environment);
    assert!(
        runtime
            .last_surface_phase_report()
            .contains(SurfacePhase::Layout)
    );
    assert_eq!(
        (
            calls.activation.get(),
            calls.measure.get(),
            calls.paint.get(),
            calls.hit_test.get(),
            calls.semantics.get(),
            calls.diagnostics.get(),
        ),
        (1, 1, 1, 1, 1, 1)
    );
    let _ = publish(&mut runtime, &environment);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_eq!(calls.measure.get(), 1);

    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::PAINT),
    );
    let _ = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );
    assert_eq!(
        (calls.paint.get(), calls.hit_test.get(), calls.measure.get()),
        (2, 1, 1)
    );
}

#[test]
fn layout_and_semantics_invalidation_execute_exact_dependencies() {
    let (calls, mut runtime, environment) = mounted_cache();
    let _ = publish(&mut runtime, &environment);
    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::LAYOUT),
    );
    let _ = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ]
    );
    assert_eq!(
        (
            calls.measure.get(),
            calls.paint.get(),
            calls.hit_test.get(),
            calls.semantics.get(),
        ),
        (2, 2, 2, 1)
    );

    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::SEMANTICS),
    );
    let _ = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Semantics]
    );
    assert_eq!(
        (
            calls.semantics.get(),
            calls.paint.get(),
            calls.hit_test.get()
        ),
        (2, 2, 2)
    );
}

#[test]
fn hit_invalidation_recomputes_only_hit_and_changes_targetability() {
    let (calls, mut runtime, environment) = mounted_cache();
    let initial = publish(&mut runtime, &environment);
    let point = LogicalPoint::new(1.0, 1.0).unwrap_or_else(|_| unreachable!());
    assert!(initial.hit_test_scene().target_at(point).is_none());
    let counts_before = (
        calls.measure.get(),
        calls.paint.get(),
        calls.hit_test.get(),
        calls.semantics.get(),
        calls.diagnostics.get(),
    );

    process_one(&mut runtime, CacheAction::SetHit(true));
    let targetable = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::HitTesting]
    );
    assert!(targetable.hit_test_scene().target_at(point).is_some());
    assert_eq!(
        (
            calls.measure.get(),
            calls.paint.get(),
            calls.hit_test.get(),
            calls.semantics.get(),
            calls.diagnostics.get(),
        ),
        (
            counts_before.0,
            counts_before.1,
            counts_before.2 + 1,
            counts_before.3,
            counts_before.4,
        )
    );

    process_one(&mut runtime, CacheAction::SetHit(false));
    let pass_through = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::HitTesting]
    );
    assert!(pass_through.hit_test_scene().target_at(point).is_none());
}

#[test]
fn diagnostics_and_interaction_invalidation_are_operationally_isolated() {
    let (calls, mut runtime, environment) = mounted_cache();
    let _ = publish(&mut runtime, &environment);
    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::DIAGNOSTICS),
    );
    let _ = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Diagnostics]
    );
    assert_eq!(calls.diagnostics.get(), 2);

    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::INTERACTION),
    );
    assert!(
        runtime
            .last_surface_phase_report()
            .contains(SurfacePhase::FocusValidation)
    );
    let _ = publish(&mut runtime, &environment);
    assert!(
        !runtime
            .last_surface_phase_report()
            .contains(SurfacePhase::Layout)
    );
    assert_eq!(
        (
            calls.activation.get(),
            calls.measure.get(),
            calls.paint.get(),
            calls.hit_test.get(),
            calls.semantics.get(),
        ),
        (2, 1, 1, 1, 1)
    );
}
