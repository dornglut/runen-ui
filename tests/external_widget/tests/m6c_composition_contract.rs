#![allow(refining_impl_trait)]

use runenui_core::{
    Color, ContributionClip, Element, ElementId, HitContribution, HitContributionContext,
    HitRegion, LogicalLength, LogicalPoint, LogicalRect, LogicalTransform, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, PaintPrimitive,
    PointerPolicy, Radius, SceneLayer, SceneOpacity, SceneShape, StyleTokens, UiApp, View, Widget,
    WidgetMeasure, children, column,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, PaintSceneItem, SurfaceBuildContext};

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("test rectangle is finite and non-negative"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

fn translation(x: f32, y: f32) -> LogicalTransform {
    LogicalTransform::translation(x, y)
        .unwrap_or_else(|_| unreachable!("test translation is finite"))
}

fn authored(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("test authored id is valid"))
}

fn assert_transform_components(actual: [f32; 6], expected: [f32; 6]) {
    assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
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

fn publish<App: UiApp>(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    let tokens = StyleTokens::new();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("test surface publication is admitted"))
}

#[derive(Clone, Copy, Debug)]
enum PaintOwnerKind {
    First,
    Second,
}

#[derive(Debug)]
struct PaintOwner(PaintOwnerKind);

impl Widget<()> for PaintOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let full = rect(0.0, 0.0, size.width(), size.height());
        match self.0 {
            PaintOwnerKind::First => {
                let opacity = SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("test opacity is valid"));
                let first_clip = ContributionClip::new(
                    SceneShape::rect(rect(0.0, 0.0, 8.0, 9.0)),
                    translation(4.0, 5.0),
                );
                let second_clip = ContributionClip::new(
                    SceneShape::rect(rect(0.0, 0.0, 10.0, 6.0)),
                    translation(6.0, 7.0),
                );
                PaintContribution::new(vec![
                    PaintContributionItem::fill_rect(full, Color::rgba(255, 0, 0, 255))
                        .with_transform(translation(2.0, 3.0))
                        .with_clip(first_clip)
                        .with_clip(second_clip)
                        .with_opacity(opacity)
                        .with_layer(SceneLayer::ZERO),
                    PaintContributionItem::fill_rect(full, Color::rgba(0, 255, 0, 255))
                        .with_layer(SceneLayer::ZERO),
                ])
            }
            PaintOwnerKind::Second => PaintContribution::new(vec![
                PaintContributionItem::fill_rect(full, Color::rgba(0, 0, 255, 255))
                    .with_layer(SceneLayer::new(-1)),
                PaintContributionItem::fill_rect(full, Color::WHITE).with_layer(SceneLayer::ZERO),
            ]),
        }
    }
}

struct PaintApp;

impl UiApp for PaintApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            Element::new(PaintOwner(PaintOwnerKind::First))
                .id("paint.a")
                .key("a"),
            Element::new(PaintOwner(PaintOwnerKind::Second))
                .id("paint.b")
                .key("b"),
        ])
        .key("root")
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn paint_scene_composes_self_contained_values_exact_order_and_conjunctive_clips() {
    let mut runtime = AppRuntime::<PaintApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();
    let items = scene.items();

    assert_eq!(items.len(), 4);
    assert_eq!(
        items
            .iter()
            .map(|item| item.primitive().color())
            .collect::<Vec<_>>(),
        vec![
            Color::rgba(0, 0, 255, 255),
            Color::rgba(255, 0, 0, 255),
            Color::rgba(0, 255, 0, 255),
            Color::WHITE,
        ]
    );
    assert_eq!(
        items.iter().map(PaintSceneItem::layer).collect::<Vec<_>>(),
        vec![
            SceneLayer::new(-1),
            SceneLayer::ZERO,
            SceneLayer::ZERO,
            SceneLayer::ZERO,
        ]
    );

    let first_owner = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored("paint.a")))
        .unwrap_or_else(|| unreachable!("first paint owner is published"));
    let red = &items[1];
    assert_transform_components(
        red.local_to_surface().components(),
        [
            1.0,
            0.0,
            0.0,
            1.0,
            first_owner.bounds().x() + 2.0,
            first_owner.bounds().y() + 3.0,
        ],
    );
    assert_eq!(red.opacity().get().to_bits(), 0.5_f32.to_bits());
    assert_eq!(red.clips().len(), 2);
    assert_transform_components(
        red.clips()[0].clip_to_surface().components(),
        [
            1.0,
            0.0,
            0.0,
            1.0,
            first_owner.bounds().x() + 4.0,
            first_owner.bounds().y() + 5.0,
        ],
    );
    assert_transform_components(
        red.clips()[1].clip_to_surface().components(),
        [
            1.0,
            0.0,
            0.0,
            1.0,
            first_owner.bounds().x() + 6.0,
            first_owner.bounds().y() + 7.0,
        ],
    );
    assert_eq!(
        red.clips()[0].shape(),
        SceneShape::rect(rect(0.0, 0.0, 8.0, 9.0))
    );

    let owner_x = first_owner.bounds().x();
    let owner_y = first_owner.bounds().y();
    assert!(fill_item_covers_surface_point(
        red,
        point(owner_x + 7.0, owner_y + 8.0)
    ));
    assert!(!fill_item_covers_surface_point(
        red,
        point(owner_x + 5.0, owner_y + 8.0)
    ));
    assert!(!fill_item_covers_surface_point(
        red,
        point(owner_x + 7.0, owner_y + 13.0)
    ));

    let green = &items[2];
    assert!(green.clips().is_empty());
    assert!(fill_item_covers_surface_point(
        green,
        point(owner_x + 1.0, owner_y + 1.0)
    ));
}

#[derive(Clone, Copy, Debug)]
enum OverlapHitKind {
    Target,
    Block,
}

#[derive(Debug)]
struct OverlapHitOwner(OverlapHitKind);

impl Widget<()> for OverlapHitOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        match self.0 {
            OverlapHitKind::Target => {
                HitContribution::new(vec![HitRegion::rect(rect(0.0, 0.0, 20.0, 20.0))])
            }
            OverlapHitKind::Block => HitContribution::new(vec![
                HitRegion::rect(rect(0.0, 0.0, 5.0, 20.0))
                    .with_transform(translation(0.0, -20.0))
                    .with_pointer_policy(PointerPolicy::Block),
            ]),
        }
    }
}

struct OverlapHitApp;

impl UiApp for OverlapHitApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            Element::new(OverlapHitOwner(OverlapHitKind::Target))
                .id("hit.a")
                .key("a"),
            Element::new(OverlapHitOwner(OverlapHitKind::Block))
                .id("hit.b")
                .key("b"),
        ])
        .key("root")
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn hit_scene_uses_layer_preorder_local_order_and_block_stops_resolution() {
    let mut runtime = AppRuntime::<OverlapHitApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.hit_test_scene();
    let target = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored("hit.a")))
        .unwrap_or_else(|| unreachable!("target hit owner is published"))
        .id();

    assert_eq!(scene.regions().len(), 2);
    assert_eq!(scene.regions()[0].pointer_policy(), PointerPolicy::Target);
    assert_eq!(scene.regions()[1].pointer_policy(), PointerPolicy::Block);
    assert_eq!(scene.target_at(point(2.0, 5.0)), None);
    assert_eq!(scene.target_at(point(10.0, 5.0)), Some(target));
}

#[derive(Debug)]
struct GeometryHitOwner;

impl Widget<()> for GeometryHitOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(50_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let singular_region = LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 35.0, 0.0)
            .unwrap_or_else(|_| unreachable!("singular test transform is finite"));
        let singular_clip = LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 40.0, 0.0)
            .unwrap_or_else(|_| unreachable!("singular test clip transform is finite"));
        let radius = Radius::all(LogicalLength::from(5_u16));

        HitContribution::new(vec![
            HitRegion::rounded_rect(rect(0.0, 0.0, 10.0, 10.0), radius),
            HitRegion::rect(rect(0.0, 0.0, 10.0, 10.0))
                .with_transform(translation(20.0, 0.0))
                .with_clip(ContributionClip::new(
                    SceneShape::rect(rect(0.0, 0.0, 4.0, 10.0)),
                    translation(22.0, 0.0),
                )),
            HitRegion::rect(rect(0.0, 0.0, 10.0, 10.0)).with_transform(singular_region),
            HitRegion::rect(rect(0.0, 0.0, 10.0, 10.0))
                .with_transform(translation(40.0, 0.0))
                .with_clip(ContributionClip::new(
                    SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
                    singular_clip,
                )),
        ])
    }
}

struct GeometryHitApp;

impl UiApp for GeometryHitApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(GeometryHitOwner)
            .id("geometry-hit")
            .key("root")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn hit_scene_applies_rounded_geometry_transforms_conjunctive_clips_and_singular_exclusion() {
    let mut runtime = AppRuntime::<GeometryHitApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.hit_test_scene();
    let target = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("geometry hit root is published"))
        .id();

    assert_eq!(scene.regions().len(), 4);
    assert_eq!(scene.target_at(point(0.0, 0.0)), None);
    assert_eq!(scene.target_at(point(5.0, 5.0)), Some(target));
    assert_eq!(scene.target_at(point(23.0, 5.0)), Some(target));
    assert_eq!(scene.target_at(point(28.0, 5.0)), None);
    assert_eq!(scene.target_at(point(35.0, 5.0)), None);
    assert_eq!(scene.target_at(point(45.0, 5.0)), None);

    assert_transform_components(
        scene.regions()[1].local_to_surface().components(),
        [1.0, 0.0, 0.0, 1.0, 20.0, 0.0],
    );
    assert_eq!(scene.regions()[1].clips().len(), 1);
    assert_transform_components(
        scene.regions()[1].clips()[0].clip_to_surface().components(),
        [1.0, 0.0, 0.0, 1.0, 22.0, 0.0],
    );
}
