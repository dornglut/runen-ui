#![allow(refining_impl_trait)]
//! ComboBox uses the standard bounded scroll viewport without another M4 focus owner.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, LayoutBound, LayoutDimension, LayoutStyle,
    LogicalLength, NoHostProtocol, OverflowPolicy, SemanticCommand, SemanticRelationshipKind,
    SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Popup(ComboBoxRequest),
    Choose(u8),
}
#[derive(Default)]
struct State {
    open: bool,
    chosen: u8,
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let options = (0..18_u8).map(|index| {
            option_item(format!("Option {index:02}"), state.chosen == index)
                .id(format!("viewport.option.{index}"))
                .with_layout(
                    LayoutStyle::default()
                        .with_width(LayoutDimension::Length(LogicalLength::from(160_u16)))
                        .with_height(LayoutDimension::Length(LogicalLength::from(24_u8))),
                )
                .on_activate(move || Action::Choose(index))
        });
        combo_box(
            "Choose",
            ElementId::new("viewport.list").unwrap_or_else(|_| unreachable!()),
            options,
            Action::Popup,
        )
        .id("viewport.owner")
        .open(state.open)
        .popup_max_height(LogicalLength::from(96_u8))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Popup(request) => state.open = matches!(request, ComboBoxRequest::Open),
            Action::Choose(index) => {
                state.chosen = index;
                state.open = false;
            }
        }
    }
}

fn settle(app: &mut AppRuntime<App>) {
    assert!(
        app.pump(PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX))
            .is_quiescent()
    );
}
fn id(app: &AppRuntime<App>, text: &str) -> MountedNodeId {
    let authored = ElementId::new(text).unwrap_or_else(|_| unreachable!());
    app.index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("current authored node"))
        .id()
        .clone()
}
fn command(app: &mut AppRuntime<App>, node: MountedNodeId, request: SemanticCommand) {
    app.submit_command(node, request, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("live semantic command admitted"));
    settle(app);
}
fn publish(app: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(350.0, 320.0).unwrap_or_else(|_| unreachable!()),
    ))
    .unwrap_or_else(|error| unreachable!("bounded ComboBox publishes: {error:?}"))
}

#[test]
fn long_combo_popup_is_bounded_and_ordinary_exact_option_reveal_preserves_owner_focus() {
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    command(&mut app, owner.clone(), SemanticCommand::Expand);
    assert!(app.state().open);
    assert_eq!(app.focus().focused_node(), Some(&owner));

    let list = id(&app, "viewport.list");
    let last = id(&app, "viewport.option.17");
    let viewport = app.index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("ListBox is inside standard scroll viewport"))
        .clone();
    let mounted = app.index();
    let scroll = mounted.node(&viewport).unwrap_or_else(|| unreachable!());
    assert_eq!(
        scroll.layout().max_height(),
        LayoutBound::Length(LogicalLength::from(96_u8))
    );
    assert_eq!(scroll.layout().overflow().vertical(), OverflowPolicy::Scroll);
    assert_eq!(scroll.layout().overflow().horizontal(), OverflowPolicy::Clip);
    assert!(!scroll.is_focusable(), "popup scroll viewport is not a second focus owner");
    assert!(!mounted.node(&last).unwrap_or_else(|| unreachable!()).is_focusable());
    let first = publish(&mut app);
    let bounds = first.frame().node(&viewport).unwrap_or_else(|| unreachable!()).bounds();
    let last_bounds = first.frame().node(&last).unwrap_or_else(|| unreachable!()).bounds();
    assert!(bounds.height() > 0.0 && bounds.height() <= 96.0);
    assert!(last_bounds.y() + last_bounds.height() > bounds.y() + bounds.height(),
        "long list has content outside the bounded viewport");
    assert_eq!(app.index().node(&viewport).unwrap_or_else(|| unreachable!())
        .interaction().scroll_offset(), (0.0, 0.0));

    // This confirms the standard #344 targetable runtime command and viewport.
    // ComboBox active-item navigation must separately gain a safe targeted emission
    // seam before M11CTRL-88 can be accepted.
    command(&mut app, last, SemanticCommand::ScrollIntoView);
    assert!(app.index().node(&viewport).unwrap_or_else(|| unreachable!())
        .interaction().scroll_offset().1 > 0.0);
    let after = publish(&mut app);
    let semantic = after.semantic_publication().snapshot();
    let owner_semantic = semantic.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox semantics"));
    assert!(owner_semantic.relationships().iter()
        .any(|link| link.kind() == SemanticRelationshipKind::Controls));
    assert_eq!(app.state().chosen, 0, "reveal must never commit a selection");
    assert_eq!(app.focus().focused_node(), Some(&owner),
        "exact Option scrolling must retain real ComboBox focus");
}

#[test]
fn closed_popup_unmounts_both_listbox_and_its_scroll_viewport() {
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    assert!(app.index().nodes().iter().all(|node|
        node.authored_id().is_none_or(|id|
            id.as_str() != "viewport.list" && id.as_str() != "viewport.option.17"
        )
    ));
    let publication = publish(&mut app);
    assert!(publication.semantic_publication().snapshot().nodes().iter().all(|node|
        node.role() != SemanticRole::ListBox && node.role() != SemanticRole::Option
    ));
}
