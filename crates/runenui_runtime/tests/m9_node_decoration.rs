#![allow(refining_impl_trait)]

use runenui_core::{
    Brush, Color, Element, FontFamilyName, GenericFontFamily, LayoutDimension, LayoutStyle,
    LogicalLength, LogicalRect, NoHostProtocol, Outline, PaintContribution,
    PaintContributionContext, PaintContributionGroup, PaintContributionItem, PaintPrimitive,
    Radius, SceneLayer, SceneOpacity, SceneShape, StrokeStyle, StyleEnvironment, UiApp, View,
    Widget, WidgetMeasure, WidgetMeasureInput, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintSceneGroupId, PaintSceneItem, SurfaceBuildContext,
    SurfacePublication,
};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

const NEGATIVE: Color = Color::rgb(11, 12, 13);
const BACKGROUND: Color = Color::rgb(21, 22, 23);
const ZERO: Color = Color::rgb(31, 32, 33);
const OUTLINE: Color = Color::rgb(41, 42, 43);
const POSITIVE: Color = Color::rgb(51, 52, 53);
const GROUPED: Color = Color::rgb(61, 62, 63);

fn fixed_layout(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(LogicalLength::from(width)))
        .with_height(LayoutDimension::length(LogicalLength::from(height)))
}

fn local_rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 4.0, 4.0)
        .unwrap_or_else(|_| unreachable!("controlled paint rectangle is valid"))
}

fn fill(color: Color, layer: i64) -> PaintContributionItem {
    PaintContributionItem::fill(SceneShape::rect(local_rect()), Brush::solid(color))
        .with_layer(SceneLayer::new(layer))
}

fn outline() -> Outline {
    Outline::new(
        Brush::solid(OUTLINE),
        StrokeStyle::new(LogicalLength::from(2_u16)),
    )
}

fn publish<App: UiApp + 'static>(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled decoration publication is admitted"))
}

fn register_font<App: UiApp>(runtime: &mut AppRuntime<App>) {
    runtime
        .register_text_font_bytes(CANTARELL.to_vec())
        .unwrap_or_else(|_| unreachable!("controlled text fixture is registerable"));
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled family name is canonical"));
    runtime
        .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
        .unwrap_or_else(|_| unreachable!("controlled generic mapping is valid"));
}

const fn solid_fill_color(item: &PaintSceneItem) -> Option<Color> {
    match item.primitive() {
        PaintPrimitive::Fill {
            brush: Brush::Solid(color),
            ..
        } => Some(*color),
        _ => None,
    }
}

const fn solid_stroke_color(item: &PaintSceneItem) -> Option<Color> {
    match item.primitive() {
        PaintPrimitive::Stroke {
            brush: Brush::Solid(color),
            ..
        } => Some(*color),
        _ => None,
    }
}

fn approx_eq(left: f32, right: f32) {
    assert!((left - right).abs() <= 1.0e-4, "{left} != {right}");
}

fn find_fill(publication: &SurfacePublication, color: Color) -> usize {
    publication
        .paint_scene()
        .items()
        .iter()
        .position(|item| solid_fill_color(item) == Some(color))
        .unwrap_or_else(|| unreachable!("controlled fill is published"))
}

fn find_stroke(publication: &SurfacePublication, color: Color) -> usize {
    publication
        .paint_scene()
        .items()
        .iter()
        .position(|item| solid_stroke_color(item) == Some(color))
        .unwrap_or_else(|| unreachable!("controlled stroke is published"))
}

struct BuiltinBackgroundApp;

impl UiApp for BuiltinBackgroundApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(Vec::<Element<()>>::new())
            .background(BACKGROUND)
            .with_layout(fixed_layout(30, 20))
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn builtin_background_has_one_runtime_publication_authority() {
    let mut runtime = AppRuntime::<BuiltinBackgroundApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(scene.items().len(), 1);
    let item = &scene.items()[0];
    assert_eq!(solid_fill_color(item), Some(BACKGROUND));
    assert!(matches!(
        item.primitive(),
        PaintPrimitive::Fill {
            shape: SceneShape::Rect(_),
            ..
        }
    ));
    assert_eq!(item.layer(), SceneLayer::ZERO);
    assert_eq!(item.opacity(), SceneOpacity::OPAQUE);
    assert!(item.clips().is_empty());
    assert_eq!(item.group(), None);
}

#[derive(Debug)]
struct LayeredTextPaint;

impl Widget<()> for LayeredTextPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: "decoration order".to_owned(),
        }
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(vec![fill(NEGATIVE, -1), fill(ZERO, 0), fill(POSITIVE, 1)])
    }
}

struct LayeredTextApp;

impl UiApp for LayeredTextApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(LayeredTextPaint)
            .background(BACKGROUND)
            .radius(Radius::all(LogicalLength::from(6_u16)))
            .outline(outline())
            .with_layout(fixed_layout(140, 40))
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn rounded_decoration_preserves_m6_layer_and_local_text_order() {
    let mut runtime = AppRuntime::<LayeredTextApp>::mount(());
    register_font(&mut runtime);
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    let negative = find_fill(&publication, NEGATIVE);
    let background = find_fill(&publication, BACKGROUND);
    let zero = find_fill(&publication, ZERO);
    let outline = find_stroke(&publication, OUTLINE);
    let positive = find_fill(&publication, POSITIVE);
    let text = scene
        .items()
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matches!(item.primitive(), PaintPrimitive::ShapedTextRun(_)).then_some(index)
        })
        .collect::<Vec<_>>();

    assert!(negative < background);
    assert!(background < zero);
    assert!(!text.is_empty());
    assert!(text.iter().all(|index| zero < *index && *index < outline));
    assert!(outline < positive);
    assert_eq!(
        scene
            .items()
            .iter()
            .filter(|item| solid_fill_color(item) == Some(BACKGROUND))
            .count(),
        1
    );

    let background_item = &scene.items()[background];
    let outline_item = &scene.items()[outline];
    let expected_radius = Radius::all(LogicalLength::from(6_u16));
    let expected_bounds = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("controlled root is published"))
        .bounds();

    let PaintPrimitive::Fill {
        shape: SceneShape::RoundedRect { rect, radius },
        ..
    } = background_item.primitive()
    else {
        unreachable!("non-zero node radius publishes a rounded background")
    };
    assert_eq!(*radius, expected_radius);
    approx_eq(rect.x(), 0.0);
    approx_eq(rect.y(), 0.0);
    approx_eq(rect.width(), expected_bounds.width());
    approx_eq(rect.height(), expected_bounds.height());

    let PaintPrimitive::Stroke {
        shape:
            SceneShape::RoundedRect {
                rect: outline_rect,
                radius: outline_radius,
            },
        style,
        ..
    } = outline_item.primitive()
    else {
        unreachable!("outline shares the rounded node-decoration shape")
    };
    assert_eq!(outline_rect, rect);
    assert_eq!(outline_radius, radius);
    assert_eq!(*style, StrokeStyle::new(LogicalLength::from(2_u16)));
    assert_eq!(
        background_item.local_to_surface(),
        outline_item.local_to_surface()
    );
    assert_eq!(background_item.layer(), SceneLayer::ZERO);
    assert_eq!(outline_item.layer(), SceneLayer::ZERO);
    assert_eq!(background_item.opacity(), SceneOpacity::OPAQUE);
    assert_eq!(outline_item.opacity(), SceneOpacity::OPAQUE);
    assert!(background_item.clips().is_empty());
    assert!(outline_item.clips().is_empty());
}

#[derive(Debug)]
struct ExplicitGroupedPaint;

impl Widget<()> for ExplicitGroupedPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(24_u16), LogicalLength::from(16_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::from_entries(vec![
            PaintContributionGroup::new(vec![fill(GROUPED, 0).into()]).into(),
        ])
    }
}

struct DecorationGroupingApp;

impl UiApp for DecorationGroupingApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(ExplicitGroupedPaint)
            .background(BACKGROUND)
            .outline(outline())
            .opacity(
                SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("controlled opacity is valid")),
            )
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn only_group(entry: runenui_runtime::PaintSceneEntry) -> PaintSceneGroupId {
    entry
        .group_id()
        .unwrap_or_else(|| unreachable!("controlled entry is a group"))
}

#[test]
fn runtime_decoration_stays_outside_explicit_group_but_inside_node_effect_group() {
    let mut runtime = AppRuntime::<DecorationGroupingApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    let background = find_fill(&publication, BACKGROUND);
    let grouped = find_fill(&publication, GROUPED);
    let outline = find_stroke(&publication, OUTLINE);
    assert_eq!(scene.root_entries().len(), 1);

    let node_group_id = only_group(scene.root_entries()[0]);
    let node_group = scene
        .group(node_group_id)
        .unwrap_or_else(|| unreachable!("runtime node-effect group resolves"));
    assert_eq!(
        node_group.opacity(),
        SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!())
    );

    let explicit_group_id = node_group
        .entries()
        .iter()
        .copied()
        .find_map(runenui_runtime::PaintSceneEntry::group_id)
        .unwrap_or_else(|| unreachable!("widget-authored explicit group remains nested"));
    let explicit_group = scene
        .group(explicit_group_id)
        .unwrap_or_else(|| unreachable!("explicit group resolves"));
    assert_eq!(explicit_group.parent(), Some(node_group_id));
    assert_eq!(explicit_group.entries().len(), 1);
    assert_eq!(explicit_group.entries()[0].item_index(), Some(grouped));

    assert_eq!(scene.items()[grouped].group(), Some(explicit_group_id));
    assert_eq!(scene.items()[background].group(), Some(node_group_id));
    assert_eq!(scene.items()[outline].group(), Some(node_group_id));
    assert!(
        node_group
            .entries()
            .iter()
            .any(|entry| entry.item_index() == Some(background))
    );
    assert!(
        node_group
            .entries()
            .iter()
            .any(|entry| entry.item_index() == Some(outline))
    );
}
