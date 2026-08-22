#![allow(refining_impl_trait)]

use runenui_core::{
    Color, Element, ElementId, HitContribution, HitContributionContext, HitRegion, LogicalPoint,
    LogicalRect, LogicalTransform, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, PointerPolicy, SceneLayer, StyleTokens, UiApp, View, Widget,
    WidgetInvalidation, WidgetMeasure, WidgetUpdateContext, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, SurfaceBuildContext,
    SurfacePublication,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
}

fn overlay_transform(overlay_up: bool) -> LogicalTransform {
    if overlay_up {
        LogicalTransform::translation(0.0, -20.0)
            .unwrap_or_else(|_| unreachable!("test translation is finite"))
    } else {
        LogicalTransform::IDENTITY
    }
}

fn colors(name: &str) -> [Color; 3] {
    match name {
        "a" => [
            Color::rgba(10, 0, 0, 255),
            Color::rgba(11, 0, 0, 255),
            Color::rgba(12, 0, 0, 255),
        ],
        "b" => [
            Color::rgba(20, 0, 0, 255),
            Color::rgba(21, 0, 0, 255),
            Color::rgba(22, 0, 0, 255),
        ],
        _ => unreachable!("test owner name is known"),
    }
}

#[derive(Debug)]
struct OrderOwner {
    name: &'static str,
    overlay_up: bool,
}

impl Widget<OrderAction> for OrderOwner {
    type State = bool;

    fn create_state(&self) -> Self::State {
        self.overlay_up
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<OrderAction>) {
        if *state != self.overlay_up {
            *state = self.overlay_up;
            context.invalidate(WidgetInvalidation::PAINT | WidgetInvalidation::HIT_TEST);
        }
    }

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: runenui_core::LogicalLength::from(20_u16),
            height: runenui_core::LogicalLength::from(20_u16),
        }
    }

    fn paint(&self, state: &Self::State, _: PaintContributionContext) -> PaintContribution {
        let full = rect(0.0, 0.0, 20.0, 20.0);
        let transform = overlay_transform(*state);
        let [low, first, second] = colors(self.name);
        PaintContribution::new(vec![
            PaintContributionItem::fill_rect(full, low)
                .with_transform(transform)
                .with_layer(SceneLayer::new(-1)),
            PaintContributionItem::fill_rect(full, first)
                .with_transform(transform)
                .with_layer(SceneLayer::ZERO),
            PaintContributionItem::fill_rect(full, second)
                .with_transform(transform)
                .with_layer(SceneLayer::ZERO),
        ])
    }

    fn hit_test(&self, state: &Self::State, _: HitContributionContext) -> HitContribution {
        let full = rect(0.0, 0.0, 20.0, 20.0);
        let left = rect(0.0, 0.0, 10.0, 20.0);
        let transform = overlay_transform(*state);
        HitContribution::new(vec![
            HitRegion::rect(full)
                .with_transform(transform)
                .with_layer(SceneLayer::new(-1))
                .with_pointer_policy(PointerPolicy::Block),
            HitRegion::rect(full)
                .with_transform(transform)
                .with_layer(SceneLayer::ZERO),
            HitRegion::rect(left)
                .with_transform(transform)
                .with_layer(SceneLayer::ZERO)
                .with_pointer_policy(PointerPolicy::Block),
        ])
    }
}

#[derive(Clone, Copy, Debug)]
enum OrderAction {
    Swap,
}

#[derive(Debug)]
struct OrderState {
    order: [&'static str; 2],
}

struct OrderApp;

impl UiApp for OrderApp {
    type State = OrderState;
    type Action = OrderAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let children: Vec<_> = state
            .order
            .iter()
            .enumerate()
            .map(|(position, name)| {
                Element::new(OrderOwner {
                    name,
                    overlay_up: position == 1,
                })
                .id(format!("order.{name}"))
                .key(*name)
            })
            .collect();
        column(children).key("root").into_element()
    }

    fn update(state: &mut Self::State, OrderAction::Swap: Self::Action) {
        state.order.swap(0, 1);
    }
}

fn authored(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("test authored id is valid"))
}

fn node_id(publication: &SurfacePublication, authored_id: &str) -> MountedNodeId {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored(authored_id)))
        .unwrap_or_else(|| unreachable!("order owner is published"))
        .id()
        .clone()
}

fn publish(runtime: &mut AppRuntime<OrderApp>, tokens: &StyleTokens) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("order publication is admitted"))
}

fn paint_colors(publication: &SurfacePublication) -> Vec<Color> {
    publication
        .paint_scene()
        .items()
        .iter()
        .map(|item| {
            item.primitive()
                .color()
                .unwrap_or_else(|| unreachable!("order fixture carries literal colors"))
        })
        .collect()
}

fn assert_hit_order(
    publication: &SurfacePublication,
    first: &MountedNodeId,
    second: &MountedNodeId,
) {
    let regions = publication.hit_test_scene().regions();
    assert_eq!(regions.len(), 6);
    assert_eq!(
        regions
            .iter()
            .map(runenui_runtime::HitTestRegion::layer)
            .collect::<Vec<_>>(),
        vec![
            SceneLayer::new(-1),
            SceneLayer::new(-1),
            SceneLayer::ZERO,
            SceneLayer::ZERO,
            SceneLayer::ZERO,
            SceneLayer::ZERO,
        ]
    );
    assert_eq!(
        regions
            .iter()
            .map(runenui_runtime::HitTestRegion::target)
            .collect::<Vec<_>>(),
        vec![first, second, first, first, second, second]
    );
    assert_eq!(
        regions
            .iter()
            .map(runenui_runtime::HitTestRegion::pointer_policy)
            .collect::<Vec<_>>(),
        vec![
            PointerPolicy::Block,
            PointerPolicy::Block,
            PointerPolicy::Target,
            PointerPolicy::Block,
            PointerPolicy::Target,
            PointerPolicy::Block,
        ]
    );
    assert_eq!(
        publication.hit_test_scene().target_at(point(5.0, 5.0)),
        None
    );
    assert_eq!(
        publication.hit_test_scene().target_at(point(15.0, 5.0)),
        Some(second)
    );
}

#[test]
fn layer_preorder_and_local_order_follow_logical_reorder_not_retained_storage_order() {
    let mut runtime = AppRuntime::<OrderApp>::mount(OrderState { order: ["a", "b"] });
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let tokens = StyleTokens::new();

    let initial = publish(&mut runtime, &tokens);
    let a = node_id(&initial, "order.a");
    let b = node_id(&initial, "order.b");
    assert_eq!(
        paint_colors(&initial),
        vec![
            colors("a")[0],
            colors("b")[0],
            colors("a")[1],
            colors("a")[2],
            colors("b")[1],
            colors("b")[2]
        ]
    );
    assert_hit_order(&initial, &a, &b);

    runtime
        .submit_action(OrderAction::Swap)
        .unwrap_or_else(|_| unreachable!("swap action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let reordered = publish(&mut runtime, &tokens);
    assert_eq!(node_id(&reordered, "order.a"), a);
    assert_eq!(node_id(&reordered, "order.b"), b);
    assert_eq!(
        paint_colors(&reordered),
        vec![
            colors("b")[0],
            colors("a")[0],
            colors("b")[1],
            colors("b")[2],
            colors("a")[1],
            colors("a")[2]
        ]
    );
    assert_hit_order(&reordered, &b, &a);
}
