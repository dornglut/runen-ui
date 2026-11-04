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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        (): &(),
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(16_u16), LogicalLength::from(16_u16))
    }
}

struct SameClockApp;

impl UiApp for SameClockApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One fixed leaf and its initial mounted-subscription reconciliation.
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }
    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // This leaf produces no routed outputs or work in any reachable state.
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }

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
    let mut harness = TestHarness::<SameClockApp>::mount(())
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
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
    assert_eq!(
        first.hit_test_scene().regions(),
        second.hit_test_scene().regions(),
        "same-clock publication retry must preserve sampled hit geometry"
    );
    assert_eq!(
        first.hit_test_scene().mounted_targets(),
        second.hit_test_scene().mounted_targets(),
        "same-clock publication retry must preserve displayed mounted membership"
    );
    assert_eq!(
        first.semantic_publication().snapshot(),
        second.semantic_publication().snapshot()
    );
}
