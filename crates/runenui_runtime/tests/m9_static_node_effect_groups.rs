use runenui_core::{
    Brush, Color, DropShadow, Element, IntoEffects, LogicalLength, LogicalRect, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, PaintPrimitive, SceneLayer,
    SceneOpacity, SceneShape, StyleEnvironment, UiApp, View, Widget, WidgetMeasure,
    WidgetMeasureInput, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintSceneEntry, PaintSceneGroupId, SurfaceBuildContext,
    SurfacePublication,
};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
        .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"))
}

const fn color(red: u8) -> Color {
    Color::rgba(red, 0, 0, 255)
}

#[derive(Debug)]
struct LayeredPaint {
    entries: &'static [(i64, u8)],
}

impl Widget<()> for LayeredPaint {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u16), LogicalLength::from(20_u16))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(
            self.entries
                .iter()
                .map(|(layer, red)| {
                    PaintContributionItem::fill(SceneShape::rect(rect()), Brush::solid(color(*red)))
                        .with_layer(SceneLayer::new(*layer))
                })
                .collect(),
        )
    }
}

struct GroupedApp;

impl UiApp for GroupedApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let opacity =
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled opacity is valid"));
        let transparent_shadow =
            DropShadow::new(2.0, 3.0, LogicalLength::ZERO, 0.0, Color::TRANSPARENT)
                .unwrap_or_else(|_| unreachable!("controlled shadow is finite"));

        column(vec![
            column(vec![Element::new(LayeredPaint {
                entries: &[(-1, 10), (1, 12)],
            })])
            .opacity(opacity)
            .key("isolated")
            .into_element(),
            Element::new(LayeredPaint {
                entries: &[(0, 20)],
            })
            .key("outside"),
        ])
        .shadows(vec![transparent_shadow])
        .key("root")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

struct UngroupedApp;

impl UiApp for UngroupedApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(vec![
            Element::new(LayeredPaint {
                entries: &[(-1, 10), (1, 12)],
            }),
            Element::new(LayeredPaint {
                entries: &[(0, 20)],
            }),
        ])
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

struct SiblingGroupsApp;

impl UiApp for SiblingGroupsApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let opacity_a =
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled opacity is valid"));
        let opacity_b =
            SceneOpacity::new(0.75).unwrap_or_else(|_| unreachable!("controlled opacity is valid"));
        column(vec![
            column(vec![Element::new(LayeredPaint {
                entries: &[(-2, 10), (2, 12)],
            })])
            .opacity(opacity_a)
            .key("group-a")
            .into_element(),
            column(vec![Element::new(LayeredPaint {
                entries: &[(-1, 20), (1, 22)],
            })])
            .opacity(opacity_b)
            .key("group-b")
            .into_element(),
        ])
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

struct EmptyGroupApp;

impl UiApp for EmptyGroupApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(Vec::<Element<()>>::new()).opacity(
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("controlled opacity is valid")),
        )
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn publish<App: UiApp<State = (), Action = (), HostProtocol = NoHostProtocol> + 'static>(
    runtime: &mut AppRuntime<App>,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled publication is admitted"))
}

fn item_reds(publication: &SurfacePublication) -> Vec<u8> {
    publication
        .paint_scene()
        .items()
        .iter()
        .map(|item| match item.primitive() {
            PaintPrimitive::Fill {
                shape: SceneShape::Rect(_),
                brush: Brush::Solid(color),
            } => color.red(),
            _ => unreachable!("fixture contains only solid rectangle fills"),
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
        .unwrap_or_else(|| unreachable!("fixture entry is a group"))
}

#[test]
fn static_node_effect_groups_contract_at_first_member_without_changing_pre_group_items() {
    let mut runtime = AppRuntime::<GroupedApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(item_reds(&publication), vec![10, 20, 12]);
    assert_eq!(scene.groups().len(), 2);
    assert_eq!(scene.root_entries().len(), 1);

    let root_group_id = only_group(scene.root_entries()[0]);
    let root_group = scene
        .group(root_group_id)
        .unwrap_or_else(|| unreachable!("root group reference resolves in this scene"));
    assert_eq!(root_group.parent(), None);
    assert_eq!(root_group.opacity(), SceneOpacity::OPAQUE);
    assert_eq!(root_group.shadows().len(), 1);
    assert_eq!(root_group.shadows()[0].color(), Color::TRANSPARENT);
    assert!(root_group.clips().is_empty());

    assert_eq!(root_group.entries().len(), 2);
    let opacity_group_id = only_group(root_group.entries()[0]);
    assert_eq!(root_group.entries()[1].item_index(), Some(1));

    let opacity_group = scene
        .group(opacity_group_id)
        .unwrap_or_else(|| unreachable!("nested group reference resolves in this scene"));
    assert_eq!(opacity_group.parent(), Some(root_group_id));
    assert_eq!(
        opacity_group.opacity(),
        SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!())
    );
    assert!(opacity_group.shadows().is_empty());
    assert_eq!(entry_items(opacity_group.entries()), vec![Some(0), Some(2)]);

    assert_eq!(scene.items()[0].group(), Some(opacity_group_id));
    assert_eq!(scene.items()[1].group(), Some(root_group_id));
    assert_eq!(scene.items()[2].group(), Some(opacity_group_id));
}

#[test]
fn sibling_groups_contract_by_first_member_and_descendant_layers_do_not_escape() {
    let mut runtime = AppRuntime::<SiblingGroupsApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(item_reds(&publication), vec![10, 20, 22, 12]);
    assert_eq!(scene.groups().len(), 2);
    assert_eq!(scene.root_entries().len(), 2);

    let early_anchor_id = only_group(scene.root_entries()[0]);
    let late_anchor_id = only_group(scene.root_entries()[1]);
    let early_group = scene
        .group(early_anchor_id)
        .unwrap_or_else(|| unreachable!("first sibling group resolves"));
    let late_group = scene
        .group(late_anchor_id)
        .unwrap_or_else(|| unreachable!("second sibling group resolves"));

    assert_eq!(early_group.parent(), None);
    assert_eq!(late_group.parent(), None);
    assert_eq!(entry_items(early_group.entries()), vec![Some(0), Some(3)]);
    assert_eq!(entry_items(late_group.entries()), vec![Some(1), Some(2)]);
    assert_eq!(scene.items()[0].group(), Some(early_anchor_id));
    assert_eq!(scene.items()[3].group(), Some(early_anchor_id));
    assert_eq!(scene.items()[1].group(), Some(late_anchor_id));
    assert_eq!(scene.items()[2].group(), Some(late_anchor_id));
}

#[test]
fn identity_defaults_preserve_exact_ungrouped_m6_root_order() {
    let mut runtime = AppRuntime::<UngroupedApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert_eq!(item_reds(&publication), vec![10, 20, 12]);
    assert!(scene.groups().is_empty());
    assert_eq!(
        entry_items(scene.root_entries()),
        vec![Some(0), Some(1), Some(2)]
    );
    assert!(scene.items().iter().all(|item| item.group().is_none()));
}

#[test]
fn static_effect_group_with_no_admitted_descendant_paint_is_omitted() {
    let mut runtime = AppRuntime::<EmptyGroupApp>::mount(());
    let publication = publish(&mut runtime);
    let scene = publication.paint_scene();

    assert!(scene.items().is_empty());
    assert!(scene.groups().is_empty());
    assert!(scene.root_entries().is_empty());
}
