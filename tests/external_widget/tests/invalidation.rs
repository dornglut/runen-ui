#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    ChildLayout, ChildLayoutWidget, Element, NoHostProtocol, SemanticContribution,
    SemanticContributionContext, StyleTokens, UiApp, View, Widget, WidgetActivation,
    WidgetDiagnostic, WidgetInvalidation, WidgetMeasure, WidgetPaintProof, WidgetUpdateContext,
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

fn context(tokens: &StyleTokens) -> SurfaceBuildContext<'_> {
    SurfaceBuildContext::new(tokens, LayoutConstraints::unbounded())
}

#[derive(Default, Debug)]
struct Calls {
    activation: Cell<usize>,
    measure: Cell<usize>,
    layout: Cell<usize>,
    paint: Cell<usize>,
    semantics: Cell<usize>,
    diagnostics: Cell<usize>,
}

#[derive(Debug)]
struct InvalidationProbe {
    calls: Rc<Calls>,
    invalidation: WidgetInvalidation,
}
impl Widget<()> for InvalidationProbe {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn update(&self, (): &mut Self::State, context: &mut WidgetUpdateContext) {
        context.invalidate(self.invalidation);
    }
    fn activation(&self, (): &Self::State) -> WidgetActivation {
        self.calls.activation.set(self.calls.activation.get() + 1);
        WidgetActivation::NONE
    }
    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        self.calls.measure.set(self.calls.measure.get() + 1);
        WidgetMeasure::default()
    }
    fn paint(&self, (): &Self::State) -> WidgetPaintProof {
        self.calls.paint.set(self.calls.paint.get() + 1);
        WidgetPaintProof::default()
    }
    fn semantics(
        &self,
        (): &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        self.calls.semantics.set(self.calls.semantics.get() + 1);
        SemanticContribution::empty()
    }
    fn diagnostics(&self, (): &Self::State) -> Vec<WidgetDiagnostic> {
        self.calls.diagnostics.set(self.calls.diagnostics.get() + 1);
        Vec::new()
    }
}
impl ChildLayoutWidget<()> for InvalidationProbe {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        self.calls.layout.set(self.calls.layout.get() + 1);
        ChildLayout::Linear {
            axis: runenui_core::Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct CacheState {
    invalidation: WidgetInvalidation,
    calls: Rc<Calls>,
}
#[derive(Clone, Copy, Debug)]
enum CacheAction {
    Invalidate(WidgetInvalidation),
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
            },
            Vec::<Element<()>>::new(),
        )
        .key("cache")
        .into_element()
        .map_action(|()| unreachable!())
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        let CacheAction::Invalidate(value) = action;
        state.invalidation = value;
    }
}

fn mounted_cache() -> (Rc<Calls>, AppRuntime<CacheApp>, StyleTokens) {
    let calls = Rc::new(Calls::default());
    let mut runtime = AppRuntime::<CacheApp>::mount(CacheState {
        invalidation: WidgetInvalidation::NONE,
        calls: Rc::clone(&calls),
    });
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    (calls, runtime, StyleTokens::new())
}

fn publish(runtime: &mut AppRuntime<CacheApp>, tokens: &StyleTokens) {
    let _ = runtime.publish_surface(&context(tokens));
}

#[test]
fn clean_and_paint_only_publication_skip_unrelated_work() {
    let (calls, mut runtime, tokens) = mounted_cache();
    publish(&mut runtime, &tokens);
    assert!(
        runtime
            .last_surface_phase_report()
            .contains(SurfacePhase::Layout)
    );
    assert_eq!(
        (
            calls.activation.get(),
            calls.measure.get(),
            calls.layout.get(),
            calls.paint.get(),
            calls.semantics.get(),
            calls.diagnostics.get(),
        ),
        (0, 1, 1, 1, 1, 1)
    );
    publish(&mut runtime, &tokens);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_eq!(calls.measure.get(), 1);

    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::PAINT),
    );
    publish(&mut runtime, &tokens);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );
    assert_eq!((calls.paint.get(), calls.measure.get()), (2, 1));
}

#[test]
fn layout_and_semantics_invalidation_execute_exact_dependencies() {
    let (calls, mut runtime, tokens) = mounted_cache();
    publish(&mut runtime, &tokens);
    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::LAYOUT),
    );
    publish(&mut runtime, &tokens);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Layout, SurfacePhase::HitTesting]
    );
    assert_eq!(
        (calls.measure.get(), calls.layout.get(), calls.paint.get()),
        (2, 2, 1)
    );

    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::SEMANTICS),
    );
    publish(&mut runtime, &tokens);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Semantics]
    );
    assert_eq!((calls.semantics.get(), calls.paint.get()), (2, 1));
}

#[test]
fn diagnostics_and_interaction_invalidation_are_operationally_isolated() {
    let (calls, mut runtime, tokens) = mounted_cache();
    publish(&mut runtime, &tokens);
    process_one(
        &mut runtime,
        CacheAction::Invalidate(WidgetInvalidation::DIAGNOSTICS),
    );
    publish(&mut runtime, &tokens);
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
    publish(&mut runtime, &tokens);
    assert!(
        !runtime
            .last_surface_phase_report()
            .contains(SurfacePhase::Layout)
    );
    assert_eq!(
        (
            calls.activation.get(),
            calls.measure.get(),
            calls.paint.get()
        ),
        (0, 1, 1)
    );
}
