#![allow(refining_impl_trait)]
macro_rules! admitted {
    ($result:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "expected accepted runtime admission, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

use std::time::Duration;

use runenui_core::{
    AnimationId, Color, ExplicitTimeline, LayoutDimension, LayoutStyle, LogicalLength,
    LogicalPoint, MotionEasing, MotionKeyframe, MotionRepeat, MotionValue, NoHostProtocol,
    PresentationOrigin, PresentationRotation, PresentationScale, PresentationTransform,
    PresentationTranslation, ReducedMotionStrategy, StyleEnvironment, TimelineSpec, UiApp,
    UnitInterval, View, button,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

struct PresentationOrderApp;

impl UiApp for PresentationOrderApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The authored tree has 1 owners and no initial callback output or work.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 5)
            .with_application_outputs(0)
            .with_reconciliation(1, 1, 1)
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        button("ordered")
            .on_activate(|| ())
            .background(Color::WHITE)
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(LogicalLength::from(100_u16)))
                    .with_height(LayoutDimension::length(LogicalLength::from(50_u16))),
            )
            .timeline(presentation_timeline())
            .key("root")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn presentation(
    translation_x: f32,
    translation_y: f32,
    scale_x: f32,
    scale_y: f32,
    rotation: f32,
) -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(translation_x, translation_y)
            .unwrap_or_else(|_| unreachable!("controlled translation is finite")),
        PresentationScale::new(scale_x, scale_y)
            .unwrap_or_else(|_| unreachable!("controlled scale is finite")),
        PresentationRotation::radians(rotation)
            .unwrap_or_else(|_| unreachable!("controlled rotation is finite")),
        PresentationOrigin::new(UnitInterval::HALF, UnitInterval::HALF),
    )
}

fn presentation_timeline() -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Presentation(Some(presentation(0.0, 0.0, 1.0, 1.0, 0.0))),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Presentation(Some(presentation(
                    10.0,
                    20.0,
                    2.0,
                    1.0,
                    core::f32::consts::FRAC_PI_2,
                ))),
            ),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("presentation-order timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("presentation-order")
            .unwrap_or_else(|_| unreachable!("presentation-order id is valid")),
        spec,
    )
}

fn approx_eq(left: f32, right: f32) {
    assert!((left - right).abs() <= 1.0e-3, "{left} != {right}");
}

#[test]
fn sampled_origin_scale_rotate_translate_order_correlates_paint_hit_and_semantics_without_relayout()
{
    let mut runtime = admitted!(AppRuntime::<PresentationOrderApp>::mount(()));
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());

    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial presentation-order publication is admitted"));
    let initial_layout = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("presentation-order app has a root"))
        .bounds();
    approx_eq(initial_layout.x(), 0.0);
    approx_eq(initial_layout.y(), 0.0);
    approx_eq(initial_layout.width(), 100.0);
    approx_eq(initial_layout.height(), 50.0);

    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("bounded presentation-order advance is valid"));
    let terminal = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("terminal presentation-order publication is admitted"));

    let layout = terminal
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("presentation-order app has a root"))
        .bounds();
    assert_eq!(
        layout, initial_layout,
        "presentation motion must not mutate structural layout"
    );

    let paint = terminal
        .paint_scene()
        .items()
        .iter()
        .find(|item| item.primitive().brush().is_some())
        .unwrap_or_else(|| unreachable!("button background contributes paint"));
    let hit = terminal
        .hit_test_scene()
        .regions()
        .first()
        .unwrap_or_else(|| unreachable!("actionable button contributes hit geometry"));
    assert_eq!(paint.local_to_surface(), hit.local_to_surface());

    let [m11, m12, m21, m22, tx, ty] = paint.local_to_surface().components();
    approx_eq(m11, 0.0);
    approx_eq(m12, 2.0);
    approx_eq(m21, -1.0);
    approx_eq(m22, 0.0);
    approx_eq(tx, 85.0);
    approx_eq(ty, -55.0);

    let semantic = terminal
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("ordered"))
        .unwrap_or_else(|| unreachable!("button publishes semantics"));
    let bounds = semantic.bounds();
    approx_eq(bounds.x(), 35.0);
    approx_eq(bounds.y(), -55.0);
    approx_eq(bounds.width(), 50.0);
    approx_eq(bounds.height(), 200.0);

    let transformed_center = LogicalPoint::new(60.0, 45.0)
        .unwrap_or_else(|_| unreachable!("transformed center is finite"));
    assert_eq!(
        terminal.hit_test_scene().target_at(transformed_center),
        Some(
            terminal
                .frame()
                .root()
                .unwrap_or_else(|| unreachable!())
                .id()
        )
    );
}
