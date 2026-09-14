#![allow(refining_impl_trait)]
#![allow(
    clippy::float_cmp,
    reason = "M9 reduced-motion endpoint proofs require exact accepted endpoint identity"
)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, StylePreferencePolicy, StylePreferences, StyleProperties, TimelineSpec,
    UiApp, UnitInterval, View, text,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePublication};

struct HoldInitialApp;
struct SnapToEndApp;
struct PreservedApp;

macro_rules! timeline_app {
    ($app:ty, $strategy:expr) => {
        impl UiApp for $app {
            type State = ();
            type Action = ();
            type HostProtocol = NoHostProtocol;

            fn root((): &Self::State) -> Element<Self::Action> {
                text("preference motion")
                    .key("root")
                    .timeline(opacity_timeline($strategy))
                    .into_element()
            }

            fn update((): &mut Self::State, (): Self::Action) {}
        }
    };
}

timeline_app!(HoldInitialApp, ReducedMotionStrategy::HoldInitial);
timeline_app!(SnapToEndApp, ReducedMotionStrategy::SnapToEnd);
timeline_app!(PreservedApp, ReducedMotionStrategy::PreserveEssential);

fn opacity_timeline(strategy: ReducedMotionStrategy) -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Opacity(SceneOpacity::TRANSPARENT),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(strategy),
    )
    .unwrap_or_else(|_| unreachable!("preference proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("fade")
            .unwrap_or_else(|_| unreachable!("preference proof animation id is valid")),
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
        .unwrap_or_else(|_| unreachable!("preference motion publication is admitted"))
}

fn root_opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("preference motion proof has a root"))
        .computed_style()
        .opacity()
        .get()
}

fn reduced_motion() -> StyleEnvironment {
    StyleEnvironment::default().with_preferences(StylePreferences::new(false, true))
}

fn high_contrast_opacity_override() -> StyleEnvironment {
    StyleEnvironment::default()
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_opacity(SceneOpacity::OPAQUE)),
        )
}

#[test]
fn hold_initial_accrues_no_hidden_time_and_restarts_when_reduced_motion_lifts() {
    let mut runtime = AppRuntime::<HoldInitialApp>::mount(());
    let reduced = reduced_motion();

    let initial = publish(&mut runtime, &reduced);
    assert_eq!(root_opacity(&initial), 0.0);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded reduced-motion advance is representable"));
    let held = publish(&mut runtime, &reduced);
    assert_eq!(root_opacity(&held), 0.0);

    let normal = StyleEnvironment::default();
    let restarted = publish(&mut runtime, &normal);
    assert_eq!(
        root_opacity(&restarted),
        0.0,
        "lifting reduced motion must restart HoldInitial from the preference-change instant"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded restarted advance is representable"));
    let middle = publish(&mut runtime, &normal);
    assert!((root_opacity(&middle) - 0.5).abs() <= f32::EPSILON);
}

#[test]
fn snap_to_end_commits_the_terminal_sample_without_later_restart() {
    let mut runtime = AppRuntime::<SnapToEndApp>::mount(());
    let reduced = reduced_motion();

    let snapped = publish(&mut runtime, &reduced);
    assert_eq!(root_opacity(&snapped), 1.0);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded post-snap advance is representable"));
    let still_snapped = publish(&mut runtime, &reduced);
    assert_eq!(root_opacity(&still_snapped), 1.0);

    let normal = StyleEnvironment::default();
    let retained = publish(&mut runtime, &normal);
    assert_eq!(
        root_opacity(&retained),
        1.0,
        "a completed SnapToEnd declaration must remain completed when reduced motion lifts"
    );
}

#[test]
fn high_contrast_suppression_keeps_the_underlying_timeline_on_the_same_clock() {
    let mut runtime = AppRuntime::<PreservedApp>::mount(());
    let high_contrast = high_contrast_opacity_override();

    let suppressed = publish(&mut runtime, &high_contrast);
    assert_eq!(root_opacity(&suppressed), 1.0);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded suppressed advance is representable"));
    let still_suppressed = publish(&mut runtime, &high_contrast);
    assert_eq!(root_opacity(&still_suppressed), 1.0);

    let normal = StyleEnvironment::default();
    let revealed = publish(&mut runtime, &normal);
    assert!(
        (root_opacity(&revealed) - 0.5).abs() <= f32::EPSILON,
        "lifting high contrast must reveal the current same-clock sample rather than restart the timeline"
    );
}
