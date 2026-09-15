#![allow(refining_impl_trait)]
#![allow(
    clippy::float_cmp,
    reason = "M9C public-harness timing boundaries require exact accepted endpoint identity"
)]

use core::num::NonZeroU64;
use std::time::Duration;

use runenui_core::{
    AnimationId, Element, FontFamilyName, GenericFontFamily, LayoutDimension, LayoutStyle,
    LogicalLength, MotionEasing, MotionKeyframe, MotionRepeat, MotionTarget, MotionValue,
    NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment, StylePreferences,
    TimelineSpec, TransitionSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{LayoutConstraints, PumpBudget, SurfaceBuildContext};
use runenui_testing::TestHarness;

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

fn register_controlled_text<App: UiApp>(harness: &mut TestHarness<App>) {
    assert!(
        harness
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled Cantarell fixture is registerable"))
            > 0
    );
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled family name is canonical"));
    assert!(
        harness
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
            .unwrap_or_else(|_| unreachable!("controlled generic mapping is valid"))
    );
}

fn finite_repeat_two() -> MotionRepeat {
    MotionRepeat::finite(NonZeroU64::new(2).unwrap_or_else(|| unreachable!("two is non-zero")))
}

fn opacity_timeline() -> runenui_core::ExplicitTimeline {
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
        Duration::from_millis(50),
        finite_repeat_two(),
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("public-harness timing timeline is valid"));
    runenui_core::ExplicitTimeline::new(
        AnimationId::from_static("public-timing")
            .unwrap_or_else(|_| unreachable!("public timing id is valid")),
        spec,
    )
}

struct TimingHarnessApp;

impl UiApp for TimingHarnessApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("timing").key("timing").timeline(opacity_timeline())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn harness_opacity<App: UiApp>(harness: &TestHarness<App>) -> f32 {
    harness
        .publication()
        .and_then(|publication| publication.frame().root())
        .unwrap_or_else(|| unreachable!("public harness retains one root publication"))
        .computed_style()
        .opacity()
        .get()
}

#[test]
fn public_harness_manual_clock_preserves_delay_and_repeat_boundaries() {
    let mut harness = TestHarness::<TimingHarnessApp>::mount(());
    register_controlled_text(&mut harness);

    harness.publish().unwrap_or_else(|_| unreachable!());
    assert_eq!(harness_opacity(&harness), 0.0);

    harness
        .advance_time(Duration::from_millis(49))
        .unwrap_or_else(|_| unreachable!("bounded delay advance is valid"));
    harness.publish().unwrap_or_else(|_| unreachable!());
    assert_eq!(harness_opacity(&harness), 0.0);

    harness
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!("delay boundary is valid"));
    harness.publish().unwrap_or_else(|_| unreachable!());
    assert_eq!(harness_opacity(&harness), 0.0);

    harness
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("first repeat boundary is valid"));
    harness.publish().unwrap_or_else(|_| unreachable!());
    assert_eq!(
        harness_opacity(&harness),
        0.0,
        "the exact non-final iteration boundary restarts at keyframe zero"
    );

    harness
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("second iteration midpoint is valid"));
    harness.publish().unwrap_or_else(|_| unreachable!());
    assert!((harness_opacity(&harness) - 0.5).abs() <= f32::EPSILON);

    harness
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("final repeat boundary is valid"));
    harness.publish().unwrap_or_else(|_| unreachable!());
    assert_eq!(harness_opacity(&harness), 1.0);
}

fn width_timeline() -> runenui_core::ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Width(LayoutDimension::length(LogicalLength::from(180_u16))),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Width(LayoutDimension::length(LogicalLength::from(60_u16))),
            ),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("public structural timeline is valid"));
    runenui_core::ExplicitTimeline::new(
        AnimationId::from_static("public-width")
            .unwrap_or_else(|_| unreachable!("public width id is valid")),
        spec,
    )
}

struct StructuralHarnessApp;

impl UiApp for StructuralHarnessApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("deterministic structural motion must re-enter ordinary text layout")
            .key("structural")
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(LogicalLength::from(180_u16))),
            )
            .timeline(width_timeline())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn public_harness_structural_motion_reenters_layout_and_text_measurement() {
    let mut harness = TestHarness::<StructuralHarnessApp>::mount(());
    register_controlled_text(&mut harness);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());

    let initial = harness
        .publish_with_context(&context)
        .unwrap_or_else(|_| unreachable!("initial structural publication is admitted"));
    let initial_width = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .width();
    let initial_text = initial
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!())
        .text_measurements()
        .last()
        .unwrap_or_else(|| unreachable!("text leaf records retained measurement"))
        .measured_size();
    assert!((initial_width - 180.0).abs() <= f32::EPSILON);

    harness
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("bounded structural advance is valid"));
    let terminal = harness
        .publish_with_context(&context)
        .unwrap_or_else(|_| unreachable!("terminal structural publication is admitted"));
    let terminal_width = terminal
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .width();
    let terminal_text = terminal
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!())
        .text_measurements()
        .last()
        .unwrap_or_else(|| unreachable!("text leaf records terminal measurement"))
        .measured_size();

    assert!((terminal_width - 60.0).abs() <= f32::EPSILON);
    assert!(
        terminal_text.height() > initial_text.height(),
        "narrow sampled width must flow through Taffy into ordinary text re-line-breaking"
    );
    assert!(
        terminal
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .any(|node| node.bounds().width() <= 60.0 + f32::EPSILON),
        "semantic geometry must observe the same structural sample"
    );
}

#[derive(Clone, Copy)]
struct TransitionState {
    transparent: bool,
}

struct TransitionHarnessApp;

impl UiApp for TransitionHarnessApp {
    type State = TransitionState;
    type Action = bool;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        text("transition")
            .key("transition")
            .opacity(if state.transparent {
                SceneOpacity::TRANSPARENT
            } else {
                SceneOpacity::OPAQUE
            })
            .transition(
                MotionTarget::Opacity,
                TransitionSpec::new(
                    Duration::from_millis(100),
                    Duration::ZERO,
                    MotionEasing::Linear,
                    Some(ReducedMotionStrategy::SnapToEnd),
                )
                .unwrap_or_else(|_| unreachable!("public transition spec is valid")),
            )
            .into_element()
    }

    fn update(state: &mut Self::State, transparent: Self::Action) {
        state.transparent = transparent;
    }
}

fn pump_actions(harness: &mut TestHarness<TransitionHarnessApp>) {
    let report = harness.pump(PumpBudget::new(4, usize::MAX, usize::MAX, usize::MAX));
    assert!(report.is_quiescent());
}

#[test]
fn public_harness_transition_replacement_and_preference_change_use_the_same_runtime_clock() {
    let mut harness =
        TestHarness::<TransitionHarnessApp>::mount(TransitionState { transparent: false });
    register_controlled_text(&mut harness);
    let normal = StyleEnvironment::default();
    let normal_context = SurfaceBuildContext::new(&normal, LayoutConstraints::unbounded());

    harness
        .publish_with_context(&normal_context)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(harness_opacity(&harness), 1.0);

    harness
        .submit_action(true)
        .unwrap_or_else(|_| unreachable!());
    pump_actions(&mut harness);
    harness
        .publish_with_context(&normal_context)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(harness_opacity(&harness), 1.0);

    harness
        .advance_time(Duration::from_millis(40))
        .unwrap_or_else(|_| unreachable!("bounded transition advance is valid"));
    harness
        .publish_with_context(&normal_context)
        .unwrap_or_else(|_| unreachable!());
    assert!((harness_opacity(&harness) - 0.6).abs() <= f32::EPSILON);

    harness
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!());
    pump_actions(&mut harness);
    harness
        .publish_with_context(&normal_context)
        .unwrap_or_else(|_| unreachable!());
    assert!((harness_opacity(&harness) - 0.6).abs() <= f32::EPSILON);

    let reduced = StyleEnvironment::default().with_preferences(StylePreferences::new(false, true));
    let reduced_context = SurfaceBuildContext::new(&reduced, LayoutConstraints::unbounded());
    harness
        .submit_action(true)
        .unwrap_or_else(|_| unreachable!());
    pump_actions(&mut harness);
    harness
        .publish_with_context(&reduced_context)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        harness_opacity(&harness),
        0.0,
        "SnapToEnd under reduced motion must commit the current target through ordinary publication"
    );
}
