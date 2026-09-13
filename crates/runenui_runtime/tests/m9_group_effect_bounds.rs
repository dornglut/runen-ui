use runenui_core::{
    Brush, Color, ContributionClip, DropShadow, Element, IntoEffects, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionGroup, PaintContributionItem, ResourceKind, ResourceRef, SceneOpacity,
    SceneShape, StyleEnvironment, UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintDamage, PaintSceneBounds, PaintSceneGroupId,
    SurfaceBuildContext, SurfacePublication,
};

#[derive(Clone, Copy, Debug)]
enum Case {
    NestedShadows,
    RepeatedShadows,
    PositiveSpreadOffsetBlur,
    NegativeSpread,
    ZeroOpacity,
    ClipAfterEffects,
    UnboundedChildFiniteClip,
}

#[derive(Debug)]
struct BoundsPaint {
    case: Case,
}

impl Widget<()> for BoundsPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(24_u16), LogicalLength::from(24_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        match self.case {
            Case::NestedShadows => nested_shadows(),
            Case::RepeatedShadows => repeated_shadows(),
            Case::PositiveSpreadOffsetBlur => positive_spread_offset_blur(),
            Case::NegativeSpread => negative_spread(),
            Case::ZeroOpacity => zero_opacity(),
            Case::ClipAfterEffects => clip_after_effects(),
            Case::UnboundedChildFiniteClip => unbounded_child_finite_clip(),
        }
    }
}

struct App;

impl UiApp for App {
    type State = Case;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(BoundsPaint { case: *state })
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"))
}

fn leaf() -> PaintContributionItem {
    PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 0.0, 10.0, 10.0)),
        Brush::solid(Color::WHITE),
    )
}

fn shadow(offset_x: f32, offset_y: f32, sigma: f32, spread: f32) -> DropShadow {
    DropShadow::new(
        offset_x,
        offset_y,
        LogicalLength::new(sigma).unwrap_or_else(|_| unreachable!("sigma is valid")),
        spread,
        Color::TRANSPARENT,
    )
    .unwrap_or_else(|_| unreachable!("shadow is finite"))
}

fn nested_shadows() -> PaintContribution {
    let inner = PaintContributionGroup::new(vec![leaf().into()])
        .with_shadows(vec![shadow(10.0, 0.0, 0.0, 0.0)]);
    let outer = PaintContributionGroup::new(vec![inner.into()])
        .with_shadows(vec![shadow(10.0, 0.0, 0.0, 0.0)]);
    PaintContribution::from_entries(vec![outer.into()])
}

fn repeated_shadows() -> PaintContribution {
    let group = PaintContributionGroup::new(vec![leaf().into()]).with_shadows(vec![
        shadow(10.0, 0.0, 0.0, 0.0),
        shadow(10.0, 0.0, 0.0, 0.0),
    ]);
    PaintContribution::from_entries(vec![group.into()])
}

fn positive_spread_offset_blur() -> PaintContribution {
    let group = PaintContributionGroup::new(vec![leaf().into()])
        .with_shadows(vec![shadow(5.0, -3.0, 1.0, 2.0)]);
    PaintContribution::from_entries(vec![group.into()])
}

fn negative_spread() -> PaintContribution {
    let group = PaintContributionGroup::new(vec![leaf().into()])
        .with_shadows(vec![shadow(10.0, 0.0, 0.0, -2.0)]);
    PaintContribution::from_entries(vec![group.into()])
}

fn zero_opacity() -> PaintContribution {
    let group =
        PaintContributionGroup::new(vec![leaf().into()]).with_opacity(SceneOpacity::TRANSPARENT);
    PaintContribution::from_entries(vec![group.into()])
}

fn clip_after_effects() -> PaintContribution {
    let clip = ContributionClip::identity(SceneShape::rect(rect(15.0, 0.0, 5.0, 10.0)));
    let group = PaintContributionGroup::new(vec![leaf().into()])
        .with_shadows(vec![shadow(10.0, 0.0, 0.0, 0.0)])
        .with_clip(clip);
    PaintContribution::from_entries(vec![group.into()])
}

fn unbounded_child_finite_clip() -> PaintContribution {
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let run = PaintContributionItem::shaped_text_run(
        resource,
        LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
        Color::WHITE,
    )
    .unwrap_or_else(|_| unreachable!("resource kind matches"));
    let clip = ContributionClip::identity(SceneShape::rect(rect(3.0, 4.0, 5.0, 6.0)));
    let group = PaintContributionGroup::new(vec![run.into()]).with_clip(clip);
    PaintContribution::from_entries(vec![group.into()])
}

fn publish(case: Case) -> SurfacePublication {
    let mut runtime = AppRuntime::<App>::mount(case);
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled publication is admitted"))
}

fn root_group(publication: &SurfacePublication) -> PaintSceneGroupId {
    publication.paint_scene().root_entries()[0]
        .group_id()
        .unwrap_or_else(|| unreachable!("fixture root entry is a group"))
}

fn finite_group_bounds(publication: &SurfacePublication, group: PaintSceneGroupId) -> LogicalRect {
    let Some(PaintSceneBounds::Finite(bounds)) = publication.paint_scene().group_bounds(group)
    else {
        unreachable!("fixture group has finite conservative bounds");
    };
    bounds
}

fn finite_item_bounds(publication: &SurfacePublication, item_index: usize) -> LogicalRect {
    let Some(PaintSceneBounds::Finite(bounds)) = publication.paint_scene().item_bounds(item_index)
    else {
        unreachable!("fixture item has finite conservative bounds");
    };
    bounds
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 8.0 * f32::EPSILON);
}

#[test]
fn nested_group_effects_feed_parent_pre_shadow_coverage() {
    let publication = publish(Case::NestedShadows);
    let scene = publication.paint_scene();
    assert_eq!(scene.groups().len(), 2);

    let outer_id = root_group(&publication);
    let outer = scene
        .group(outer_id)
        .unwrap_or_else(|| unreachable!("outer group resolves"));
    let inner_id = outer.entries()[0]
        .group_id()
        .unwrap_or_else(|| unreachable!("outer child is inner group"));

    let item = finite_item_bounds(&publication, 0);
    let inner = finite_group_bounds(&publication, inner_id);
    let outer = finite_group_bounds(&publication, outer_id);

    assert_close(inner.x(), item.x());
    assert_close(inner.max_x(), item.max_x() + 10.0);
    assert_close(outer.x(), item.x());
    assert_close(outer.max_x(), item.max_x() + 20.0);
    assert_close(outer.y(), item.y());
    assert_close(outer.max_y(), item.max_y());
}

#[test]
fn ordered_shadows_each_derive_from_same_pre_shadow_child_coverage() {
    let publication = publish(Case::RepeatedShadows);
    let group_id = root_group(&publication);
    let item = finite_item_bounds(&publication, 0);
    let group = finite_group_bounds(&publication, group_id);

    assert_close(group.x(), item.x());
    assert_close(group.max_x(), item.max_x() + 10.0);
    assert_close(group.y(), item.y());
    assert_close(group.max_y(), item.max_y());
}

#[test]
fn positive_spread_offset_and_three_sigma_support_expand_conservatively() {
    let publication = publish(Case::PositiveSpreadOffsetBlur);
    let group_id = root_group(&publication);
    let item = finite_item_bounds(&publication, 0);
    let group = finite_group_bounds(&publication, group_id);

    assert!(group.x() <= item.x());
    assert_close(group.max_x(), item.max_x() + 10.0);
    assert_close(group.y(), item.y() - 8.0);
    assert_close(group.max_y(), item.max_y() + 2.0);
}

#[test]
fn negative_spread_retains_pre_shadow_aabb_before_offset() {
    let publication = publish(Case::NegativeSpread);
    let group_id = root_group(&publication);
    let item = finite_item_bounds(&publication, 0);
    let group = finite_group_bounds(&publication, group_id);

    assert_close(group.x(), item.x());
    assert_close(group.max_x(), item.max_x() + 10.0);
    assert_close(group.y(), item.y());
    assert_close(group.max_y(), item.max_y());
}

#[test]
fn zero_group_opacity_does_not_shrink_conservative_geometric_bounds() {
    let publication = publish(Case::ZeroOpacity);
    let group_id = root_group(&publication);
    let item = finite_item_bounds(&publication, 0);
    let group = finite_group_bounds(&publication, group_id);
    assert_eq!(group, item);
}

#[test]
fn group_clip_constrains_complete_child_plus_shadow_result() {
    let publication = publish(Case::ClipAfterEffects);
    let group_id = root_group(&publication);
    let group = finite_group_bounds(&publication, group_id);

    assert_close(group.x(), 15.0);
    assert_close(group.y(), 0.0);
    assert_close(group.width(), 5.0);
    assert_close(group.height(), 10.0);
}

#[test]
fn finite_group_clip_narrows_unbounded_child_coverage() {
    let publication = publish(Case::UnboundedChildFiniteClip);
    assert_eq!(
        publication.paint_scene().item_bounds(0),
        Some(PaintSceneBounds::Unbounded)
    );
    let group_id = root_group(&publication);
    assert_eq!(
        publication.paint_scene().group_bounds(group_id),
        Some(PaintSceneBounds::Finite(rect(3.0, 4.0, 5.0, 6.0)))
    );
}

#[test]
fn group_effect_bounds_do_not_change_m6_full_surface_damage_policy() {
    let publication = publish(Case::PositiveSpreadOffsetBlur);
    assert_eq!(
        publication.paint_publication().damage(),
        PaintDamage::FullSurface
    );
}
