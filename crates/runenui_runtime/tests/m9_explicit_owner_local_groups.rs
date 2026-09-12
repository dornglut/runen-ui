#![allow(refining_impl_trait)]

use runenui_core::{
    Brush, ChildBearingWidget, Color, ContributionClip, Element, FontFamilyName, GenericFontFamily,
    IntoEffects, LogicalLength, LogicalRect, LogicalTransform, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionGroup, PaintContributionItem, PaintPrimitive,
    PresentationOrigin, PresentationRotation, PresentationScale, PresentationTransform,
    PresentationTranslation, SceneLayer, SceneOpacity, SceneShape, StyleEnvironment, UiApp,
    UnitInterval, View, Widget, WidgetMeasure, WidgetMeasureInput, container,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintSceneEntry, PaintSceneGroupId, SurfaceBuildContext,
    SurfacePublication,
};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
        .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"))
}

fn fill(red: u8, layer: i64) -> PaintContributionItem {
    PaintContributionItem::fill(
        SceneShape::rect(rect()),
        Brush::solid(Color::rgba(red, 0, 0, 255)),
    )
    .with_layer(SceneLayer::new(layer))
}

fn item_reds(publication: &SurfacePublication) -> Vec<u8> {
    publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| match item.primitive() {
            PaintPrimitive::Fill {
                shape: SceneShape::Rect(_),
                brush: Brush::Solid(color),
            } => Some(color.red()),
            _ => None,
        })
        .collect()
}

fn entry_items(entries: &[PaintSceneEntry]) -> Vec<Option<usize>> {
    entries
        .iter()
        .copied()
        .map(PaintSceneEntry::item_index)
        .collect()
}

fn only_group(entry: PaintSceneEntry) -> PaintSceneGroupId {
    entry
        .group_id()
        .unwrap_or_else(|| unreachable!("controlled entry is a group"))
}

fn publish<App: UiApp + 'static>(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled publication is admitted"))
}

fn diagnostic_codes(publication: &SurfacePublication) -> Vec<&str> {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("controlled root is published"))
        .diagnostics()
        .iter()
        .map(runenui_core::WidgetDiagnostic::code)
        .collect()
}

fn presentation(
    translation_x: f32,
    translation_y: f32,
    scale_x: f32,
    scale_y: f32,
) -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(translation_x, translation_y)
            .unwrap_or_else(|_| unreachable!("controlled translation is finite")),
        PresentationScale::new(scale_x, scale_y)
            .unwrap_or_else(|_| unreachable!("controlled scale is finite")),
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

#[derive(Debug)]
struct NestedExplicitPaint;

impl Widget<()> for NestedExplicitPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let nested = PaintContributionGroup::new(vec![fill(20, 2).into()]).with_opacity(
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled opacity is valid")),
        );
        let outer = PaintContributionGroup::new(vec![fill(10, -2).into(), nested.into()]);
        let empty =
            PaintContributionGroup::new(vec![PaintContributionGroup::new(Vec::new()).into()]);
        PaintContribution::from_entries(vec![empty.into(), outer.into(), fill(30, 0).into()])
    }
}

struct NestedExplicitApp;

impl UiApp for NestedExplicitApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(NestedExplicitPaint)
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn nested_explicit_groups_use_first_member_contraction_without_layer_escape() {
    let mut runtime = AppRuntime::<NestedExplicitApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(item_reds(&publication), vec![10, 30, 20]);
    assert_eq!(scene.groups().len(), 2, "empty explicit groups are omitted");
    assert_eq!(scene.root_entries().len(), 2);

    let outer_id = only_group(scene.root_entries()[0]);
    assert_eq!(scene.root_entries()[1].item_index(), Some(1));
    let outer = scene
        .group(outer_id)
        .unwrap_or_else(|| unreachable!("outer explicit group resolves"));
    assert_eq!(outer.parent(), None);
    assert_eq!(outer.opacity(), SceneOpacity::OPAQUE);
    assert_eq!(outer.entries().len(), 2);
    assert_eq!(outer.entries()[0].item_index(), Some(0));

    let nested_id = only_group(outer.entries()[1]);
    let nested = scene
        .group(nested_id)
        .unwrap_or_else(|| unreachable!("nested explicit group resolves"));
    assert_eq!(nested.parent(), Some(outer_id));
    assert_eq!(entry_items(nested.entries()), vec![Some(2)]);
    assert_eq!(
        nested.opacity(),
        SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!())
    );

    assert_eq!(scene.items()[0].group(), Some(outer_id));
    assert_eq!(scene.items()[1].group(), None);
    assert_eq!(scene.items()[2].group(), Some(nested_id));
}

#[derive(Debug)]
struct ExplicitOwnerContainer;

impl Widget<()> for ExplicitOwnerContainer {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::from_entries(vec![
            PaintContributionGroup::new(vec![fill(10, 0).into()]).into(),
        ])
    }
}

impl ChildBearingWidget<()> for ExplicitOwnerContainer {}

#[derive(Debug)]
struct ForeignChildPaint;

impl Widget<()> for ForeignChildPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::single(fill(20, 0))
    }
}

struct ExplicitInsideNodeEffectApp;

impl UiApp for ExplicitInsideNodeEffectApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        container(
            ExplicitOwnerContainer,
            vec![Element::new(ForeignChildPaint)],
        )
        .opacity(
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled opacity is valid")),
        )
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn explicit_owner_group_nests_inside_node_effect_without_capturing_mounted_child_paint() {
    let mut runtime = AppRuntime::<ExplicitInsideNodeEffectApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(item_reds(&publication), vec![10, 20]);
    assert_eq!(scene.groups().len(), 2);
    assert_eq!(scene.root_entries().len(), 1);

    let node_group_id = only_group(scene.root_entries()[0]);
    let node_group = scene
        .group(node_group_id)
        .unwrap_or_else(|| unreachable!("runtime node-effect group resolves"));
    assert_eq!(
        node_group.opacity(),
        SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!())
    );
    assert_eq!(node_group.entries().len(), 2);

    let explicit_group_id = only_group(node_group.entries()[0]);
    assert_eq!(node_group.entries()[1].item_index(), Some(1));
    let explicit_group = scene
        .group(explicit_group_id)
        .unwrap_or_else(|| unreachable!("explicit group resolves"));
    assert_eq!(explicit_group.parent(), Some(node_group_id));
    assert_eq!(entry_items(explicit_group.entries()), vec![Some(0)]);

    assert_eq!(scene.items()[0].group(), Some(explicit_group_id));
    assert_eq!(scene.items()[1].group(), Some(node_group_id));
}

#[derive(Debug)]
struct TextAndExplicitPaint;

impl Widget<()> for TextAndExplicitPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: "grouped text".to_owned(),
        }
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::from_entries(vec![
            PaintContributionGroup::new(vec![fill(10, 0).into()]).into(),
        ])
    }
}

struct TextAndExplicitApp;

impl UiApp for TextAndExplicitApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(TextAndExplicitPaint).opacity(
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled opacity is valid")),
        )
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn runtime_generated_shaped_text_stays_outside_widget_authored_group_but_inside_node_effect() {
    let mut runtime = AppRuntime::<TextAndExplicitApp>::mount(());
    assert!(runtime.register_text_font_bytes(CANTARELL.to_vec()).is_ok());
    let families = [FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled font family is valid"))];
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &families)
            .is_ok()
    );

    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();
    assert_eq!(scene.groups().len(), 2);
    assert!(scene.items().len() >= 2);
    assert_eq!(item_reds(&publication), vec![10]);

    let node_group_id = only_group(scene.root_entries()[0]);
    let node_group = scene
        .group(node_group_id)
        .unwrap_or_else(|| unreachable!("node group resolves"));
    let explicit_group_id = only_group(node_group.entries()[0]);
    let explicit_group = scene
        .group(explicit_group_id)
        .unwrap_or_else(|| unreachable!("explicit group resolves"));
    assert_eq!(entry_items(explicit_group.entries()), vec![Some(0)]);
    assert_eq!(scene.items()[0].group(), Some(explicit_group_id));

    let shaped_indices = scene
        .items()
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matches!(item.primitive(), PaintPrimitive::ShapedTextRun(_)).then_some(index)
        })
        .collect::<Vec<_>>();
    assert!(!shaped_indices.is_empty());
    for index in shaped_indices {
        assert_eq!(scene.items()[index].group(), Some(node_group_id));
        assert!(
            node_group
                .entries()
                .iter()
                .any(|entry| entry.item_index() == Some(index))
        );
    }
}

#[derive(Clone, Copy)]
struct ClipGroupState {
    clip_transform: LogicalTransform,
    item_transform: LogicalTransform,
    presentation: PresentationTransform,
}

#[derive(Debug)]
struct ClipGroupOwner {
    clip_transform: LogicalTransform,
    item_transform: LogicalTransform,
}

impl Widget<()> for ClipGroupOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let group = PaintContributionGroup::new(vec![
            fill(10, 0).with_transform(self.item_transform).into(),
        ])
        .with_clip(ContributionClip::new(
            SceneShape::rect(rect()),
            self.clip_transform,
        ));
        PaintContribution::from_entries(vec![group.into()])
    }
}

impl ChildBearingWidget<()> for ClipGroupOwner {}

struct ClipGroupApp;

impl UiApp for ClipGroupApp {
    type State = ClipGroupState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            ClipGroupOwner {
                clip_transform: state.clip_transform,
                item_transform: state.item_transform,
            },
            Vec::<Element<()>>::new(),
        )
        .presentation(state.presentation)
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn group_clip_uses_owner_presentation_path_without_inheriting_child_item_transform() {
    let clip_transform = LogicalTransform::translation(3.0, 4.0)
        .unwrap_or_else(|_| unreachable!("controlled transform is finite"));
    let item_transform = LogicalTransform::translation(7.0, 8.0)
        .unwrap_or_else(|_| unreachable!("controlled transform is finite"));
    let node_presentation = presentation(10.0, 20.0, 1.0, 1.0);
    let state = ClipGroupState {
        clip_transform,
        item_transform,
        presentation: node_presentation,
    };
    let mut runtime = AppRuntime::<ClipGroupApp>::mount(state);
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();
    let frame = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("controlled root is published"));

    let presentation_transform = node_presentation
        .resolve_in_box(frame.bounds().size())
        .unwrap_or_else(|_| unreachable!("controlled presentation resolves"));
    let placement = LogicalTransform::translation(frame.bounds().x(), frame.bounds().y())
        .unwrap_or_else(|_| unreachable!("controlled placement is finite"));
    let owner_to_surface = presentation_transform
        .then(placement)
        .unwrap_or_else(|_| unreachable!("controlled owner transform composes"));
    let expected_clip = clip_transform
        .then(owner_to_surface)
        .unwrap_or_else(|_| unreachable!("controlled group clip composes"));
    let expected_item = item_transform
        .then(owner_to_surface)
        .unwrap_or_else(|_| unreachable!("controlled item transform composes"));

    assert_eq!(scene.items().len(), 1);
    assert_eq!(scene.groups().len(), 1);
    let group_id = only_group(scene.root_entries()[0]);
    let group = scene
        .group(group_id)
        .unwrap_or_else(|| unreachable!("explicit group resolves"));
    assert_eq!(group.clips().len(), 1);
    assert_eq!(group.clips()[0].clip_to_surface(), expected_clip);
    assert_eq!(scene.items()[0].local_to_surface(), expected_item);
    assert_ne!(group.clips()[0].clip_to_surface(), expected_item);
    assert!(diagnostic_codes(&publication).is_empty());
}

#[test]
fn singular_group_clip_is_retained_as_empty_coverage_with_diagnostic_and_no_fallback() {
    let singular = LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)
        .unwrap_or_else(|_| unreachable!("controlled singular transform is finite"));
    let mut runtime = AppRuntime::<ClipGroupApp>::mount(ClipGroupState {
        clip_transform: singular,
        item_transform: LogicalTransform::IDENTITY,
        presentation: presentation(0.0, 0.0, 1.0, 1.0),
    });
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(scene.items().len(), 1);
    assert_eq!(scene.groups().len(), 1);
    let group = scene
        .group(only_group(scene.root_entries()[0]))
        .unwrap_or_else(|| unreachable!("singular-clip group remains published"));
    assert_eq!(group.clips().len(), 1);
    assert!(group.clips()[0].clip_to_surface().inverse().is_none());
    let sample = runenui_core::LogicalPoint::new(1.0, 1.0)
        .unwrap_or_else(|_| unreachable!("controlled sample is finite"));
    assert!(!group.clips()[0].contains_surface_point(sample));
    assert_eq!(
        diagnostic_codes(&publication),
        vec!["runenui.scene.paint-group-clip-transform-non-invertible"]
    );
}

#[derive(Debug)]
struct OverflowGroupClipOwner;

impl Widget<()> for OverflowGroupClipOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let huge_scale = LogicalTransform::try_new(f32::MAX, 0.0, 0.0, 1.0, 0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("maximum finite scale is accepted"));
        let group = PaintContributionGroup::new(vec![fill(10, 0).into()])
            .with_clip(ContributionClip::new(SceneShape::rect(rect()), huge_scale));
        PaintContribution::from_entries(vec![group.into(), fill(20, 0).into()])
    }
}

impl ChildBearingWidget<()> for OverflowGroupClipOwner {}

struct OverflowGroupClipApp;

impl UiApp for OverflowGroupClipApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        container(OverflowGroupClipOwner, Vec::<Element<()>>::new())
            .presentation(presentation(0.0, 0.0, 2.0, 1.0))
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn non_finite_group_clip_composition_excludes_only_that_explicit_subtree() {
    let mut runtime = AppRuntime::<OverflowGroupClipApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(item_reds(&publication), vec![20]);
    assert!(scene.groups().is_empty());
    assert_eq!(entry_items(scene.root_entries()), vec![Some(0)]);
    assert_eq!(
        diagnostic_codes(&publication),
        vec!["runenui.scene.paint-group-clip-transform-non-finite"]
    );
}
