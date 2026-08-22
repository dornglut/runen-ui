#![allow(refining_impl_trait)]

use runenui_core::{
    Color, ContributionClip, Element, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, PaintPrimitive, Radius,
    SceneOpacity, SceneShape, StyleTokens, UiApp, Widget, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, PaintSceneItem, SurfaceBuildContext};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

fn item_covers(item: &PaintSceneItem, surface_point: LogicalPoint) -> bool {
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
    rect.width() > 0.0
        && rect.height() > 0.0
        && rect.contains(local_point)
        && item
            .clips()
            .iter()
            .all(|clip| clip.contains_surface_point(surface_point))
}

fn srgb8_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn source_over(dst: [f32; 4], item: &PaintSceneItem) -> [f32; 4] {
    let color = item.primitive().color();
    let alpha = (f32::from(color.alpha()) / 255.0) * item.opacity().get();
    let one_minus_alpha = 1.0 - alpha;
    [
        dst[0].mul_add(one_minus_alpha, srgb8_to_linear(color.red()) * alpha),
        dst[1].mul_add(one_minus_alpha, srgb8_to_linear(color.green()) * alpha),
        dst[2].mul_add(one_minus_alpha, srgb8_to_linear(color.blue()) * alpha),
        dst[3].mul_add(one_minus_alpha, alpha),
    ]
}

fn composite(items: &[PaintSceneItem], sample: LogicalPoint) -> [f32; 4] {
    items
        .iter()
        .filter(|item| item_covers(item, sample))
        .fold([0.0; 4], source_over)
}

fn assert_close(actual: [f32; 4], expected: [f32; 4]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 1.0e-6,
            "{actual} != {expected}"
        );
    }
}

#[derive(Debug)]
struct OpacityOwner;

impl Widget<()> for OpacityOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(10_u16),
            height: LogicalLength::from(10_u16),
        }
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let radius = Radius::all(LogicalLength::from(5_u16));
        let half = SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("test opacity is valid"));
        PaintContribution::new(vec![
            PaintContributionItem::fill_rect(rect(), Color::rgba(255, 0, 0, 255)),
            PaintContributionItem::fill_rect(rect(), Color::rgba(0, 0, 255, 255))
                .with_opacity(half)
                .with_clip(ContributionClip::identity(SceneShape::rounded_rect(
                    rect(),
                    radius,
                ))),
        ])
    }
}

struct OpacityApp;

impl UiApp for OpacityApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(OpacityOwner).key("root")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn item_opacity_multiplies_source_alpha_and_rounded_paint_clip_uses_shared_shape_semantics() {
    let mut runtime = AppRuntime::<OpacityApp>::mount(());
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("opacity publication is admitted"));
    let items = publication.paint_scene().items();
    assert_eq!(items.len(), 2);

    assert_close(composite(items, point(5.0, 5.0)), [0.5, 0.0, 0.5, 1.0]);
    assert_close(composite(items, point(0.0, 0.0)), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(items[1].opacity().get().to_bits(), 0.5_f32.to_bits());
    assert_eq!(items[1].clips().len(), 1);
    assert!(matches!(
        items[1].clips()[0].shape(),
        SceneShape::RoundedRect { .. }
    ));
}
