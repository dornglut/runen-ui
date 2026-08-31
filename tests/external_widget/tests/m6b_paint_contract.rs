#![allow(refining_impl_trait)]

use runenui_core::{
    Color, Element, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, PaintPrimitive, StyleEnvironment, UiApp,
    Widget, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

#[derive(Debug)]
struct PaintProbe;

impl Widget<()> for PaintProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(10_u16),
            height: LogicalLength::from(10_u16),
        }
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let full = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid rectangle"));
        let zero_area = LogicalRect::try_new(2.0, 2.0, 0.0, 4.0)
            .unwrap_or_else(|_| unreachable!("zero-width logical rectangle is valid"));
        PaintContribution::new(vec![
            PaintContributionItem::fill_rect(full, Color::rgba(255, 0, 0, 128)),
            PaintContributionItem::stroke_rect(
                full,
                Color::rgba(0, 0, 255, 128),
                LogicalLength::from(2_u16),
            ),
            PaintContributionItem::fill_rect(zero_area, Color::WHITE),
            PaintContributionItem::stroke_rect(full, Color::BLACK, LogicalLength::ZERO),
        ])
    }
}

struct App;

impl UiApp for App {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(PaintProbe).key("paint-probe")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn logical_rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("test rectangle is finite and non-negative"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

fn primitive_covers(primitive: &PaintPrimitive, sample: LogicalPoint) -> bool {
    match primitive {
        PaintPrimitive::FillRect { rect, .. } => {
            rect.width() > 0.0 && rect.height() > 0.0 && rect.contains(sample)
        }
        PaintPrimitive::StrokeRect { rect, width, .. } => {
            let stroke = width.get();
            if stroke == 0.0 || rect.width() == 0.0 || rect.height() == 0.0 {
                return false;
            }
            let half = stroke / 2.0;
            let expanded = logical_rect(
                rect.x() - half,
                rect.y() - half,
                rect.width() + stroke,
                rect.height() + stroke,
            );
            if !expanded.contains(sample) {
                return false;
            }
            if rect.width() <= stroke || rect.height() <= stroke {
                return true;
            }
            let inset = logical_rect(
                rect.x() + half,
                rect.y() + half,
                rect.width() - stroke,
                rect.height() - stroke,
            );
            !inset.contains(sample)
        }
        _ => false,
    }
}

fn srgb8_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn source_over(dst: [f32; 4], color: Color) -> [f32; 4] {
    let alpha = f32::from(color.alpha()) / 255.0;
    let one_minus_alpha = 1.0 - alpha;
    [
        dst[0].mul_add(one_minus_alpha, srgb8_to_linear(color.red()) * alpha),
        dst[1].mul_add(one_minus_alpha, srgb8_to_linear(color.green()) * alpha),
        dst[2].mul_add(one_minus_alpha, srgb8_to_linear(color.blue()) * alpha),
        dst[3].mul_add(one_minus_alpha, alpha),
    ]
}

fn composite(primitives: &[PaintPrimitive], sample: LogicalPoint) -> [f32; 4] {
    primitives
        .iter()
        .filter(|primitive| primitive_covers(primitive, sample))
        .fold([0.0; 4], |dst, primitive| {
            source_over(
                dst,
                primitive
                    .color()
                    .unwrap_or_else(|| unreachable!("fixture primitive carries literal color")),
            )
        })
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-6
}

fn assert_close(actual: f32, expected: f32) {
    assert!(close(actual, expected), "{actual} != {expected}");
}

#[test]
fn downstream_scene_preserves_basic_rect_literals_order_and_owner_placement() {
    let mut runtime = AppRuntime::<App>::mount(());
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("external paint publication is admitted"));
    let items = publication.paint_scene().items();

    assert_eq!(items.len(), 4);
    for (actual, expected) in items[0]
        .local_to_surface()
        .components()
        .into_iter()
        .zip([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    {
        assert_close(actual, expected);
    }
    assert!(matches!(
        items[0].primitive(),
        PaintPrimitive::FillRect { rect, color }
            if close(rect.width(), 10.0)
                && close(rect.height(), 10.0)
                && *color == Color::rgba(255, 0, 0, 128)
    ));
    assert!(matches!(
        items[1].primitive(),
        PaintPrimitive::StrokeRect { rect, color, width }
            if close(rect.width(), 10.0)
                && close(rect.height(), 10.0)
                && *color == Color::rgba(0, 0, 255, 128)
                && *width == LogicalLength::from(2_u16)
    ));
    assert!(matches!(
        items[2].primitive(),
        PaintPrimitive::FillRect { rect, .. } if rect.width() == 0.0
    ));
    assert_eq!(
        items[3].primitive().stroke_width(),
        Some(LogicalLength::ZERO)
    );
}

#[test]
fn independent_logical_coverage_proves_degenerate_and_centered_miter_strokes() {
    let fill_zero_width = PaintPrimitive::FillRect {
        rect: logical_rect(0.0, 0.0, 0.0, 10.0),
        color: Color::WHITE,
    };
    let stroke_zero_rect = PaintPrimitive::StrokeRect {
        rect: logical_rect(0.0, 0.0, 0.0, 10.0),
        color: Color::WHITE,
        width: LogicalLength::from(2_u16),
    };
    let stroke_zero_width = PaintPrimitive::StrokeRect {
        rect: logical_rect(0.0, 0.0, 10.0, 10.0),
        color: Color::WHITE,
        width: LogicalLength::ZERO,
    };
    let centered = PaintPrimitive::StrokeRect {
        rect: logical_rect(0.0, 0.0, 10.0, 10.0),
        color: Color::WHITE,
        width: LogicalLength::from(2_u16),
    };
    let collapsed_inset = PaintPrimitive::StrokeRect {
        rect: logical_rect(0.0, 0.0, 1.0, 10.0),
        color: Color::WHITE,
        width: LogicalLength::from(2_u16),
    };

    assert!(!primitive_covers(&fill_zero_width, point(0.0, 1.0)));
    assert!(!primitive_covers(&stroke_zero_rect, point(0.0, 1.0)));
    assert!(!primitive_covers(&stroke_zero_width, point(0.0, 1.0)));
    assert!(primitive_covers(&centered, point(-0.5, -0.5)));
    assert!(primitive_covers(&centered, point(0.5, 5.0)));
    assert!(!primitive_covers(&centered, point(5.0, 5.0)));
    assert!(primitive_covers(&collapsed_inset, point(0.5, 5.0)));
    assert!(LogicalLength::new(-1.0).is_err());
}

#[test]
fn independent_fixed_opacity_compositor_decodes_srgb_and_uses_source_over_scene_order() {
    let area = logical_rect(0.0, 0.0, 10.0, 10.0);
    let red = PaintPrimitive::FillRect {
        rect: area,
        color: Color::rgba(255, 0, 0, 128),
    };
    let blue = PaintPrimitive::FillRect {
        rect: area,
        color: Color::rgba(0, 0, 255, 128),
    };
    let alpha = 128.0 / 255.0;
    let red_then_blue = composite(&[red.clone(), blue.clone()], point(1.0, 1.0));
    let blue_then_red = composite(&[blue, red], point(1.0, 1.0));

    assert_close(srgb8_to_linear(128), 0.215_860_53);
    assert_close(red_then_blue[0], alpha * (1.0 - alpha));
    assert_close(red_then_blue[2], alpha);
    assert_close(red_then_blue[3], alpha + alpha * (1.0 - alpha));
    assert_close(blue_then_red[0], alpha);
    assert_close(blue_then_red[2], alpha * (1.0 - alpha));
    assert!(
        red_then_blue
            .iter()
            .zip(blue_then_red)
            .any(|(red_first, blue_first)| !close(*red_first, blue_first))
    );
}
