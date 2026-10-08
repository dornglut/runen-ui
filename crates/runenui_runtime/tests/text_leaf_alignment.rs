#![allow(refining_impl_trait)]

use runenui_core::{
    EdgeInsets, Element, FontFamilyName, GenericFontFamily, LayoutBound, LayoutDimension,
    LayoutStyle, LogicalLength, LogicalPoint, LogicalSize, NoHostProtocol, StyleEnvironment,
    TextAlignment, TextBlockPlacement, TextLeafMeasure, UiApp, View, Widget, WidgetMeasure,
    WidgetMeasureInput, button, text,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePublication};

const CANTARELL: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");
const ARABIC: &[u8] =
    include_bytes!("../../runenui_text/tests/fixtures/RunenUIFixtureArabic-Regular.ttf");

#[derive(Clone, Copy)]
enum Kind {
    Plain,
    Button,
    Downstream(TextAlignment, TextBlockPlacement),
}

struct Case {
    kind: Kind,
    label: &'static str,
    layout: LayoutStyle,
    padding: EdgeInsets,
}

struct AlignmentApp;

#[derive(Debug)]
struct DownstreamLeaf {
    text: &'static str,
    inline: TextAlignment,
    block: TextBlockPlacement,
}

impl Widget<()> for DownstreamLeaf {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(
            TextLeafMeasure::new(self.text)
                .with_inline_alignment(self.inline)
                .with_block_placement(self.block),
        )
    }
}

impl UiApp for AlignmentApp {
    type State = Case;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        match state.kind {
            Kind::Plain => text(state.label)
                .with_layout(state.layout.clone())
                .padding(state.padding)
                .into_element(),
            Kind::Button => button(state.label)
                .on_activate(|| ())
                .with_layout(state.layout.clone())
                .padding(state.padding)
                .into_element(),
            Kind::Downstream(inline, block) => Element::new(DownstreamLeaf {
                text: state.label,
                inline,
                block,
            })
            .with_layout(state.layout.clone())
            .padding(state.padding),
        }
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn length(value: u16) -> LogicalLength {
    LogicalLength::from(value)
}

fn fixed(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(length(width)))
        .with_height(LayoutDimension::length(length(height)))
}

fn minimum(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_min_width(LayoutBound::length(length(width)))
        .with_min_height(LayoutBound::length(length(height)))
}

fn publish(
    kind: Kind,
    label: &'static str,
    layout: LayoutStyle,
    padding: u16,
) -> SurfacePublication {
    let mut app = AppRuntime::<AlignmentApp>::mount(Case {
        kind,
        label,
        layout,
        padding: EdgeInsets::all(length(padding)),
    });
    assert!(app.register_text_font_bytes(CANTARELL.to_vec()).is_ok());
    assert!(app.register_text_font_bytes(ARABIC.to_vec()).is_ok());
    let latin = FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!("known family"));
    let arabic = FontFamilyName::new("RunenUI Fixture Arabic")
        .unwrap_or_else(|_| unreachable!("known Arabic fixture"));
    assert!(
        app.set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[latin, arabic])
            .is_ok()
    );
    let style = StyleEnvironment::default();
    app.publish_surface(&SurfaceBuildContext::new(
        &style,
        LayoutConstraints::unbounded(),
    ))
    .unwrap_or_else(|error| panic!("aligned text publication: {error:?}"))
}

fn first_origin(publication: &SurfacePublication) -> LogicalPoint {
    let run = publication
        .paint_scene()
        .items()
        .iter()
        .find_map(|item| item.primitive().as_shaped_text_run())
        .unwrap_or_else(|| unreachable!("text leaf emits retained shaped text"));
    assert!(
        publication
            .paint_scene()
            .shaped_text_resource(run.resource_ref())
            .is_some(),
        "text paint must retain the exact shaped resource"
    );
    let retained = publication
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!("retained layout is published"))
        .text_measurements()
        .iter()
        .filter(|measurement| measurement.retained_for_paint())
        .collect::<Vec<_>>();
    assert_eq!(retained.len(), 1, "one final text artifact supplies paint");
    let painted_refs = publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .map(|run| run.resource_ref().clone())
        .collect::<Vec<_>>();
    assert_eq!(
        retained[0].retained_resource_refs(),
        painted_refs.as_slice(),
        "final text measurement and paint retain identical shaped resources"
    );
    run.origin()
}

fn bounds(publication: &SurfacePublication) -> LogicalSize {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("mounted leaf publishes"))
        .bounds()
        .size()
}

fn approximately_equal(a: f32, b: f32) {
    assert!((a - b).abs() < 0.05, "{a} != {b}");
}

#[test]
fn text_preserves_start_start_and_button_centers_only_available_slack() {
    let plain = publish(Kind::Plain, "Label", fixed(180, 80), 8);
    let button = publish(Kind::Button, "Label", fixed(180, 80), 8);
    let plain_origin = first_origin(&plain);
    let centered_origin = first_origin(&button);
    approximately_equal(bounds(&plain).width(), 180.0);
    approximately_equal(bounds(&button).height(), 80.0);
    assert!(centered_origin.x() > plain_origin.x() + 10.0);
    assert!(centered_origin.y() > plain_origin.y() + 5.0);
    assert!(plain_origin.x() >= 8.0 && plain_origin.y() >= 8.0);
}

#[test]
fn button_intrinsic_size_is_unchanged_and_minimum_geometry_centers() {
    let intrinsic = publish(Kind::Button, "Label", LayoutStyle::default(), 8);
    let fixed = publish(Kind::Button, "Label", fixed(180, 80), 8);
    let min = publish(Kind::Button, "Label", minimum(180, 80), 8);
    let a = bounds(&intrinsic);
    let b = bounds(&fixed);
    let c = bounds(&min);
    assert!(a.width() > 16.0 && a.width() < 180.0);
    assert!(a.height() > 16.0 && a.height() < 80.0);
    approximately_equal(b.width(), 180.0);
    approximately_equal(b.height(), 80.0);
    assert!(c.width() >= 180.0 && c.height() >= 80.0);
    assert!(first_origin(&fixed).x() > first_origin(&intrinsic).x());
    assert!(first_origin(&fixed).y() > first_origin(&intrinsic).y());
    assert!(first_origin(&min).x() > first_origin(&intrinsic).x());
    assert!(first_origin(&min).y() > first_origin(&intrinsic).y());
}

#[test]
fn downstream_uses_same_inline_and_block_placement_without_runtime_type_checks() {
    let plain = publish(
        Kind::Downstream(TextAlignment::Start, TextBlockPlacement::Start),
        "Custom",
        fixed(180, 80),
        8,
    );
    let end = publish(
        Kind::Downstream(TextAlignment::End, TextBlockPlacement::End),
        "Custom",
        fixed(180, 80),
        8,
    );
    let center = publish(
        Kind::Downstream(TextAlignment::Center, TextBlockPlacement::Center),
        "Custom",
        fixed(180, 80),
        8,
    );
    let a = first_origin(&plain);
    let b = first_origin(&end);
    let c = first_origin(&center);
    assert!(a.x() < c.x() && c.x() < b.x());
    assert!(a.y() < c.y() && c.y() < b.y());
}

#[test]
fn overflowing_text_does_not_receive_negative_block_origin() {
    let text = "Long text with multiple line breaks\nSecond long line\nThird long line";
    let start = publish(
        Kind::Downstream(TextAlignment::Start, TextBlockPlacement::Start),
        text,
        fixed(70, 12),
        0,
    );
    let end = publish(
        Kind::Downstream(TextAlignment::Start, TextBlockPlacement::End),
        text,
        fixed(70, 12),
        0,
    );
    approximately_equal(first_origin(&start).y(), first_origin(&end).y());
    assert!(first_origin(&end).y() >= 0.0);
}

#[test]
fn multiline_center_and_logical_end_are_engine_aligned_not_extra_runtime_offsets() {
    let label = "Longer first line\nShort";
    let start = publish(
        Kind::Downstream(TextAlignment::Start, TextBlockPlacement::Start),
        label,
        fixed(220, 90),
        8,
    );
    let center = publish(
        Kind::Downstream(TextAlignment::Center, TextBlockPlacement::Start),
        label,
        fixed(220, 90),
        8,
    );
    let end = publish(
        Kind::Downstream(TextAlignment::End, TextBlockPlacement::Start),
        label,
        fixed(220, 90),
        8,
    );
    let starts = |p: &SurfacePublication| {
        p.paint_scene()
            .items()
            .iter()
            .filter_map(|item| item.primitive().as_shaped_text_run())
            .map(|run| run.origin().x())
            .collect::<Vec<_>>()
    };
    let a = starts(&start);
    let b = starts(&center);
    let c = starts(&end);
    assert!(a.len() >= 2 && a.len() == b.len() && b.len() == c.len());
    for ((start, center), end) in a.iter().zip(&b).zip(&c) {
        assert!(*start < *center && *center < *end);
    }
}

#[test]
fn rtl_start_end_are_logical_and_center_is_direction_independent() {
    // The Arabic font is repository-bundled: no host font fallback or text direction guess.
    let label = "سلام";
    let start = publish(
        Kind::Downstream(TextAlignment::Start, TextBlockPlacement::Start),
        label,
        fixed(220, 70),
        8,
    );
    let center = publish(
        Kind::Downstream(TextAlignment::Center, TextBlockPlacement::Start),
        label,
        fixed(220, 70),
        8,
    );
    let end = publish(
        Kind::Downstream(TextAlignment::End, TextBlockPlacement::Start),
        label,
        fixed(220, 70),
        8,
    );
    let start_x = first_origin(&start).x();
    let center_x = first_origin(&center).x();
    let end_x = first_origin(&end).x();
    assert!(start_x > center_x && center_x > end_x);
    approximately_equal(center_x * 2.0, start_x + end_x);
}

#[test]
fn minimum_narrower_than_intrinsic_does_not_force_line_breaks() {
    let label = "A long intrinsic button label without explicit breaks";
    let intrinsic = publish(Kind::Button, label, LayoutStyle::default(), 8);
    let narrow_minimum = publish(Kind::Button, label, minimum(20, 10), 8);
    approximately_equal(bounds(&intrinsic).width(), bounds(&narrow_minimum).width());
    approximately_equal(
        bounds(&intrinsic).height(),
        bounds(&narrow_minimum).height(),
    );
    approximately_equal(
        first_origin(&intrinsic).x(),
        first_origin(&narrow_minimum).x(),
    );
    let count_runs = |p: &SurfacePublication| {
        p.paint_scene()
            .items()
            .iter()
            .filter(|item| item.primitive().as_shaped_text_run().is_some())
            .count()
    };
    assert_eq!(count_runs(&intrinsic), count_runs(&narrow_minimum));
}
