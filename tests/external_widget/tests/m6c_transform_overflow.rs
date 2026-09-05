#![allow(refining_impl_trait)]

use runenui_core::{
    Color, ContributionClip, Element, ElementId, HitContribution, HitContributionContext,
    HitRegion, LogicalLength, LogicalRect, LogicalTransform, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, SceneShape, StyleEnvironment, UiApp, View,
    Widget, WidgetMeasure, children, column,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn huge_translation() -> LogicalTransform {
    LogicalTransform::translation(0.0, 3.0e38)
        .unwrap_or_else(|_| unreachable!("test translation is finite"))
}

fn authored(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("test authored id is valid"))
}

#[derive(Debug)]
struct TallSpacer;

impl Widget<()> for TallSpacer {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(20_u16),
            LogicalLength::new(1.0e38)
                .unwrap_or_else(|_| unreachable!("test spacer height is finite")),
        )
    }
}

#[derive(Debug)]
struct OverflowOwner;

impl Widget<()> for OverflowOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(vec![
            PaintContributionItem::fill_rect(rect(), Color::BLACK)
                .with_transform(huge_translation()),
            PaintContributionItem::fill_rect(rect(), Color::WHITE).with_clip(
                ContributionClip::new(SceneShape::rect(rect()), huge_translation()),
            ),
        ])
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        HitContribution::new(vec![
            HitRegion::rect(rect()).with_transform(huge_translation()),
            HitRegion::rect(rect()).with_clip(ContributionClip::new(
                SceneShape::rect(rect()),
                huge_translation(),
            )),
        ])
    }
}

struct OverflowApp;

impl UiApp for OverflowApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            Element::new(TallSpacer).key("spacer"),
            Element::new(OverflowOwner).id("overflow").key("overflow"),
        ])
        .key("root")
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn finite_authored_transforms_that_overflow_surface_composition_are_excluded_and_diagnosed() {
    let mut runtime = AppRuntime::<OverflowApp>::mount(());
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("overflow fixture publication is admitted"));
    let overflow = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored("overflow")))
        .unwrap_or_else(|| unreachable!("overflow owner is published"));

    assert!(overflow.bounds().y() > 5.0e37);
    assert!(publication.paint_scene().items().is_empty());
    assert!(publication.hit_test_scene().regions().is_empty());
    assert_eq!(
        overflow
            .diagnostics()
            .iter()
            .map(runenui_core::WidgetDiagnostic::code)
            .collect::<Vec<_>>(),
        vec![
            "runenui.scene.hit-transform-non-finite",
            "runenui.scene.hit-clip-transform-non-finite",
            "runenui.scene.paint-transform-non-finite",
            "runenui.scene.paint-clip-transform-non-finite",
        ]
    );
}
