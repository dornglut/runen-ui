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
    navigation_with_disabled_holes: bool,
    all_disabled: bool,
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
                .enabled(
                    !state.all_disabled
                        && (!state.navigation_with_disabled_holes
                            || index == 2
                            || index == 14),
                )
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
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
}
fn id(app: &mut AppRuntime<App>, text: &str) -> MountedNodeId {
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
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    command(&mut app, owner.clone(), SemanticCommand::Expand);
    assert!(app.state().open);
    assert_eq!(app.focus().focused_node(), Some(&owner));

    let list = id(&mut app, "viewport.list");
    let last = id(&mut app, "viewport.option.17");
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
    assert!(
        app.index().nodes().iter().all(|node| {
            node.authored_id().is_none_or(|id| {
                id.as_str() != "viewport.list" && id.as_str() != "viewport.option.17"
            })
        })
    );
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
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let old = id(&mut app, "viewport.option.17");
    let old_list = id(&mut app, "viewport.list");
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
    assert!(
        app.index()
            .nodes()
            .iter()
            .all(|node| node.id() != &old_viewport)
    );
    let Err(error) = app.submit_command(
        old.clone(),
        SemanticCommand::ScrollIntoView,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("exact retired Option cannot be accepted by #344")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::StaleTarget);
    assert_eq!(error.unaccepted().target(), &old);
    assert_eq!(
        error.unaccepted().command(),
        SemanticCommand::ScrollIntoView
    );

    app.submit_action(Action::Popup(ComboBoxRequest::Open))
        .unwrap_or_else(|_| unreachable!("application reopens standard popup"));
    settle(&mut app);
    let current = id(&mut app, "viewport.option.17");
    assert_ne!(
        current, old,
        "reopened same authored Option gets a new generation"
    );
    let list = id(&mut app, "viewport.list");
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
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let foreign_option = id(&mut foreign, "viewport.option.17");
    let list = id(&mut app, "viewport.list");
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
        RuntimeConfig::default().with_limits(
            runenui_runtime::RuntimeLimits::default()
                .with_waiting_envelopes(64)
                .with_transaction_outputs(1),
        ),
    );
    settle(&mut app);
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let last = id(&mut app, "viewport.option.17");
    let list = id(&mut app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("bounded popup viewport"))
        .clone();
    let _ = publish(&mut app);
    // Initial publication may itself queue the selected Option's reveal.
    // Settle it before explicitly filling all ordinary command slots.
    settle(&mut app);
    for _ in 0..64 {
        app.submit_command(
            owner.clone(),
            SemanticCommand::ScrollIntoView,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("all configured public FIFO slots available"));
    }
    let Err(error) = app.submit_command(
        last.clone(),
        SemanticCommand::ScrollIntoView,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("full canonical FIFO rejects the excess exact Option command")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::Full);
    assert_eq!(error.unaccepted().target(), &last);
    assert_eq!(
        error.unaccepted().command(),
        SemanticCommand::ScrollIntoView
    );
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

fn active_option_name(app: &mut AppRuntime<App>) -> Option<String> {
    let published = publish(app);
    let semantics = published.semantic_publication().snapshot();
    let combo = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)?;
    let target = combo
        .relationships()
        .iter()
        .find(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)?
        .target();
    semantics
        .nodes()
        .iter()
        .find(|node| node.id() == target)
        .and_then(|node| node.name())
        .map(str::to_owned)
}

#[test]
fn sparse_enabled_candidates_navigate_without_disabled_or_out_of_range_active_targets() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        navigation_with_disabled_holes: true,
        ..State::default()
    });
    settle(&mut app);
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    // Option 00 is application-selected but disabled. Active eligibility is
    // independent of committed value, even across many disabled Options.
    assert_eq!(active_option_name(&mut app).as_deref(), Some("Option 02"));
    command(&mut app, owner.clone(), SemanticCommand::FocusUp);
    assert_eq!(
        active_option_name(&mut app).as_deref(),
        Some("Option 02"),
        "Previous clamps at the first enabled Option"
    );
    command(&mut app, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(active_option_name(&mut app).as_deref(), Some("Option 14"));
    command(&mut app, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(
        active_option_name(&mut app).as_deref(),
        Some("Option 14"),
        "Next clamps at the last enabled Option"
    );
    command(&mut app, owner.clone(), SemanticCommand::FocusGroupFirst);
    assert_eq!(active_option_name(&mut app).as_deref(), Some("Option 02"));
    command(&mut app, owner.clone(), SemanticCommand::FocusGroupLast);
    assert_eq!(active_option_name(&mut app).as_deref(), Some("Option 14"));
    command(&mut app, owner.clone(), SemanticCommand::FocusUp);
    assert_eq!(active_option_name(&mut app).as_deref(), Some("Option 02"));
    assert_eq!(app.focus().focused_node(), Some(&owner));
    assert_eq!(app.state().chosen, 0);
}

#[test]
fn all_disabled_options_have_no_active_descendant_or_keyboard_selection_action() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        all_disabled: true,
        ..State::default()
    });
    settle(&mut app);
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    assert_eq!(active_option_name(&mut app), None);
    for direction in [
        SemanticCommand::FocusDown,
        SemanticCommand::FocusUp,
        SemanticCommand::FocusGroupFirst,
        SemanticCommand::FocusGroupLast,
    ] {
        command(&mut app, owner.clone(), direction);
        assert_eq!(
            active_option_name(&mut app),
            None,
            "no enabled Option may be used as a synthetic active descendant"
        );
    }
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert_eq!(app.state().chosen, 0);
    assert_eq!(app.focus().focused_node(), Some(&owner));
    assert!(
        app.state().open,
        "activation of no eligible Option is inert"
    );
}

#[test]
fn mounted_popup_navigation_routes_exact_active_option_reveal_through_standard_scroll() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        ..State::default()
    });
    settle(&mut app);
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let list = id(&mut app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("scroll viewport"))
        .clone();
    let target = id(&mut app, "viewport.option.17");
    let _ = publish(&mut app);
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1,
        0.0
    );
    command(&mut app, owner.clone(), SemanticCommand::FocusGroupLast);
    assert!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1
            > 0.0,
        "navigation must queue exact active Option reveal, not just update semantics"
    );
    let publication = publish(&mut app);
    let semantic = publication.semantic_publication().snapshot();
    let combo = semantic
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("one ComboBox owner"));
    let active = combo
        .relationships()
        .iter()
        .find(|relationship| relationship.kind() == SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("last navigated proxy"));
    assert_eq!(
        semantic.node(active.target()).and_then(|node| node.name()),
        Some("Option 17")
    );
    assert_eq!(app.focus().focused_node(), Some(&owner));
    assert_eq!(app.state().chosen, 0);
    assert_eq!(app.state().open, true);
    assert_eq!(
        app.index()
            .node(&target)
            .unwrap_or_else(|| unreachable!())
            .is_focusable(),
        false
    );
}

#[test]
fn first_published_open_combo_reveals_deep_selected_option_after_mount_and_geometry() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        chosen: 17,
        ..State::default()
    });
    settle(&mut app);
    let list = id(&mut app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("standard scroll owner"))
        .clone();
    let deepest = id(&mut app, "viewport.option.17");
    let first = publish(&mut app);
    let owner_bounds = first
        .frame()
        .node(&viewport)
        .unwrap_or_else(|| unreachable!("published viewport"))
        .bounds();
    let target_bounds = first
        .frame()
        .node(&deepest)
        .unwrap_or_else(|| unreachable!("published deep Option"))
        .bounds();
    assert!(target_bounds.max_y() > owner_bounds.max_y());
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1,
        0.0,
        "publication does not mutate scroll outside the canonical FIFO"
    );
    settle(&mut app);
    assert!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1
            > 0.0
    );
    let after = publish(&mut app);
    assert_eq!(app.state().chosen, 17);
    assert!(app.state().open);
    assert_eq!(
        app.focus().focused_node(),
        None,
        "reveal cannot synthesize real focus"
    );
    let options = after
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .filter(|node| node.role() == SemanticRole::Option)
        .count();
    assert_eq!(options, 18);
    let offset = app
        .index()
        .node(&viewport)
        .unwrap_or_else(|| unreachable!())
        .interaction()
        .scroll_offset()
        .1;
    // A repeated unchanged publication cannot fabricate another reveal event.
    let _ = publish(&mut app);
    settle(&mut app);
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1,
        offset
    );
}

#[test]
fn close_and_reopen_before_an_intermediate_publication_reveals_new_option_generation() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        chosen: 17,
        ..State::default()
    });
    settle(&mut app);
    let old_option = id(&mut app, "viewport.option.17");
    let _ = publish(&mut app);
    settle(&mut app);
    app.submit_action(Action::Popup(ComboBoxRequest::Close))
        .unwrap_or_else(|_| unreachable!("close application state"));
    settle(&mut app);
    assert!(
        app.index()
            .nodes()
            .iter()
            .all(|node| node.id() != &old_option)
    );
    // No surface publication occurs in the closed state: the retained semantic
    // snapshot still describes the earlier open popup.
    app.submit_action(Action::Popup(ComboBoxRequest::Open))
        .unwrap_or_else(|_| unreachable!("reopen application state"));
    settle(&mut app);
    let new_option = id(&mut app, "viewport.option.17");
    assert_ne!(new_option, old_option);
    let list = id(&mut app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("fresh standard viewport"))
        .clone();
    let _ = publish(&mut app);
    settle(&mut app);
    assert!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1
            > 0.0
    );
    assert_eq!(app.state().chosen, 17);
}

#[test]
fn queued_reveal_is_never_retargeted_after_original_option_is_retired() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        ..State::default()
    });
    settle(&mut app);
    let owner = id(&mut app, "viewport.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let retired = id(&mut app, "viewport.option.17");
    let _ = publish(&mut app);

    app.submit_command(
        owner.clone(),
        SemanticCommand::FocusGroupLast,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("navigation admission"));
    app.submit_action(Action::Popup(ComboBoxRequest::Close))
        .unwrap_or_else(|_| unreachable!("close admission"));
    let batch = app
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|error| unreachable!("original navigation: {error:?}"));
    assert!(batch.processed_through().is_some());
    assert!(app.state().open);
    settle(&mut app);
    assert!(!app.state().open);
    assert!(app.index().nodes().iter().all(|node| node.id() != &retired));

    app.submit_action(Action::Popup(ComboBoxRequest::Open))
        .unwrap_or_else(|_| unreachable!("reopen admission"));
    settle(&mut app);
    let current = id(&mut app, "viewport.option.17");
    assert_ne!(current, retired);
    let list = id(&mut app, "viewport.list");
    let viewport = app
        .index()
        .node(&list)
        .and_then(|node| node.parent())
        .unwrap_or_else(|| unreachable!("new scroll viewport"))
        .clone();
    let _ = publish(&mut app);
    settle(&mut app);
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1,
        0.0
    );
    assert_eq!(app.state().chosen, 0);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}
