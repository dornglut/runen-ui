#![allow(refining_impl_trait)]

use runenui_core::{
    EdgeInsets, Element, FlexBasis, FlexContainerStyle, FlexDirection, FlexItemStyle, FlexWrap,
    FontFamilyName, GenericFontFamily, GridAutoFlow, GridAxisPlacement, GridContainerStyle,
    GridItemPlacement, GridItemStyle, GridLine, GridSpan, GridTrack, ItemAlignment,
    LayoutContainer, LayoutDimension, LayoutGap, LayoutInsets, LayoutPosition, LayoutStyle,
    LogicalLength, LogicalSize, MainAxisAlignment, NoHostProtocol, OverflowPolicy, OverflowStyle,
    OverlayAlignment, OverlayContainerStyle, UiApp, View, Widget, WidgetMeasure,
    WidgetMeasureInput, WidgetMeasuredSize, children, row, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfaceFrame, SurfaceLayoutNode,
    SurfacePublication,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!())
}

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!())
}

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

fn register_controlled_text<App: UiApp>(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled text fixture is registerable"))
            > 0
    );
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled family name is canonical"));
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
            .unwrap_or_else(|_| unreachable!("controlled family mapping is valid"))
    );
}

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    width: f32,
    height: f32,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &runenui_core::StyleEnvironment::default(),
            LayoutConstraints::loose(size(width, height)),
        ))
        .unwrap_or_else(|_| unreachable!("layout-mode publication is admitted"))
}

fn frame_node<'a>(frame: &'a SurfaceFrame, authored: &str) -> &'a runenui_runtime::SurfaceNode {
    frame
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored frame node is published"))
}

fn layout_node<'a>(
    report: &'a runenui_runtime::SurfaceLayoutReport,
    authored: &str,
) -> &'a SurfaceLayoutNode {
    report
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored layout node is published"))
}

#[derive(Debug)]
struct FixedMeasured {
    width: LogicalLength,
    height: LogicalLength,
}

impl Widget<()> for FixedMeasured {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, _state: &Self::State, _input: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Measured(WidgetMeasuredSize::new(
            LogicalSize::new(self.width, self.height),
            None,
            None,
        ))
    }
}

fn fixed(width: f32, height: f32) -> Element<()> {
    Element::new(FixedMeasured {
        width: length(width),
        height: length(height),
    })
}

struct FlexAlignmentApp;

impl UiApp for FlexAlignmentApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            fixed(20.0, 10.0).id("flex.first"),
            fixed(20.0, 20.0).id("flex.second")
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default()
                        .with_direction(FlexDirection::RowReverse)
                        .with_justify_content(MainAxisAlignment::Center)
                        .with_align_items(ItemAlignment::Center),
                ))
                .with_gaps(LayoutGap::new(length(10.0), length(0.0)))
                .with_width(LayoutDimension::length(length(100.0)))
                .with_height(LayoutDimension::length(length(60.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn flex_direction_gap_and_alignment_follow_authored_order() {
    let mut runtime = AppRuntime::<FlexAlignmentApp>::mount(());
    let publication = publish(&mut runtime, 100.0, 60.0);
    let first = frame_node(publication.frame(), "flex.first").bounds();
    let second = frame_node(publication.frame(), "flex.second").bounds();
    assert!(first.x() > second.x());
    assert!((first.x() - second.x() - 30.0).abs() <= f32::EPSILON);
    assert!((first.y() - 25.0).abs() <= f32::EPSILON);
    assert!((second.y() - 20.0).abs() <= f32::EPSILON);
}

struct FlexSizingApp;

impl UiApp for FlexSizingApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            fixed(20.0, 10.0).id("flex.grow-a").with_layout(
                LayoutStyle::default().with_flex_item(
                    FlexItemStyle::default()
                        .with_basis(FlexBasis::length(length(20.0)))
                        .with_grow(runenui_core::LayoutFactor::ONE),
                )
            ),
            fixed(20.0, 10.0).id("flex.grow-b").with_layout(
                LayoutStyle::default().with_flex_item(
                    FlexItemStyle::default()
                        .with_basis(FlexBasis::length(length(20.0)))
                        .with_grow(runenui_core::LayoutFactor::new(2.0).unwrap_or_default()),
                )
            )
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Flex(FlexContainerStyle::default()))
                .with_width(LayoutDimension::length(length(100.0)))
                .with_height(LayoutDimension::length(length(30.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn flex_basis_and_growth_change_final_geometry() {
    let mut runtime = AppRuntime::<FlexSizingApp>::mount(());
    let publication = publish(&mut runtime, 100.0, 30.0);
    let a = frame_node(publication.frame(), "flex.grow-a").bounds();
    let b = frame_node(publication.frame(), "flex.grow-b").bounds();
    assert!((a.width() + b.width() - 100.0).abs() <= f32::EPSILON);
    assert!(a.width() > 20.0);
    assert!(b.width() > a.width());
}

struct FlexWrapApp;

impl UiApp for FlexWrapApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            fixed(30.0, 10.0).id("flex.wrap-a"),
            fixed(30.0, 10.0).id("flex.wrap-b"),
            fixed(30.0, 10.0).id("flex.wrap-c")
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_wrap(FlexWrap::Wrap),
                ))
                .with_gap(length(5.0))
                .with_width(LayoutDimension::length(length(60.0)))
                .with_height(LayoutDimension::length(length(60.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn flex_wrap_places_later_authored_items_on_a_new_line() {
    let mut runtime = AppRuntime::<FlexWrapApp>::mount(());
    let publication = publish(&mut runtime, 60.0, 60.0);
    let a = frame_node(publication.frame(), "flex.wrap-a").bounds();
    let c = frame_node(publication.frame(), "flex.wrap-c").bounds();
    assert!(c.y() > a.y());
}

fn grid_line(line: u16) -> GridLine {
    GridLine::new(line).unwrap_or_else(|_| unreachable!())
}

struct GridApp;

impl UiApp for GridApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            text("Grid text").id("grid.auto"),
            fixed(10.0, 10.0).id("grid.explicit").with_layout(
                LayoutStyle::default().with_grid_item(GridItemStyle::default().with_placement(
                    GridItemPlacement::new(
                        GridAxisPlacement::new(Some(grid_line(2)), GridSpan::ONE),
                        GridAxisPlacement::default(),
                    )
                ),)
            ),
            fixed(20.0, 10.0)
                .id("grid.span")
                .with_layout(LayoutStyle::default().with_grid_item(
                    GridItemStyle::default().with_placement(GridItemPlacement::new(
                        GridAxisPlacement::new(
                            Some(grid_line(1)),
                            GridSpan::new(2).unwrap_or_else(|_| unreachable!()),
                        ),
                        GridAxisPlacement::new(Some(grid_line(2)), GridSpan::ONE),
                    )),
                ))
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Grid(GridContainerStyle::new(
                    [
                        GridTrack::length(length(30.0)),
                        GridTrack::fraction(runenui_core::LayoutFactor::ONE),
                    ],
                    [GridTrack::length(length(20.0)), GridTrack::auto()],
                )))
                .with_gaps(LayoutGap::new(length(5.0), length(5.0)))
                .with_width(LayoutDimension::length(length(100.0)))
                .with_height(LayoutDimension::length(length(60.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn grid_tracks_placement_spans_and_intrinsic_contribution_are_published() {
    let mut runtime = AppRuntime::<GridApp>::mount(());
    register_controlled_text(&mut runtime);
    let publication = publish(&mut runtime, 100.0, 60.0);
    let auto = frame_node(publication.frame(), "grid.auto").bounds();
    let explicit = frame_node(publication.frame(), "grid.explicit").bounds();
    let span = frame_node(publication.frame(), "grid.span").bounds();
    assert!(auto.x() <= f32::EPSILON);
    assert!(explicit.x() > 30.0);
    assert!(span.y() >= 25.0);
    assert!(span.width() > 30.0);
}

struct GridColumnFlowApp;

impl UiApp for GridColumnFlowApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            fixed(10.0, 10.0).id("grid.column-a"),
            fixed(10.0, 10.0).id("grid.column-b"),
            fixed(10.0, 10.0).id("grid.column-c")
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Grid(
                    GridContainerStyle::new(
                        [GridTrack::length(length(20.0))],
                        [
                            GridTrack::length(length(20.0)),
                            GridTrack::length(length(20.0)),
                        ],
                    )
                    .with_auto_columns(GridTrack::length(length(20.0)))
                    .with_auto_flow(GridAutoFlow::Column),
                ))
                .with_gaps(LayoutGap::new(length(5.0), length(5.0)))
                .with_width(LayoutDimension::length(length(70.0)))
                .with_height(LayoutDimension::length(length(40.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn grid_column_auto_flow_creates_an_implicit_column() {
    let mut runtime = AppRuntime::<GridColumnFlowApp>::mount(());
    let publication = publish(&mut runtime, 70.0, 40.0);
    let a = frame_node(publication.frame(), "grid.column-a").bounds();
    let b = frame_node(publication.frame(), "grid.column-b").bounds();
    let c = frame_node(publication.frame(), "grid.column-c").bounds();
    assert!(b.y() > a.y());
    assert!(c.x() > a.x());
}

struct PositionedApp;

impl UiApp for PositionedApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            fixed(20.0, 10.0).id("absolute.child").with_layout(
                LayoutStyle::default()
                    .with_position(LayoutPosition::Absolute(LayoutInsets::new(
                        runenui_core::LayoutOffset::length(length(7.0)),
                        runenui_core::LayoutOffset::Auto,
                        runenui_core::LayoutOffset::Auto,
                        runenui_core::LayoutOffset::length(length(11.0)),
                    )))
                    .with_width(LayoutDimension::length(length(20.0)))
                    .with_height(LayoutDimension::length(length(10.0))),
            )
        ])
        .with_layout(
            LayoutStyle::default()
                .with_width(LayoutDimension::length(length(100.0)))
                .with_height(LayoutDimension::length(length(80.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn absolute_positioning_uses_authored_containing_block_offsets() {
    let mut runtime = AppRuntime::<PositionedApp>::mount(());
    let publication = publish(&mut runtime, 100.0, 80.0);
    let bounds = frame_node(publication.frame(), "absolute.child").bounds();
    assert!((bounds.x() - 11.0).abs() <= f32::EPSILON);
    assert!((bounds.y() - 7.0).abs() <= f32::EPSILON);
}

struct MarginApp;

impl UiApp for MarginApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![fixed(20.0, 10.0).id("margin.child").with_layout(
            LayoutStyle::default().with_margin(EdgeInsets::new(
                length(5.0),
                length(0.0),
                length(0.0),
                length(7.0),
            )),
        )])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Block)
                .with_width(LayoutDimension::length(length(100.0)))
                .with_height(LayoutDimension::length(length(40.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn block_margin_contributes_to_positioned_box_geometry() {
    let mut runtime = AppRuntime::<MarginApp>::mount(());
    let publication = publish(&mut runtime, 100.0, 40.0);
    let bounds = frame_node(publication.frame(), "margin.child").bounds();
    assert!((bounds.x() - 7.0).abs() <= f32::EPSILON);
    assert!((bounds.y() - 5.0).abs() <= f32::EPSILON);
}

struct OverlayApp;

impl UiApp for OverlayApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        row(children![
            fixed(20.0, 10.0).id("overlay.small"),
            fixed(30.0, 20.0).id("overlay.large")
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Overlay(OverlayContainerStyle::new(
                    OverlayAlignment::Center,
                    OverlayAlignment::End,
                )))
                .with_width(LayoutDimension::length(length(100.0)))
                .with_height(LayoutDimension::length(length(80.0))),
        )
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn overlay_children_share_one_cell_and_keep_alignment_geometry() {
    let mut runtime = AppRuntime::<OverlayApp>::mount(());
    let publication = publish(&mut runtime, 100.0, 80.0);
    let small = frame_node(publication.frame(), "overlay.small").bounds();
    let large = frame_node(publication.frame(), "overlay.large").bounds();
    assert!(small.x() > 0.0 && large.x() > 0.0);
    assert!(small.y() > large.y());
    assert!(
        (small.width() - large.width()).abs() > f32::EPSILON
            || (small.height() - large.height()).abs() > f32::EPSILON
    );
}

struct OverflowApp;

impl UiApp for OverflowApp {
    type State = OverflowPolicy;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(policy: &Self::State) -> Element<Self::Action> {
        row(children![
            fixed(100.0, 50.0).id("overflow.content").with_layout(
                LayoutStyle::default()
                    .with_position(LayoutPosition::Absolute(LayoutInsets::new(
                        runenui_core::LayoutOffset::length(length(0.0)),
                        runenui_core::LayoutOffset::Auto,
                        runenui_core::LayoutOffset::Auto,
                        runenui_core::LayoutOffset::length(length(0.0)),
                    )))
                    .with_width(LayoutDimension::length(length(100.0)))
                    .with_height(LayoutDimension::length(length(50.0))),
            )
        ])
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Block)
                .with_width(LayoutDimension::length(length(40.0)))
                .with_height(LayoutDimension::length(length(20.0)))
                .with_overflow(OverflowStyle::all(*policy)),
        )
        .id("overflow.root")
        .into_element()
    }

    fn update(_: &mut Self::State, (): ()) {}
}

#[test]
fn logical_overflow_reports_box_content_and_scroll_extents_for_each_policy() {
    for policy in [
        OverflowPolicy::Visible,
        OverflowPolicy::Clip,
        OverflowPolicy::Scroll,
    ] {
        let mut runtime = AppRuntime::<OverflowApp>::mount(policy);
        let publication = publish(&mut runtime, 40.0, 20.0);
        let root = layout_node(publication.layout_report(), "overflow.root");
        assert_eq!(root.layout_extent(), size(40.0, 20.0));
        assert!(
            root.content_extent().width() >= 100.0,
            "policy {policy:?} content extent: {:?}",
            root.content_extent()
        );
        assert!(root.overflow().width());
        if policy == OverflowPolicy::Scroll {
            assert!(root.scrollable_extent().width() >= 100.0);
        }
    }
}
