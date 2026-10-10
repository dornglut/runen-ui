#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, Color, CommandOrigin, ContributionClip, Element, EventContext, EventPhase,
    HitContribution, HitContributionContext, HitRegion, LayoutDimension, LayoutStyle, LogicalDelta,
    LogicalLength, LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol, OverflowPolicy,
    OverflowStyle, PaintContribution, PaintContributionContext, PaintContributionItem,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    PresentationOrigin, PresentationRotation, PresentationScale, PresentationTransform,
    PresentationTranslation, SceneLayer, SceneShape, SemanticCommand, StyleEnvironment,
    SurfacePresentation, SurfacePresentationAlignment, SurfacePresentationAnchor,
    SurfacePresentationPlacement, SurfacePresentationSide, UiApp, UiEvent, UnitInterval, View,
    Widget, WidgetEventOutput, WidgetMeasure, WidgetMeasureInput, button, column, container, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, SurfaceBuildContext,
};

fn fixed(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(LogicalLength::from(width)))
        .with_height(LayoutDimension::length(LogicalLength::from(height)))
}

fn tight_context<'a>(
    environment: &'a StyleEnvironment,
    width: u16,
    height: u16,
) -> SurfaceBuildContext<'a> {
    SurfaceBuildContext::new(
        environment,
        LayoutConstraints::tight(LogicalSize::new(
            LogicalLength::from(width),
            LogicalLength::from(height),
        )),
    )
}

fn node<'a>(
    publication: &'a runenui_runtime::SurfacePublication,
    id: &str,
) -> &'a runenui_runtime::SurfaceNode {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|value| value.as_str() == id))
        .unwrap_or_else(|| unreachable!("fixture authored node is published"))
}

fn owner_translation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(4.0, 3.0)
            .unwrap_or_else(|_| unreachable!("fixture translation is finite")),
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

struct PlacementApp;

impl UiApp for PlacementApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let popup = button("popup")
            .on_activate(|| ())
            .id("popup")
            .with_layout(fixed(30, 20))
            .surface_presentation(
                SurfacePresentation::new(
                    SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom)
                        .with_alignment(SurfacePresentationAlignment::Start),
                )
                .with_fallback(
                    SurfacePresentationPlacement::new(SurfacePresentationSide::Top)
                        .with_alignment(SurfacePresentationAlignment::Start),
                ),
            )
            .into_element();
        let owner = column(vec![popup])
            .id("owner")
            .with_layout(fixed(40, 20))
            .presentation(owner_translation())
            .into_element();
        let spacer = text("")
            .id("spacer")
            .with_layout(fixed(1, 35))
            .into_element();
        column(vec![spacer, owner])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn presentation_is_out_of_flow_and_fallback_geometry_is_correlated() {
    let mut runtime = AppRuntime::<PlacementApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("controlled publication is admitted"));

    let owner = node(&publication, "owner");
    let popup = node(&publication, "popup");
    assert_eq!(owner.bounds().width(), 40.0);
    assert_eq!(owner.bounds().height(), 20.0);
    assert_eq!(popup.bounds().width(), 30.0);
    assert_eq!(popup.bounds().height(), 20.0);

    let presentation = popup
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("popup root publishes placement facts"));
    assert_eq!(presentation.candidate_index(), 1);
    assert_eq!(presentation.placement().side(), SurfacePresentationSide::Top);
    assert_eq!(presentation.anchor_bounds().x(), owner.bounds().x() + 4.0);
    assert_eq!(presentation.anchor_bounds().y(), owner.bounds().y() + 3.0);
    assert_eq!(presentation.anchor_bounds().width(), owner.bounds().width());
    assert_eq!(presentation.anchor_bounds().height(), owner.bounds().height());
    assert_eq!(presentation.placed_bounds().x(), owner.bounds().x() + 4.0);
    assert_eq!(presentation.placed_bounds().y(), owner.bounds().y() + 3.0 - 20.0);

    let semantic = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("popup"))
        .unwrap_or_else(|| unreachable!("popup semantic node is published"));
    assert_eq!(semantic.bounds(), presentation.visible_bounds());

    let center = LogicalPoint::new(
        presentation.visible_bounds().x() + 15.0,
        presentation.visible_bounds().y() + 10.0,
    )
    .unwrap_or_else(|_| unreachable!("fixture point is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(center),
        Some(popup.id()),
        "physical hit geometry must use the same projected placement"
    );
}

struct AnchorApp;

impl UiApp for AnchorApp {
    type State = SurfacePresentationAnchor;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(anchor: &Self::State) -> impl View<Self::Action> {
        let popup = button("anchored")
            .on_activate(|| ())
            .id("anchored")
            .with_layout(fixed(20, 10))
            .surface_presentation(SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
            )
            .with_anchor(*anchor))
            .into_element();
        column(vec![popup]).id("owner")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn surface_point_and_viewport_anchors_resolve_without_owner_geometry_authority() {
    let environment = StyleEnvironment::default();

    let point = LogicalPoint::new(70.0, 40.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));
    let mut point_runtime = AppRuntime::<AnchorApp>::mount(SurfacePresentationAnchor::SurfacePoint(point));
    let point_publication = point_runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("point-anchor publication is admitted"));
    let point_snapshot = node(&point_publication, "anchored")
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("point anchor resolves"));
    assert_eq!(point_snapshot.anchor_bounds().x(), 70.0);
    assert_eq!(point_snapshot.anchor_bounds().y(), 40.0);
    assert_eq!(point_snapshot.placed_bounds().x(), 60.0);
    assert_eq!(point_snapshot.placed_bounds().y(), 35.0);

    let owner_rect = LogicalRect::try_new(8.0, 12.0, 20.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture owner-local rect is valid"));
    let mut rect_runtime =
        AppRuntime::<AnchorApp>::mount(SurfacePresentationAnchor::OwnerRect(owner_rect));
    let rect_publication = rect_runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("owner-rect publication is admitted"));
    let rect_snapshot = node(&rect_publication, "anchored")
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("owner rect anchor resolves"));
    assert_eq!(rect_snapshot.anchor_bounds(), owner_rect);

    let mut viewport_runtime =
        AppRuntime::<AnchorApp>::mount(SurfacePresentationAnchor::SurfaceViewport);
    let viewport_publication = viewport_runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("viewport-anchor publication is admitted"));
    let viewport_snapshot = node(&viewport_publication, "anchored")
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("viewport anchor resolves"));
    assert_eq!(viewport_snapshot.anchor_bounds().width(), 100.0);
    assert_eq!(viewport_snapshot.anchor_bounds().height(), 60.0);
    assert_eq!(viewport_snapshot.placed_bounds().x(), 40.0);
    assert_eq!(viewport_snapshot.placed_bounds().y(), 25.0);
}

#[derive(Clone, Debug)]
struct LayerProbe {
    layer: SceneLayer,
    color: Color,
}

impl<Action> Widget<Action> for LayerProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(40_u8), LogicalLength::from(40_u8))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let rect = LogicalRect::try_new(0.0, 0.0, 40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture rect is valid"));
        PaintContribution::single(
            PaintContributionItem::fill(SceneShape::rect(rect), self.color.into())
                .with_layer(self.layer),
        )
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let rect = LogicalRect::try_new(0.0, 0.0, 40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture rect is valid"));
        HitContribution::new(vec![HitRegion::rect(rect).with_layer(self.layer)])
    }
}

impl<Action> ChildBearingWidget<Action> for LayerProbe {}


struct StackingApp;

impl UiApp for StackingApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let ordinary = Element::new(LayerProbe {
            layer: SceneLayer::new(10_000),
            color: Color::WHITE,
        })
        .id("ordinary");
        let presentation = Element::new(LayerProbe {
            layer: SceneLayer::new(-10_000),
            color: Color::BLACK,
        })
        .id("presentation")
        .surface_presentation(SurfacePresentation::new(
            SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
        )
        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
            LogicalPoint::new(20.0, 20.0)
                .unwrap_or_else(|_| unreachable!("fixture point is finite")),
        )));
        column(vec![ordinary, presentation])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn presentation_band_outranks_ordinary_scene_layers_for_paint_and_hit() {
    let mut runtime = AppRuntime::<StackingApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 80, 80))
        .unwrap_or_else(|_| unreachable!("stacking publication is admitted"));

    let ordinary = node(&publication, "ordinary");
    let presentation = node(&publication, "presentation");
    let point = LogicalPoint::new(10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));

    assert_eq!(
        publication.hit_test_scene().target_at(point),
        Some(presentation.id()),
        "presentation-band hit ordering must outrank an arbitrarily higher ordinary SceneLayer"
    );
    assert_ne!(ordinary.id(), presentation.id());

    let items = publication.paint_scene().items();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].layer(), SceneLayer::new(10_000));
    assert_eq!(items[1].layer(), SceneLayer::new(-10_000));
}


fn singular_presentation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::ZERO,
        PresentationScale::new(0.0, 1.0)
            .unwrap_or_else(|_| unreachable!("zero scale is an accepted singular transform")),
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

struct SingularAnchorApp;

impl UiApp for SingularAnchorApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let popup = button("withheld")
            .on_activate(|| ())
            .id("withheld")
            .with_layout(fixed(20, 20))
            .surface_presentation(SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom),
            ))
            .into_element();
        column(vec![popup])
            .id("singular-owner")
            .with_layout(fixed(40, 20))
            .presentation(singular_presentation())
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn owner_relative_anchor_fails_closed_for_singular_owner_projection() {
    let mut runtime = AppRuntime::<SingularAnchorApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 80, 60))
        .unwrap_or_else(|_| unreachable!("surface publication remains admissible"));

    let withheld = node(&publication, "withheld");
    assert!(withheld.surface_presentation().is_none());
    assert!(
        withheld
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "runenui.presentation.anchor-unavailable")
    );
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|semantic| semantic.name() != Some("withheld"))
    );
    let point = LogicalPoint::new(5.0, 25.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));
    assert_ne!(publication.hit_test_scene().target_at(point), Some(withheld.id()));
}

#[derive(Clone, Debug)]
struct ClipProbe;

impl Widget<()> for ClipProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(80_u8), LogicalLength::from(20_u8))
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let full = LogicalRect::try_new(0.0, 0.0, 80.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture rect is valid"));
        let local_clip = LogicalRect::try_new(0.0, 0.0, 50.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture clip is valid"));
        HitContribution::new(vec![
            HitRegion::rect(full)
                .with_clip(ContributionClip::identity(SceneShape::rect(local_clip))),
        ])
    }
}

struct ClipEscapeApp;

impl UiApp for ClipEscapeApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let popup = Element::new(ClipProbe)
            .id("clip-popup")
            .surface_presentation(SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
            )
            .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                LogicalPoint::new(40.0, 10.0)
                    .unwrap_or_else(|_| unreachable!("fixture point is finite")),
            )));
        let owner = column(vec![popup])
            .id("clip-owner")
            .with_layout(
                fixed(20, 20).with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
            .into_element();
        column(vec![owner])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn presentation_escapes_ancestor_clip_but_preserves_local_clip_and_surface_clip() {
    let mut runtime = AppRuntime::<ClipEscapeApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 60, 40))
        .unwrap_or_else(|_| unreachable!("clip publication is admitted"));
    let popup = node(&publication, "clip-popup");

    let inside_local = LogicalPoint::new(35.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(inside_local),
        Some(popup.id()),
        "presentation must escape its logical owner's 20px ancestor clip"
    );

    let outside_local = LogicalPoint::new(55.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));
    assert_ne!(
        publication.hit_test_scene().target_at(outside_local),
        Some(popup.id()),
        "presentation-local authored clip must remain authoritative"
    );

    let outside_surface = LogicalPoint::new(70.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));
    assert_ne!(
        publication.hit_test_scene().target_at(outside_surface),
        Some(popup.id()),
        "hard surface clip must constrain oversized projected content"
    );

    assert_eq!(
        popup
            .surface_presentation()
            .unwrap_or_else(|| unreachable!("presentation snapshot exists"))
            .visible_bounds()
            .max_x(),
        60.0
    );
}

struct NestedStackingApp;

impl UiApp for NestedStackingApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let nested = Element::new(LayerProbe {
            layer: SceneLayer::new(-500),
            color: Color::BLACK,
        })
        .id("nested")
        .surface_presentation(SurfacePresentation::new(
            SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
        )
        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
            LogicalPoint::new(20.0, 20.0)
                .unwrap_or_else(|_| unreachable!("fixture point is finite")),
        )));

        let ancestor = container(
            LayerProbe {
                layer: SceneLayer::new(500),
                color: Color::WHITE,
            },
            vec![nested],
        )
        .id("ancestor")
        .surface_presentation(SurfacePresentation::new(
            SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
        )
        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
            LogicalPoint::new(20.0, 20.0)
                .unwrap_or_else(|_| unreachable!("fixture point is finite")),
        )));

        column(vec![ancestor.into_element()])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn nested_presentation_root_is_an_independent_later_band_above_ancestor() {
    let mut runtime = AppRuntime::<NestedStackingApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 80, 80))
        .unwrap_or_else(|_| unreachable!("nested presentation publication is admitted"));
    let nested = node(&publication, "nested");
    let point = LogicalPoint::new(10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));

    assert_eq!(publication.hit_test_scene().target_at(point), Some(nested.id()));
    let items = publication.paint_scene().items();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].layer(), SceneLayer::new(500));
    assert_eq!(items[1].layer(), SceneLayer::new(-500));
}


#[derive(Clone, Copy)]
struct MovingState {
    x: f32,
}

#[derive(Clone, Copy)]
enum MovingAction {
    Move,
}

struct MovingPresentationApp;

impl UiApp for MovingPresentationApp {
    type State = MovingState;
    type Action = MovingAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let point = LogicalPoint::new(state.x, 20.0)
            .unwrap_or_else(|_| unreachable!("application-owned anchor is finite"));
        let popup = button("moving")
            .on_activate(|| MovingAction::Move)
            .id("moving")
            .key("moving")
            .with_layout(fixed(20, 10))
            .surface_presentation(
                SurfacePresentation::new(
                    SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
                )
                .with_anchor(SurfacePresentationAnchor::SurfacePoint(point)),
            )
            .into_element();
        column(vec![popup])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            MovingAction::Move => state.x = 60.0,
        }
    }
}

#[test]
fn placement_rebuild_retains_one_mounted_and_semantic_lifetime() {
    let mut runtime = AppRuntime::<MovingPresentationApp>::mount(MovingState { x: 20.0 });
    let environment = StyleEnvironment::default();
    let first = runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("initial moving publication is admitted"));
    let first_node = node(&first, "moving");
    let mounted_id = first_node.id().clone();
    let first_snapshot = first_node
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("initial presentation resolves"));
    let semantic_id = first
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|semantic| semantic.name() == Some("moving"))
        .unwrap_or_else(|| unreachable!("initial semantic node exists"))
        .id()
        .clone();

    runtime
        .submit_action(MovingAction::Move)
        .unwrap_or_else(|_| unreachable!("application action is admitted"));
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();
    assert!(report.is_quiescent());

    let second = runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("moved presentation publication is admitted"));
    let second_node = node(&second, "moving");
    let second_snapshot = second_node
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("moved presentation resolves"));
    let second_semantic = second
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|semantic| semantic.name() == Some("moving"))
        .unwrap_or_else(|| unreachable!("moved semantic node exists"));

    assert_eq!(second_node.id(), &mounted_id);
    assert_eq!(second_semantic.id(), &semantic_id);
    assert_ne!(first_snapshot.placed_bounds(), second_snapshot.placed_bounds());
    assert_eq!(second_snapshot.anchor_bounds().x(), 60.0);
}


struct ScrollAnchorApp;

impl UiApp for ScrollAnchorApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let popup = button("scroll popup")
            .on_activate(|| ())
            .id("scroll-popup")
            .with_layout(fixed(20, 10))
            .surface_presentation(SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom),
            ))
            .into_element();
        let owner = column(vec![popup])
            .id("scroll-owner")
            .with_layout(fixed(20, 20))
            .into_element();
        let filler = Element::new(LayerProbe {
            layer: SceneLayer::ZERO,
            color: Color::WHITE,
        })
        .id("scroll-filler")
        .with_layout(fixed(40, 60));

        column(vec![filler, owner])
            .id("scroll-root")
            .with_layout(
                fixed(40, 40).with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn owner_anchor_follows_accepted_scroll_without_inflating_scroll_extent() {
    let mut runtime = AppRuntime::<ScrollAnchorApp>::mount(());
    let environment = StyleEnvironment::default();
    let context = tight_context(&environment, 40, 40);
    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial scroll presentation is admitted"));

    let scroll_root = node(&initial, "scroll-root");
    let layout = initial
        .layout_report()
        .nodes()
        .iter()
        .find(|layout| layout.id() == scroll_root.id())
        .unwrap_or_else(|| unreachable!("scroll root has one layout report node"));
    assert_eq!(layout.scroll_viewport_extent().height(), 40.0);
    assert_eq!(
        layout.scrollable_extent().height(),
        80.0,
        "out-of-flow presentation content must not enlarge logical scroll extent"
    );

    let initial_anchor = node(&initial, "scroll-popup")
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("initial owner anchor resolves"))
        .anchor_bounds();

    let point = LogicalPoint::new(5.0, 5.0)
        .unwrap_or_else(|_| unreachable!("wheel point is finite"));
    let wheel = PointerEvent::new(
        PointerId::new(341).unwrap_or_else(|| unreachable!("fixture pointer is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Wheel,
        point,
        initial.input_context().clone(),
    )
    .with_scroll_delta(
        LogicalDelta::new(0.0, 10.0)
            .unwrap_or_else(|_| unreachable!("wheel delta is finite")),
    );
    runtime
        .submit_pointer(wheel)
        .unwrap_or_else(|_| unreachable!("wheel is admitted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX)).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();

    let scrolled = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("scrolled presentation is admitted"));
    let scrolled_anchor = node(&scrolled, "scroll-popup")
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("scrolled owner anchor resolves"))
        .anchor_bounds();

    assert_eq!(scrolled_anchor.x(), initial_anchor.x());
    assert_eq!(scrolled_anchor.y(), initial_anchor.y() - 10.0);
    assert_eq!(scrolled_anchor.width(), initial_anchor.width());
    assert_eq!(scrolled_anchor.height(), initial_anchor.height());
}


#[derive(Clone, Copy)]
enum SiblingStackAction {
    Reverse,
}

struct SiblingStackApp;

fn sibling_presentation(id: &'static str, color: Color) -> Element<SiblingStackAction> {
    Element::new(LayerProbe {
        layer: SceneLayer::ZERO,
        color,
    })
    .id(id)
    .key(id)
    .surface_presentation(
        SurfacePresentation::new(
            SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
        )
        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
            LogicalPoint::new(20.0, 20.0)
                .unwrap_or_else(|_| unreachable!("fixture point is finite")),
        )),
    )
}

impl UiApp for SiblingStackApp {
    type State = bool;
    type Action = SiblingStackAction;
    type HostProtocol = NoHostProtocol;

    fn root(reversed: &Self::State) -> impl View<Self::Action> {
        let first = sibling_presentation("sibling-first", Color::WHITE);
        let second = sibling_presentation("sibling-second", Color::BLACK);
        if *reversed {
            column(vec![second, first])
        } else {
            column(vec![first, second])
        }
    }

    fn update(reversed: &mut Self::State, action: Self::Action) {
        match action {
            SiblingStackAction::Reverse => *reversed = true,
        }
    }
}

#[test]
fn direct_presentation_siblings_follow_current_keyed_mounted_preorder() {
    let mut runtime = AppRuntime::<SiblingStackApp>::mount(false);
    let environment = StyleEnvironment::default();
    let context = tight_context(&environment, 80, 80);
    let point = LogicalPoint::new(10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture point is finite"));

    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial sibling publication is admitted"));
    let first_id = node(&initial, "sibling-first").id().clone();
    let second_id = node(&initial, "sibling-second").id().clone();
    assert_eq!(initial.hit_test_scene().target_at(point), Some(&second_id));

    runtime
        .submit_action(SiblingStackAction::Reverse)
        .unwrap_or_else(|_| unreachable!("application reorder action is admitted"));
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            )).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned()
            .is_quiescent()
    );

    let reordered = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("reordered sibling publication is admitted"));
    assert_eq!(node(&reordered, "sibling-first").id(), &first_id);
    assert_eq!(node(&reordered, "sibling-second").id(), &second_id);
    assert_eq!(
        reordered.hit_test_scene().target_at(point),
        Some(&first_id),
        "only the accepted keyed mounted preorder changes sibling presentation stacking"
    );
}

struct FocusProjectionApp;

impl UiApp for FocusProjectionApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let start = button("focus start")
            .on_activate(|| ())
            .id("focus-start")
            .with_layout(fixed(20, 20))
            .into_element();
        let projected = button("focus projected")
            .on_activate(|| ())
            .id("focus-projected")
            .with_layout(fixed(20, 20))
            .surface_presentation(
                SurfacePresentation::new(
                    SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
                )
                .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                    LogicalPoint::new(70.0, 10.0)
                        .unwrap_or_else(|_| unreachable!("fixture point is finite")),
                )),
            )
            .into_element();
        column(vec![start, projected])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn directional_focus_uses_projected_presentation_geometry() {
    let mut runtime = AppRuntime::<FocusProjectionApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("focus presentation publication is admitted"));
    let start = node(&publication, "focus-start").id().clone();
    let projected = node(&publication, "focus-projected").id().clone();
    let projected_snapshot = node(&publication, "focus-projected")
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("projected focus target has placement facts"));
    assert!(projected_snapshot.visible_bounds().x() > node(&publication, "focus-start").bounds().max_x());

    runtime
        .submit_command(
            start.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is admitted"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX)).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();
    assert_eq!(runtime.focus().focused_node(), Some(&start));

    runtime
        .submit_command(
            start,
            SemanticCommand::FocusRight,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("directional focus command is admitted"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX)).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&projected),
        "directional focus must rank the same projected geometry used by paint/hit/semantics"
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RouteFact {
    widget: &'static str,
    phase: EventPhase,
}

#[derive(Clone, Debug)]
struct RouteProbe {
    name: &'static str,
    facts: Rc<RefCell<Vec<RouteFact>>>,
}

impl Widget<()> for RouteProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if event
            .as_semantic_command()
            .is_some_and(|command| command.command() == SemanticCommand::OpenMenu)
        {
            self.facts.borrow_mut().push(RouteFact {
                widget: self.name,
                phase: context.phase(),
            });
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u8), LogicalLength::from(20_u8))
    }
}

impl ChildBearingWidget<()> for RouteProbe {}

struct RouteProjectionApp;

impl UiApp for RouteProjectionApp {
    type State = Rc<RefCell<Vec<RouteFact>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(facts: &Self::State) -> impl View<Self::Action> {
        let target = Element::new(RouteProbe {
            name: "target",
            facts: Rc::clone(facts),
        })
        .id("route-target")
        .surface_presentation(
            SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
            )
            .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                LogicalPoint::new(60.0, 20.0)
                    .unwrap_or_else(|_| unreachable!("fixture point is finite")),
            )),
        );
        container(
            RouteProbe {
                name: "owner",
                facts: Rc::clone(facts),
            },
            vec![target],
        )
        .id("route-owner")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn projected_target_keeps_ordinary_logical_routed_ancestry() {
    let facts = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<RouteProjectionApp>::mount(Rc::clone(&facts));
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("route presentation publication is admitted"));
    let target = node(&publication, "route-target").id().clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("route command is admitted"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX)).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();

    assert_eq!(
        facts.borrow().as_slice(),
        [
            RouteFact {
                widget: "owner",
                phase: EventPhase::Capture,
            },
            RouteFact {
                widget: "target",
                phase: EventPhase::Target,
            },
            RouteFact {
                widget: "owner",
                phase: EventPhase::Bubble,
            },
        ],
        "visual projection must not rewrite the ordinary mounted route"
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureMoveFact {
    routed_target: MountedNodeId,
    physical_target: Option<MountedNodeId>,
}

#[derive(Clone, Debug)]
struct CaptureProjectionProbe {
    capture_on_down: bool,
    moves: Rc<RefCell<Vec<CaptureMoveFact>>>,
}

impl Widget<()> for CaptureProjectionProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::Pointer(pointer)
                if self.capture_on_down && pointer.phase() == PointerPhase::Down =>
            {
                context.capture_pointer();
            }
            UiEvent::Pointer(pointer)
                if self.capture_on_down && pointer.phase() == PointerPhase::Move =>
            {
                self.moves.borrow_mut().push(CaptureMoveFact {
                    routed_target: context.original_target().clone(),
                    physical_target: context.physical_target().cloned(),
                });
            }
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u8), LogicalLength::from(20_u8))
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let rect = LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture rect is valid"));
        HitContribution::new(vec![HitRegion::rect(rect)])
    }
}

struct CaptureProjectionApp;

impl UiApp for CaptureProjectionApp {
    type State = Rc<RefCell<Vec<CaptureMoveFact>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(moves: &Self::State) -> impl View<Self::Action> {
        let capture = Element::new(CaptureProjectionProbe {
            capture_on_down: true,
            moves: Rc::clone(moves),
        })
        .id("capture-owner");
        let projected = Element::new(CaptureProjectionProbe {
            capture_on_down: false,
            moves: Rc::clone(moves),
        })
        .id("capture-projected")
        .surface_presentation(
            SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
            )
            .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                LogicalPoint::new(60.0, 10.0)
                    .unwrap_or_else(|_| unreachable!("fixture point is finite")),
            )),
        );
        column(vec![capture, projected])
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn projected_physical_target_does_not_steal_existing_pointer_capture() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<CaptureProjectionApp>::mount(Rc::clone(&moves));
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&tight_context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("capture presentation publication is admitted"));
    let capture = node(&publication, "capture-owner").id().clone();
    let projected = node(&publication, "capture-projected").id().clone();
    let context = publication.input_context().clone();

    let pointer_id = PointerId::new(341)
        .unwrap_or_else(|| unreachable!("fixture pointer identity is non-zero"));
    let down_point = LogicalPoint::new(10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture down point is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                down_point,
                context.clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("capture-start pointer is admitted"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX)).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();

    let move_point = LogicalPoint::new(60.0, 10.0)
        .unwrap_or_else(|_| unreachable!("fixture move point is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Move,
                move_point,
                context,
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|_| unreachable!("captured move is admitted"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX)).unwrap_or_else(|_| unreachable!("pump observation")).report().to_owned();

    assert_eq!(
        moves.borrow().as_slice(),
        [CaptureMoveFact {
            routed_target: capture,
            physical_target: Some(projected),
        }],
        "presentation hit testing may update the physical target but must not steal capture routing"
    );
}
