#![allow(refining_impl_trait)]

use runenui_core::{
    Color, ContributionClip, Element, HitContribution, HitContributionContext, HitRegion,
    LogicalLength, LogicalPoint, LogicalRect, LogicalTransform, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, PaintPrimitive, SceneShape, StyleTokens,
    UiApp, Widget, WidgetInvalidation, WidgetMeasure, WidgetUpdateContext,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintSceneItem, PumpBudget, SurfaceBuildContext, SurfacePhase,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

fn singular_transform() -> LogicalTransform {
    LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)
        .unwrap_or_else(|_| unreachable!("singular test transform is finite"))
}

fn process_one(runtime: &mut AppRuntime<SceneDiagnosticApp>, action: SceneDiagnosticAction) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!("test action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn fill_item_covers_surface_point(item: &PaintSceneItem, surface_point: LogicalPoint) -> bool {
    let Some(local_point) = item
        .local_to_surface()
        .inverse()
        .and_then(|surface_to_local| surface_to_local.transform_point(surface_point))
    else {
        return false;
    };
    let PaintPrimitive::FillRect { rect, .. } = item.primitive() else {
        return false;
    };
    rect.contains(local_point)
        && item
            .clips()
            .iter()
            .all(|clip| clip.contains_surface_point(surface_point))
}

fn diagnostic_codes(publication: &runenui_runtime::SurfacePublication) -> Vec<&str> {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("diagnostic root is published"))
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect()
}

#[derive(Debug)]
struct SceneDiagnosticOwner {
    paint_singular: bool,
    hit_singular: bool,
}

impl Widget<SceneDiagnosticAction> for SceneDiagnosticOwner {
    type State = (bool, bool);

    fn create_state(&self) -> Self::State {
        (self.paint_singular, self.hit_singular)
    }

    fn update(
        &self,
        state: &mut Self::State,
        context: &mut WidgetUpdateContext<SceneDiagnosticAction>,
    ) {
        if state.0 != self.paint_singular {
            state.0 = self.paint_singular;
            context.invalidate(WidgetInvalidation::PAINT);
        }
        if state.1 != self.hit_singular {
            state.1 = self.hit_singular;
            context.invalidate(WidgetInvalidation::HIT_TEST);
        }
    }

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn paint(&self, state: &Self::State, _: PaintContributionContext) -> PaintContribution {
        let transform = if state.0 {
            singular_transform()
        } else {
            LogicalTransform::IDENTITY
        };
        let full = rect(0.0, 0.0, 20.0, 20.0);
        PaintContribution::new(vec![
            PaintContributionItem::fill_rect(full, Color::BLACK).with_transform(transform),
            PaintContributionItem::fill_rect(full, Color::WHITE)
                .with_clip(ContributionClip::new(SceneShape::rect(full), transform)),
        ])
    }

    fn hit_test(&self, state: &Self::State, _: HitContributionContext) -> HitContribution {
        let transform = if state.1 {
            singular_transform()
        } else {
            LogicalTransform::IDENTITY
        };
        let full = rect(0.0, 0.0, 20.0, 20.0);
        HitContribution::new(vec![
            HitRegion::rect(rect(0.0, 0.0, 8.0, 8.0)),
            HitRegion::rect(full).with_transform(transform),
            HitRegion::rect(full)
                .with_clip(ContributionClip::new(SceneShape::rect(full), transform)),
        ])
    }
}

#[derive(Debug)]
struct SceneDiagnosticState {
    paint_singular: bool,
    hit_singular: bool,
}

#[derive(Clone, Copy, Debug)]
enum SceneDiagnosticAction {
    FixPaint,
    FixHit,
}

struct SceneDiagnosticApp;

impl UiApp for SceneDiagnosticApp {
    type State = SceneDiagnosticState;
    type Action = SceneDiagnosticAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(SceneDiagnosticOwner {
            paint_singular: state.paint_singular,
            hit_singular: state.hit_singular,
        })
        .key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            SceneDiagnosticAction::FixPaint => state.paint_singular = false,
            SceneDiagnosticAction::FixHit => state.hit_singular = false,
        }
    }
}

fn publish(
    runtime: &mut AppRuntime<SceneDiagnosticApp>,
    tokens: &StyleTokens,
) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("test publication is admitted"))
}

#[test]
fn singular_scene_diagnostics_are_public_fail_closed_and_cleared_by_their_owning_phase() {
    let mut runtime = AppRuntime::<SceneDiagnosticApp>::mount(SceneDiagnosticState {
        paint_singular: true,
        hit_singular: true,
    });
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let tokens = StyleTokens::new();

    let initial = publish(&mut runtime, &tokens);
    let root = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("diagnostic root is published"));
    let lower_sample = point(root.bounds().x() + 5.0, root.bounds().y() + 5.0);
    let invalid_only_sample = point(root.bounds().x() + 15.0, root.bounds().y() + 15.0);
    assert_eq!(initial.paint_scene().items().len(), 2);
    assert!(
        initial
            .paint_scene()
            .items()
            .iter()
            .all(|item| !fill_item_covers_surface_point(item, lower_sample))
    );
    assert_eq!(initial.hit_test_scene().regions().len(), 3);
    assert_eq!(
        initial.hit_test_scene().target_at(lower_sample),
        Some(root.id())
    );
    assert_eq!(
        initial.hit_test_scene().target_at(invalid_only_sample),
        None
    );
    assert_eq!(
        diagnostic_codes(&initial),
        vec![
            "runenui.scene.hit-transform-non-invertible",
            "runenui.scene.hit-clip-transform-non-invertible",
            "runenui.scene.paint-transform-non-invertible",
            "runenui.scene.paint-clip-transform-non-invertible",
        ]
    );

    process_one(&mut runtime, SceneDiagnosticAction::FixPaint);
    let paint_fixed = publish(&mut runtime, &tokens);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );
    assert!(
        paint_fixed
            .paint_scene()
            .items()
            .iter()
            .all(|item| fill_item_covers_surface_point(item, lower_sample))
    );
    assert_eq!(
        paint_fixed.hit_test_scene().target_at(lower_sample),
        paint_fixed.frame().root().map(|node| node.id())
    );
    assert_eq!(
        paint_fixed.hit_test_scene().target_at(invalid_only_sample),
        None
    );
    assert_eq!(
        diagnostic_codes(&paint_fixed),
        vec![
            "runenui.scene.hit-transform-non-invertible",
            "runenui.scene.hit-clip-transform-non-invertible",
        ]
    );
    let paint_revision_after_fix = paint_fixed.paint_publication().revision();

    process_one(&mut runtime, SceneDiagnosticAction::FixHit);
    let hit_fixed = publish(&mut runtime, &tokens);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::HitTesting]
    );
    assert!(diagnostic_codes(&hit_fixed).is_empty());
    let target = hit_fixed
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("diagnostic root remains published"))
        .id();
    assert_eq!(
        hit_fixed.hit_test_scene().target_at(lower_sample),
        Some(target)
    );
    assert_eq!(
        hit_fixed.hit_test_scene().target_at(invalid_only_sample),
        Some(target)
    );
    assert_eq!(
        hit_fixed.paint_publication().revision(),
        paint_revision_after_fix
    );
}
