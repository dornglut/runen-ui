#![allow(refining_impl_trait)]
#![allow(
    clippy::float_cmp,
    reason = "M9 group/cache proofs require exact accepted identity and midpoint samples"
)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Color, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
    StylePreferencePolicy, StylePreferences, StyleProperties, TimelineSpec, UiApp, UnitInterval,
    View, text,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePublication};

struct GroupLifetimeApp;
struct EqualSampleApp;

impl UiApp for GroupLifetimeApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text("group lifetime")
            .key("root")
            .background(Color::WHITE)
            .timeline(group_lifetime_timeline())
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

impl UiApp for EqualSampleApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text("equal sample")
            .key("root")
            .timeline(equal_sample_timeline())
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn explicit_timeline(id: &'static str, keyframes: Vec<MotionKeyframe>) -> ExplicitTimeline {
    let easings = vec![MotionEasing::Linear; keyframes.len() - 1];
    let spec = TimelineSpec::new(
        keyframes,
        easings,
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("bounded group/cache proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static(id)
            .unwrap_or_else(|_| unreachable!("group/cache proof animation id is valid")),
        spec,
    )
}

fn group_lifetime_timeline() -> ExplicitTimeline {
    let middle =
        SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled midpoint is valid"));
    explicit_timeline(
        "isolate",
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
            MotionKeyframe::new(UnitInterval::HALF, MotionValue::Opacity(middle)),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
        ],
    )
}

fn equal_sample_timeline() -> ExplicitTimeline {
    explicit_timeline(
        "identity",
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
        ],
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
        .unwrap_or_else(|_| unreachable!("group/cache proof publication is admitted"))
}

fn root_opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("group/cache proof has a root"))
        .computed_style()
        .opacity()
        .get()
}

fn advance<App: UiApp>(runtime: &AppRuntime<App>, millis: u64) {
    runtime
        .advance_time(Duration::from_millis(millis))
        .unwrap_or_else(|_| unreachable!("bounded group/cache advance is representable"));
}

fn high_contrast_opaque() -> StyleEnvironment {
    StyleEnvironment::default()
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_opacity(SceneOpacity::OPAQUE)),
        )
}

#[test]
fn active_spec_retains_group_at_identity_sample_and_drops_it_at_terminal_identity() {
    let mut runtime = AppRuntime::<GroupLifetimeApp>::mount(());
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&initial), 1.0);
    assert!(
        !initial.paint_scene().groups().is_empty(),
        "an active spec with a non-identity opacity keyframe must retain isolation even when the current sample is identity"
    );

    advance(&runtime, 50);
    let middle = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&middle), 0.5);
    assert!(!middle.paint_scene().groups().is_empty());

    advance(&runtime, 50);
    let completed = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&completed), 1.0);
    assert!(
        completed.paint_scene().groups().is_empty(),
        "the exact terminal identity candidate must drop motion-only isolation when the timeline completes"
    );
}

#[test]
fn mandatory_suppression_removes_motion_group_until_the_same_clock_sample_is_revealed() {
    let mut runtime = AppRuntime::<GroupLifetimeApp>::mount(());
    let high_contrast = high_contrast_opaque();

    let initial = publish(&mut runtime, &high_contrast);
    assert_eq!(root_opacity(&initial), 1.0);
    assert!(initial.paint_scene().groups().is_empty());

    advance(&runtime, 50);
    let suppressed = publish(&mut runtime, &high_contrast);
    assert_eq!(root_opacity(&suppressed), 1.0);
    assert!(suppressed.paint_scene().groups().is_empty());

    let normal = StyleEnvironment::default();
    let revealed = publish(&mut runtime, &normal);
    assert_eq!(root_opacity(&revealed), 0.5);
    assert!(
        !revealed.paint_scene().groups().is_empty(),
        "lifting mandatory suppression must reveal the current same-clock sample and its required isolation"
    );
}

#[test]
fn equal_sample_reuses_paint_revision_while_active_motion_keeps_redraw_live() {
    let mut runtime = AppRuntime::<EqualSampleApp>::mount(());
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    let revision = initial.paint_publication().revision();
    assert!(
        runtime.take_redraw_request().is_some(),
        "active motion must request a future redraw even when its current effective sample equals accepted content"
    );

    advance(&runtime, 50);
    let middle = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&middle), 1.0);
    assert_eq!(
        middle.paint_publication().revision(),
        revision,
        "time-only advancement to equal effective paint content must reuse the exact paint revision"
    );
    assert!(
        runtime.take_redraw_request().is_some(),
        "equal sampled content does not cancel scheduling authority while motion remains active"
    );
}
