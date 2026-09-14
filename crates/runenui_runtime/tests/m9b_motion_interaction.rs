#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    Element, HitContribution, HitContributionContext, LogicalLength, LogicalPoint, LogicalRect,
    MotionEasing, MotionTarget, NoHostProtocol, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, ReducedMotionStrategy, SceneOpacity, StyleEnvironment, StyleInteractionState,
    StyleProperties, StyleRecipe, StyleRecipeId, StyleTheme, StyleTokens, TransitionSpec, UiApp,
    Widget, WidgetMeasure, WidgetMeasureInput,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

struct HoverTransitionApp;

#[derive(Debug)]
struct HitProbe;

impl Widget<()> for HitProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(32_u16), LogicalLength::from(32_u16))
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rectangle"));
        HitContribution::single_rect(rect)
    }
}

impl UiApp for HoverTransitionApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(HitProbe).recipe(recipe_id())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn recipe_id() -> StyleRecipeId {
    StyleRecipeId::from_static("hover-transition")
        .unwrap_or_else(|_| unreachable!("interaction proof recipe id is valid"))
}

fn transition_spec() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("interaction proof transition spec is valid"))
}

fn environment() -> StyleEnvironment {
    let mut recipe = StyleRecipe::new(
        StyleProperties::EMPTY
            .with_opacity(SceneOpacity::OPAQUE)
            .with_transition(MotionTarget::Opacity, transition_spec()),
    );
    recipe
        .define_interaction(
            StyleInteractionState::Hover,
            StyleProperties::EMPTY.with_opacity(SceneOpacity::TRANSPARENT),
        )
        .unwrap_or_else(|_| unreachable!("interaction proof defines hover once"));
    let mut theme = StyleTheme::new(StyleTokens::default());
    theme
        .define_recipe(recipe_id(), recipe)
        .unwrap_or_else(|_| unreachable!("interaction proof defines recipe once"));
    StyleEnvironment::new(theme)
}

fn pump_all(runtime: &mut AppRuntime<HoverTransitionApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

#[test]
fn canonical_hover_style_change_starts_the_authored_transition() {
    let mut runtime = AppRuntime::<HoverTransitionApp>::mount(());
    let environment = environment();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("interaction proof surface size is finite"));
    let context = SurfaceBuildContext::tight(&environment, size);

    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial interaction proof publication is admitted"));
    let root = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("interaction proof has a root"));
    assert_eq!(root.computed_style().opacity(), SceneOpacity::OPAQUE);

    let bounds = root.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let pointer = PointerEvent::new(
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer id is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        initial.input_context().clone(),
    );
    runtime
        .submit_pointer(pointer)
        .unwrap_or_else(|_| unreachable!("hover ingress is admitted"));
    pump_all(&mut runtime);

    let started = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("hover transition start publication is admitted"));
    assert_eq!(
        started
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!("interaction proof has a root"))
            .computed_style()
            .opacity(),
        SceneOpacity::OPAQUE,
        "canonical hover changes the resolved target, but the transition starts from the previously presented opacity"
    );
    assert!(runtime.take_redraw_request().is_some());

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded interaction transition advance is valid"));
    let middle = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("hover transition midpoint publication is admitted"));
    let opacity = middle
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("interaction proof has a root"))
        .computed_style()
        .opacity()
        .get();
    assert!((opacity - 0.5).abs() <= f32::EPSILON);
}
