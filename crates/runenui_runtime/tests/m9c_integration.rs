#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Brush, Color, CommandOrigin, ContributionClip, Element, ElementId,
    ExplicitTimeline, HitContribution, HitContributionContext, HitRegion, LogicalLength,
    LogicalPoint, LogicalRect, LogicalTransform, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, PaintPrimitive, PointerButton, PointerButtons, PointerDeviceKind,
    PointerEvent, PointerId, PointerPhase, PresentationOrigin, PresentationRotation,
    PresentationScale, PresentationTransform, PresentationTranslation, ReducedMotionStrategy,
    SceneOpacity, SceneShape, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, StyleEnvironment, StyleInteractionState,
    StyleProperties, StyleRecipe, StyleRecipeId, StyleTheme, StyleTokens, TimelineSpec,
    TransitionSpec, UiApp, UnitInterval, View, Widget, WidgetActivation, WidgetMeasure,
    WidgetMeasureInput, children, row,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext,
    SurfacePublication,
};

const ORIGIN_COLOR: Color = Color::rgb(0xC0, 0x20, 0x20);
const MIDDLE_COLOR: Color = Color::rgb(0x20, 0xC0, 0x20);
const END_COLOR: Color = Color::rgb(0x20, 0x20, 0xC0);

#[derive(Clone, Copy, Debug)]
struct Probe {
    name: &'static str,
    color: Color,
}

impl Widget<()> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let shape = SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0));
        let clip = ContributionClip::new(
            SceneShape::rect(rect(0.0, 0.0, 18.0, 18.0)),
            translation(2.0, 0.0),
        );
        PaintContribution::single(
            PaintContributionItem::fill(shape, Brush::solid(self.color))
                .with_transform(translation(3.0, 0.0))
                .with_clip(clip),
        )
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let clip = ContributionClip::new(
            SceneShape::rect(rect(0.0, 0.0, 18.0, 18.0)),
            translation(2.0, 0.0),
        );
        HitContribution::new(vec![
            HitRegion::from_shape(SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)))
                .with_transform(translation(4.0, 0.0))
                .with_clip(clip),
        ])
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button).with_name(self.name),
        )
    }
}

struct PresentationIntegrationApp;

impl UiApp for PresentationIntegrationApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        row(children![
            Element::new(Probe {
                name: "origin",
                color: ORIGIN_COLOR,
            })
            .id("origin"),
            Element::new(Probe {
                name: "middle",
                color: MIDDLE_COLOR,
            })
            .id("middle")
            .timeline(presentation_timeline()),
            Element::new(Probe {
                name: "end",
                color: END_COLOR,
            })
            .id("end"),
        ])
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("controlled point is finite"))
}

fn translation(x: f32, y: f32) -> LogicalTransform {
    LogicalTransform::translation(x, y)
        .unwrap_or_else(|_| unreachable!("controlled translation is finite"))
}

const fn identity_presentation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::ZERO,
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

fn translated_presentation(x: f32) -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(x, 0.0)
            .unwrap_or_else(|_| unreachable!("controlled presentation translation is finite")),
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

fn presentation_timeline() -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Presentation(Some(identity_presentation())),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Presentation(Some(translated_presentation(100.0))),
            ),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("presentation integration timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("move-middle")
            .unwrap_or_else(|_| unreachable!("presentation animation id is valid")),
        spec,
    )
}

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled M9C publication is admitted"))
}

fn authored_node<'a>(
    publication: &'a SurfacePublication,
    authored: &str,
) -> &'a runenui_runtime::SurfaceNode {
    let authored =
        ElementId::new(authored).unwrap_or_else(|_| unreachable!("controlled id is canonical"));
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("authored integration node is published"))
}

fn paint_item_for_color(
    publication: &SurfacePublication,
    color: Color,
) -> &runenui_runtime::PaintSceneItem {
    publication
        .paint_scene()
        .items()
        .iter()
        .find(|item| {
            matches!(
                item.primitive(),
                PaintPrimitive::Fill {
                    brush: Brush::Solid(actual),
                    ..
                } if *actual == color
            )
        })
        .unwrap_or_else(|| unreachable!("probe paint item is published"))
}

fn hit_region_for<'a>(
    publication: &'a SurfacePublication,
    target: &MountedNodeId,
) -> &'a runenui_runtime::HitTestRegion {
    publication
        .hit_test_scene()
        .regions()
        .iter()
        .find(|region| region.target() == target)
        .unwrap_or_else(|| unreachable!("probe hit region is published"))
}

fn semantic_bounds(publication: &SurfacePublication, name: &str) -> LogicalRect {
    publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some(name))
        .unwrap_or_else(|| unreachable!("probe semantic node is published"))
        .bounds()
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-4,
        "expected {expected}, got {actual}"
    );
}

fn focus_command(
    runtime: &mut AppRuntime<PresentationIntegrationApp>,
    target: MountedNodeId,
    command: SemanticCommand,
) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("focus command is admitted"));
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

#[test]
fn sampled_presentation_correlates_paint_hit_focus_semantics_and_clips() {
    let mut runtime = AppRuntime::<PresentationIntegrationApp>::mount(());
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    let origin = authored_node(&initial, "origin").id().clone();
    let middle = authored_node(&initial, "middle").id().clone();
    let end = authored_node(&initial, "end").id().clone();

    let middle_layout = authored_node(&initial, "middle").bounds();
    assert_near(middle_layout.x(), 20.0);
    assert_near(semantic_bounds(&initial, "middle").x(), 20.0);

    focus_command(&mut runtime, origin.clone(), SemanticCommand::RequestFocus);
    focus_command(&mut runtime, origin.clone(), SemanticCommand::FocusRight);
    assert_eq!(runtime.focus().focused_node(), Some(&middle));

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded presentation advance is valid"));
    let middle_sample = publish(&mut runtime, &environment);

    assert_near(authored_node(&middle_sample, "middle").bounds().x(), 20.0);
    assert_near(semantic_bounds(&middle_sample, "middle").x(), 70.0);

    let paint = paint_item_for_color(&middle_sample, MIDDLE_COLOR);
    let [_, _, _, _, paint_x, paint_y] = paint.local_to_surface().components();
    assert_near(paint_x, 73.0);
    assert_near(paint_y, 0.0);
    assert_eq!(paint.clips().len(), 1);
    let [_, _, _, _, paint_clip_x, paint_clip_y] = paint.clips()[0].clip_to_surface().components();
    assert_near(paint_clip_x, 72.0);
    assert_near(paint_clip_y, 0.0);

    let hit = hit_region_for(&middle_sample, &middle);
    let [_, _, _, _, hit_x, hit_y] = hit.local_to_surface().components();
    assert_near(hit_x, 74.0);
    assert_near(hit_y, 0.0);
    assert_eq!(hit.clips().len(), 1);
    let [_, _, _, _, hit_clip_x, hit_clip_y] = hit.clips()[0].clip_to_surface().components();
    assert_near(hit_clip_x, 72.0);
    assert_near(hit_clip_y, 0.0);

    assert_eq!(
        middle_sample.hit_test_scene().target_at(point(76.0, 5.0)),
        Some(&middle)
    );
    assert_ne!(
        middle_sample.hit_test_scene().target_at(point(26.0, 5.0)),
        Some(&middle),
        "presentation motion must not retain the old untransformed hit region"
    );

    focus_command(&mut runtime, origin.clone(), SemanticCommand::RequestFocus);
    focus_command(&mut runtime, origin, SemanticCommand::FocusRight);
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&end),
        "directional focus must use the same sampled presentation AABB as semantics"
    );
}

struct SingularPresentationApp;

impl UiApp for SingularPresentationApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(Probe {
            name: "singular",
            color: MIDDLE_COLOR,
        })
        .id("singular")
        .timeline(singular_timeline())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn singular_timeline() -> ExplicitTimeline {
    let singular = PresentationTransform::new(
        PresentationTranslation::ZERO,
        PresentationScale::new(0.0, 1.0)
            .unwrap_or_else(|_| unreachable!("zero presentation scale is accepted")),
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    );
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Presentation(Some(identity_presentation())),
            ),
            MotionKeyframe::new(UnitInterval::ONE, MotionValue::Presentation(Some(singular))),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("singular integration timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("singular")
            .unwrap_or_else(|_| unreachable!("singular animation id is valid")),
        spec,
    )
}

#[test]
fn singular_sample_preserves_empty_hit_and_noninvertible_diagnostics() {
    let mut runtime = AppRuntime::<SingularPresentationApp>::mount(());
    let environment = StyleEnvironment::default();
    let _ = publish(&mut runtime, &environment);
    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("bounded singular advance is valid"));
    let publication = publish(&mut runtime, &environment);
    let node = authored_node(&publication, "singular");
    let region = hit_region_for(&publication, node.id());

    assert!(region.local_to_surface().inverse().is_none());
    assert_eq!(
        publication.hit_test_scene().target_at(point(4.0, 4.0)),
        None,
        "singular presentation must not fall back to stale layout geometry"
    );
    assert!(
        node.diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "runenui.scene.hit-transform-non-invertible")
    );
}

struct InteractionTransitionApp;

impl UiApp for InteractionTransitionApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(Probe {
            name: "interactive",
            color: MIDDLE_COLOR,
        })
        .id("interactive")
        .recipe(interaction_recipe_id())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn interaction_recipe_id() -> StyleRecipeId {
    StyleRecipeId::from_static("m9c-interaction")
        .unwrap_or_else(|_| unreachable!("interaction recipe id is valid"))
}

fn transition_spec() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("interaction transition spec is valid"))
}

fn interaction_environment() -> StyleEnvironment {
    let mut recipe = StyleRecipe::new(
        StyleProperties::EMPTY
            .with_opacity(SceneOpacity::OPAQUE)
            .with_transition(MotionTarget::Opacity, transition_spec()),
    );
    recipe
        .define_interaction(
            StyleInteractionState::Hover,
            StyleProperties::EMPTY.with_opacity(
                SceneOpacity::new(0.8)
                    .unwrap_or_else(|_| unreachable!("hover opacity is normalized")),
            ),
        )
        .unwrap_or_else(|_| unreachable!("hover layer is defined once"));
    recipe
        .define_interaction(
            StyleInteractionState::Focus,
            StyleProperties::EMPTY.with_opacity(
                SceneOpacity::new(0.6)
                    .unwrap_or_else(|_| unreachable!("focus opacity is normalized")),
            ),
        )
        .unwrap_or_else(|_| unreachable!("focus layer is defined once"));
    recipe
        .define_interaction(
            StyleInteractionState::Active,
            StyleProperties::EMPTY.with_opacity(
                SceneOpacity::new(0.2)
                    .unwrap_or_else(|_| unreachable!("active opacity is normalized")),
            ),
        )
        .unwrap_or_else(|_| unreachable!("active layer is defined once"));

    let mut theme = StyleTheme::new(StyleTokens::default());
    theme
        .define_recipe(interaction_recipe_id(), recipe)
        .unwrap_or_else(|_| unreachable!("interaction recipe is defined once"));
    StyleEnvironment::new(theme)
}

fn interaction_opacity(publication: &SurfacePublication) -> f32 {
    authored_node(publication, "interactive")
        .computed_style()
        .opacity()
        .get()
}

fn pump_all<App: UiApp>(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

#[test]
fn canonical_hover_focus_and_active_facts_retarget_one_transition_path() {
    let mut runtime = AppRuntime::<InteractionTransitionApp>::mount(());
    let environment = interaction_environment();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("interaction surface size is finite"));
    let context = SurfaceBuildContext::tight(&environment, size);
    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("interaction initial publication is admitted"));
    assert_near(interaction_opacity(&initial), 1.0);

    let target = authored_node(&initial, "interactive").id().clone();
    let point = point(5.0, 5.0);
    let hover_input_context = initial.input_context().clone();
    let pointer_id = PointerId::new(1).unwrap_or_else(|| unreachable!("pointer id is non-zero"));

    runtime
        .submit_pointer(PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            point,
            hover_input_context,
        ))
        .unwrap_or_else(|_| unreachable!("hover ingress is admitted"));
    pump_all(&mut runtime);
    let hover_start = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("hover start publication is admitted"));
    assert_near(interaction_opacity(&hover_start), 1.0);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("hover midpoint advance is valid"));
    let hover_midpoint = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("hover midpoint publication is admitted"));
    assert_near(interaction_opacity(&hover_midpoint), 0.9);

    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is admitted"));
    pump_all(&mut runtime);
    let focus_start = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("focus replacement publication is admitted"));
    assert_near(interaction_opacity(&focus_start), 0.9);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("focus midpoint advance is valid"));
    let focus_midpoint = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("focus midpoint publication is admitted"));
    assert_near(interaction_opacity(&focus_midpoint), 0.75);
    let down_input_context = focus_midpoint.input_context().clone();

    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        down_input_context,
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("active ingress is admitted"));
    pump_all(&mut runtime);
    let active_start = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("active replacement publication is admitted"));
    assert_near(interaction_opacity(&active_start), 0.75);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("active midpoint advance is valid"));
    let active_midpoint = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("active midpoint publication is admitted"));
    assert_near(interaction_opacity(&active_midpoint), 0.475);
    let up_input_context = active_midpoint.input_context().clone();

    let up = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        point,
        up_input_context,
    )
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("active release ingress is admitted"));
    pump_all(&mut runtime);
    let release_start = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("active release publication is admitted"));
    assert_near(interaction_opacity(&release_start), 0.475);
}
