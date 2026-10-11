#![allow(refining_impl_trait)]
//! ComboBox uses the standard bounded scroll viewport without another M4 focus owner.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, LayoutBound, LayoutDimension, LayoutStyle,
    LogicalLength, NoHostProtocol, OverflowPolicy, SemanticCommand, SemanticRelationshipKind,
    SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, RuntimeConfig, SubmitCommandErrorKind,
    SurfaceBuildContext,
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

#[test]
fn closed_popup_rejects_retired_exact_option_scroll_without_retargeting_reopened_generation() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        ..State::default()
    });
    settle(&mut app);
    let owner = id(&app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let old = id(&app, "viewport.option.17");
    let old_list = id(&app, "viewport.list");
    let old_viewport = app
        .index()
        .node(&old_list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("original bounded scroll viewport"))
        .clone();
    let _ = publish(&mut app);
    app.submit_action(Action::Popup(ComboBoxRequest::Close))
        .unwrap_or_else(|_| unreachable!("application closes standard popup"));
    settle(&mut app);
    assert!(app.index().nodes().iter().all(|node| node.id() != &old));
    assert!(app.index().nodes().iter().all(|node| node.id() != &old_viewport));
    let Err(error) = app.submit_command(
        old.clone(),
        SemanticCommand::ScrollIntoView,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("exact retired Option cannot be accepted by #344")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::StaleTarget);
    assert_eq!(error.unaccepted().target(), &old);
    assert_eq!(error.unaccepted().command(), SemanticCommand::ScrollIntoView);

    app.submit_action(Action::Popup(ComboBoxRequest::Open))
        .unwrap_or_else(|_| unreachable!("application reopens standard popup"));
    settle(&mut app);
    let current = id(&app, "viewport.option.17");
    assert_ne!(current, old, "reopened same authored Option gets a new generation");
    let list = id(&app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("new bounded scroll viewport"))
        .clone();
    let _ = publish(&mut app);
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!("new viewport"))
            .interaction()
            .scroll_offset(),
        (0.0, 0.0),
        "the old Option's rejected reveal cannot scroll a new generation"
    );
    assert_eq!(app.focus().focused_node(), Some(&owner));
    assert_eq!(app.state().chosen, 0);
}

#[test]
fn exact_option_scroll_from_another_runtime_fails_closed_without_stealing_focus() {
    let state = || State {
        open: true,
        ..State::default()
    };
    let mut app = AppRuntime::<App>::mount(state());
    let mut foreign = AppRuntime::<App>::mount(state());
    settle(&mut app);
    settle(&mut foreign);
    let owner = id(&app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let foreign_option = id(&foreign, "viewport.option.17");
    let list = id(&app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("bounded local scroll viewport"))
        .clone();
    let _ = publish(&mut app);
    let Err(error) = app.submit_command(
        foreign_option.clone(),
        SemanticCommand::ScrollIntoView,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("foreign runtime target cannot pass exact-target admission")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::ForeignTarget);
    assert_eq!(error.unaccepted().target(), &foreign_option);
    settle(&mut app);
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!("local viewport remains mounted"))
            .interaction()
            .scroll_offset(),
        (0.0, 0.0)
    );
    assert_eq!(app.focus().focused_node(), Some(&owner));
    assert_eq!(app.state().chosen, 0);
}

#[test]
fn full_public_command_queue_rejects_exact_option_reveal_without_alternate_scroll_path() {
    let mut app = AppRuntime::<App>::mount_with_config(
        State {
            open: true,
            ..State::default()
        },
        RuntimeConfig::default().with_queue_capacity(4),
    );
    settle(&mut app);
    let owner = id(&app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let last = id(&app, "viewport.option.17");
    let list = id(&app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("bounded popup viewport"))
        .clone();
    let _ = publish(&mut app);
    for _ in 0..4 {
        app.submit_command(
            owner.clone(),
            SemanticCommand::ScrollIntoView,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("four public FIFO slots available"));
    }
    let Err(error) = app.submit_command(
        last.clone(),
        SemanticCommand::ScrollIntoView,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("full canonical FIFO rejects fifth exact Option command")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::Full);
    assert_eq!(error.unaccepted().target(), &last);
    assert_eq!(error.unaccepted().command(), SemanticCommand::ScrollIntoView);
    settle(&mut app);
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!("viewport is still mounted"))
            .interaction()
            .scroll_offset(),
        (0.0, 0.0),
        "a rejected exact Option reveal cannot bypass the canonical queue"
    );
    assert_eq!(app.focus().focused_node(), Some(&owner));
    assert_eq!(app.state().chosen, 0);
}
