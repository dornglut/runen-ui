#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, LogicalLength, MotionEasing, MotionKeyframe,
    MotionRepeat, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, TimelineSpec,
    UiApp, UnitInterval, Widget, WidgetMeasure, WidgetMeasureInput,
};
use runenui_testing::TestHarness;

#[derive(Debug)]
struct Probe;

impl Widget<()> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(16_u16), LogicalLength::from(16_u16))
    }
}

struct SameClockApp;

impl UiApp for SameClockApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(Probe)
            .key("probe")
            .timeline(opacity_timeline())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn opacity_timeline() -> ExplicitTimeline {
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
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("same-clock timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("same-clock")
            .unwrap_or_else(|_| unreachable!("same-clock id is valid")),
        spec,
    )
}

fn opacity(publication: &runenui_runtime::SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("same-clock app has one root"))
        .computed_style()
        .opacity()
        .get()
}

#[test]
fn repeated_publication_at_one_manual_clock_instant_reuses_the_same_motion_sample() {
    let mut harness = TestHarness::<SameClockApp>::mount(());
    harness.publish().unwrap_or_else(|_| unreachable!());
    harness
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded same-clock advance is valid"));

    let first = harness
        .publish()
        .unwrap_or_else(|_| unreachable!("midpoint publication is admitted"))
        .clone();
    assert!((opacity(&first) - 0.5).abs() <= f32::EPSILON);

    let second = harness
        .publish()
        .unwrap_or_else(|_| unreachable!("same-clock retry publication is admitted"))
        .clone();
    assert!((opacity(&second) - 0.5).abs() <= f32::EPSILON);
    assert!(
        first.renderer_products_eq(&second),
        "same-clock publication retry must not mint a different sampled renderer product"
    );
    assert_eq!(first.hit_test_scene(), second.hit_test_scene());
    assert_eq!(
        first.semantic_publication().snapshot(),
        second.semantic_publication().snapshot()
    );
}
