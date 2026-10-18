#![allow(refining_impl_trait)]

//! `TreeView` reveal must use only M4 focus and the standard #344 scroll container.
use runenui_core::{
    CommandOrigin, FlexContainerStyle, FlexDirection, LayoutContainer, LayoutDimension,
    LayoutStyle, LogicalLength, NoHostProtocol, OverflowPolicy, OverflowStyle, SemanticCommand,
    UiApp, View, scroll_container, tree_item, tree_view,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

struct ScrollTreeApp;
impl UiApp for ScrollTreeApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(&(): &Self::State) -> impl View<Self::Action> {
        let item_layout = || {
            LayoutStyle::default()
                .with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_direction(FlexDirection::Column),
                ))
                .with_height(LayoutDimension::Length(LogicalLength::from(20_u8)))
                .with_width(LayoutDimension::Length(LogicalLength::from(120_u8)))
        };
        let tree = tree_view([
            tree_item("alpha", "Alpha", Vec::new())
                .id("tree.reveal.alpha")
                .with_layout(item_layout()),
            tree_item("bravo", "Bravo", Vec::new())
                .id("tree.reveal.bravo")
                .with_layout(item_layout()),
            tree_item("charlie", "Charlie", Vec::new())
                .id("tree.reveal.charlie")
                .with_layout(item_layout()),
        ])
        .id("tree.reveal.root");
        let overflow = OverflowStyle::all(OverflowPolicy::Scroll);
        scroll_container(tree, overflow)
            .id("tree.reveal.viewport")
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(LayoutDimension::Length(LogicalLength::from(120_u8)))
                    .with_height(LayoutDimension::Length(LogicalLength::from(20_u8)))
                    .with_overflow(overflow),
            )
    }
    fn update(&mut (): &mut Self::State, (): Self::Action) {}
}
fn settle(runtime: &mut AppRuntime<ScrollTreeApp>) {
    let batch = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("TreeView pump observation"));
    assert!(batch.report().is_quiescent());
}
fn id(runtime: &mut AppRuntime<ScrollTreeApp>, value: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(value).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("TreeView fixture node exists"))
        .id()
        .clone()
}
fn command(
    runtime: &mut AppRuntime<ScrollTreeApp>,
    target: MountedNodeId,
    request: SemanticCommand,
) {
    runtime
        .submit_command(target, request, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("public focus command admitted"));
    settle(runtime);
}
#[test]
fn tree_view_focus_navigation_reveals_from_generic_scroll_container() {
    let mut runtime = AppRuntime::<ScrollTreeApp>::mount(());
    settle(&mut runtime);
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(120.0, 20.0)
                .unwrap_or_else(|_| unreachable!("finite reveal surface")),
        ))
        .unwrap_or_else(|error| unreachable!("TreeView publishes: {error:?}"));
    let one = id(&mut runtime, "tree.reveal.alpha");
    let two = id(&mut runtime, "tree.reveal.bravo");
    let three = id(&mut runtime, "tree.reveal.charlie");
    let viewport = id(&mut runtime, "tree.reveal.viewport");
    command(&mut runtime, one.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, one, SemanticCommand::FocusDown);
    assert_eq!(runtime.focus().focused_node(), Some(&two));
    command(&mut runtime, two, SemanticCommand::FocusDown);
    assert_eq!(runtime.focus().focused_node(), Some(&three));
    let offset = runtime
        .index()
        .node(&viewport)
        .unwrap_or_else(|| unreachable!("viewport still mounted"))
        .interaction()
        .scroll_offset();
    assert_eq!(offset, (0.0, 40.0));
}
