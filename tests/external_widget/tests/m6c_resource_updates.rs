#![allow(refining_impl_trait)]

use runenui_core::{
    Color, Element, LogicalLength, LogicalPoint, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, ResourceKind, ResourceRef, StyleEnvironment,
    UiApp, Widget, WidgetInvalidation, WidgetMeasure, WidgetUpdateContext,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePhase,
};

fn origin() -> LogicalPoint {
    LogicalPoint::new(3.0, 4.0).unwrap_or_else(|_| unreachable!("test origin is finite"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnerState {
    shaped: ResourceRef,
    foreground: Color,
}

#[derive(Debug)]
struct ResourceOwner {
    shaped: ResourceRef,
    foreground: Color,
}

impl Widget<Action> for ResourceOwner {
    type State = OwnerState;

    fn create_state(&self) -> Self::State {
        OwnerState {
            shaped: self.shaped.clone(),
            foreground: self.foreground,
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.shaped != self.shaped || state.foreground != self.foreground {
            state.shaped = self.shaped.clone();
            state.foreground = self.foreground;
            context.invalidate(WidgetInvalidation::PAINT);
        }
    }

    fn measure(&self, _: &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, state: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::single(
            PaintContributionItem::shaped_text_run(
                state.shaped.clone(),
                origin(),
                state.foreground,
            )
            .unwrap_or_else(|_| unreachable!("fixture shaped ref has shaped-run kind")),
        )
    }
}

#[derive(Debug)]
enum Action {
    Recolor(Color),
    Replace(ResourceRef),
}

#[derive(Debug)]
struct State {
    shaped: ResourceRef,
    foreground: Color,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(ResourceOwner {
            shaped: state.shaped.clone(),
            foreground: state.foreground,
        })
        .key("resource-owner")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Recolor(foreground) => state.foreground = foreground,
            Action::Replace(shaped) => state.shaped = shaped,
        }
    }
}

fn publish(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("resource update publication is admitted"))
}

fn shaped_run(
    publication: &runenui_runtime::SurfacePublication,
) -> &runenui_core::ShapedTextRunPrimitive {
    publication.paint_scene().items()[0]
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("fixture publishes one shaped run"))
}

#[test]
fn foreground_only_change_reuses_ref_while_logical_content_replacement_uses_a_new_ref() {
    let original_ref = ResourceRef::new(ResourceKind::ShapedTextRun);
    let initial_foreground = Color::rgba(10, 20, 30, 255);
    let recolored_foreground = Color::rgba(30, 20, 10, 255);
    let mut runtime = AppRuntime::<App>::mount(State {
        shaped: original_ref.clone(),
        foreground: initial_foreground,
    });
    let environment = StyleEnvironment::default();
    let initial_pump = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(initial_pump.is_quiescent());

    let initial = publish(&mut runtime, &environment);
    let initial_scene = initial.paint_scene().clone();
    assert_eq!(shaped_run(&initial).resource_ref(), &original_ref);
    assert_eq!(shaped_run(&initial).foreground(), initial_foreground);

    runtime
        .submit_action(Action::Recolor(recolored_foreground))
        .unwrap_or_else(|_| unreachable!("recolor action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let recolored = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );
    assert_ne!(recolored.paint_scene(), &initial_scene);
    assert_eq!(shaped_run(&recolored).resource_ref(), &original_ref);
    assert_eq!(shaped_run(&recolored).foreground(), recolored_foreground);

    let replacement_ref = ResourceRef::new(ResourceKind::ShapedTextRun);
    runtime
        .submit_action(Action::Replace(replacement_ref.clone()))
        .unwrap_or_else(|_| unreachable!("resource replacement action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let replaced = publish(&mut runtime, &environment);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );
    assert_eq!(shaped_run(&replaced).resource_ref(), &replacement_ref);
    assert_ne!(shaped_run(&replaced).resource_ref(), &original_ref);
    assert_eq!(shaped_run(&replaced).foreground(), recolored_foreground);
}
