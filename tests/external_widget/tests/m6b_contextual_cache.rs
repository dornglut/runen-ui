#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Color, Element, LogicalLength, LogicalRect, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, PaintPrimitive, UiApp, View, Widget,
    WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePhase,
    SurfacePublication,
};

#[derive(Clone, Copy, Debug)]
enum Shade {
    Black,
    White,
}

#[derive(Debug)]
struct State {
    shade: Shade,
    paint_calls: Rc<Cell<usize>>,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    White,
}

#[derive(Debug)]
struct PaintProbe {
    paint_calls: Rc<Cell<usize>>,
}

impl Widget<Action> for PaintProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(10_u16),
            height: LogicalLength::from(10_u16),
        }
    }

    fn paint(&self, _: &Self::State, context: PaintContributionContext) -> PaintContribution {
        self.paint_calls.set(self.paint_calls.get() + 1);
        let Some(color) = context.computed_style().background() else {
            return PaintContribution::empty();
        };
        let size = context.local_size();
        let rect =
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height()).unwrap_or_else(|_| {
                unreachable!("validated local size yields a valid paint rectangle")
            });
        PaintContribution::single(PaintContributionItem::fill_rect(rect, color))
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let background = match state.shade {
            Shade::Black => Color::BLACK,
            Shade::White => Color::WHITE,
        };
        Element::new(PaintProbe {
            paint_calls: Rc::clone(&state.paint_calls),
        })
        .background(background)
        .key("paint-context")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::White => state.shade = Shade::White,
        }
    }
}

fn publish(runtime: &mut AppRuntime<App>, context: &SurfaceBuildContext<'_>) -> SurfacePublication {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("contextual paint publication is admitted"))
}

fn scene_color(publication: &SurfacePublication) -> Color {
    let item = publication
        .paint_scene()
        .items()
        .first()
        .unwrap_or_else(|| unreachable!("paint probe contributes one item"));
    match item.primitive() {
        PaintPrimitive::FillRect { color, .. } => *color,
        _ => unreachable!("paint probe contributes a fill rectangle"),
    }
}

#[test]
fn paint_contribution_cache_is_keyed_by_exact_owner_visible_context() {
    let paint_calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        shade: Shade::Black,
        paint_calls: Rc::clone(&paint_calls),
    });
    let tokens = runenui_core::StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());

    let first = publish(&mut runtime, &context);
    assert_eq!(scene_color(&first), Color::BLACK);
    assert_eq!(paint_calls.get(), 1);

    runtime
        .submit_action(Action::White)
        .unwrap_or_else(|_| unreachable!("style change action is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let changed = publish(&mut runtime, &context);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Style, SurfacePhase::Paint]
    );
    assert_eq!(scene_color(&changed), Color::WHITE);
    assert_eq!(paint_calls.get(), 2);

    let unchanged = publish(&mut runtime, &context);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_eq!(scene_color(&unchanged), Color::WHITE);
    assert_eq!(paint_calls.get(), 2);
}
