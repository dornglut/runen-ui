#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;

use runenui_core::{
    ApplicationCommand, ApplicationCommandId, CommandBinding, CommandOrigin, CommandScope, Menu,
    MenuBar, MenuButton, MenuItem, MenuItemCheckbox, MenuItemRadio, NoHostProtocol,
    PresentationDismissReason, SemanticCheckedState, SemanticCommand, SemanticOrientation,
    SemanticRelationshipKind, SemanticRole, UiApp, View,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Toggle,
    ToggleCheckbox,
    SelectRadio,
    Selected,
    Saved,
    ExpandSubmenu,
    CollapseSubmenu,
    Dismiss(PresentationDismissReason),
}

#[derive(Clone, Debug, Default)]
struct Model {
    menu: MenuMounts,
    checkbox: bool,
    radio: bool,
    selected: usize,
    saved: usize,
    dismissals: Vec<PresentationDismissReason>,
}

#[derive(Clone, Debug, Default)]
struct MenuMounts {
    open: bool,
    submenu_open: bool,
}

struct MenuApp;

fn save_command() -> ApplicationCommandId {
    ApplicationCommandId::new("menu-save")
        .unwrap_or_else(|_| unreachable!("static menu command is valid"))
}

impl UiApp for MenuApp {
    type State = Model;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut button = MenuButton::new("File", state.menu.open)
            .id("menu-button")
            .on_activate(|| Action::Toggle)
            .on_expand(|| Action::Toggle)
            .on_collapse(|| Action::Toggle);

        if state.menu.open {
            let mut more = MenuItem::new("More")
                .id("submenu-owner")
                .submenu_expanded(state.menu.submenu_open)
                .on_expand(|| Action::ExpandSubmenu)
                .on_collapse(|| Action::CollapseSubmenu);
            if state.menu.submenu_open {
                more = more.with_submenu(
                    Menu::new(vec![
                        MenuItem::new("Nested action")
                            .id("nested-action")
                            .on_activate(|| Action::Selected),
                    ])
                    .submenu()
                    .id("nested-menu")
                    .on_back(|| Action::CollapseSubmenu)
                    .on_dismiss(Action::Dismiss),
                    true,
                );
            }
            let menu = Menu::new(vec![
                MenuItem::new("Save")
                    .id("save-item")
                    .command(save_command())
                    .into_element(),
                MenuItemCheckbox::new(
                    "Autosave",
                    if state.checkbox {
                        SemanticCheckedState::Checked
                    } else {
                        SemanticCheckedState::Unchecked
                    },
                )
                .id("checkbox-item")
                .on_activate(|| Action::ToggleCheckbox)
                .into_element(),
                MenuItemRadio::new("Layout A", state.radio)
                    .id("radio-item")
                    .on_activate(|| Action::SelectRadio)
                    .into_element(),
                MenuItem::new("Unavailable")
                    .id("disabled-item")
                    .disabled()
                    .on_activate(|| Action::Selected)
                    .into_element(),
                more.into_element(),
            ])
            .id("file-menu")
            .on_dismiss(Action::Dismiss);
            button = button.with_submenu(menu, true);
        }

        CommandScope::new(
            [CommandBinding::new(
                ApplicationCommand::new(save_command(), true),
                || Action::Saved,
            )],
            vec![
                button.into_element(),
                MenuBar::new(vec![
                    MenuButton::new("View", false)
                        .id("bar-view")
                        .on_activate(|| Action::Selected)
                        .into_element(),
                    MenuButton::new("Tools", false)
                        .id("bar-tools")
                        .on_expand(|| Action::Selected)
                        .on_activate(|| Action::Selected)
                        .into_element(),
                ])
                .id("menu-bar")
                .into_element(),
            ],
        )
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Toggle => state.menu.open = !state.menu.open,
            Action::ToggleCheckbox => state.checkbox = !state.checkbox,
            Action::SelectRadio => state.radio = true,
            Action::Selected => state.selected += 1,
            Action::Saved => state.saved += 1,
            Action::ExpandSubmenu => state.menu.submenu_open = true,
            Action::CollapseSubmenu => state.menu.submenu_open = false,
            Action::Dismiss(reason) => {
                state.menu.open = false;
                state.menu.submenu_open = false;
                state.dismissals.push(reason);
            }
        }
    }
}

fn settle(harness: &mut TestHarness<MenuApp>) {
    assert_eq!(
        harness
            .run_until_idle(SettleBudget::new(
                NonZeroUsize::new(12).unwrap_or(NonZeroUsize::MIN),
                PumpBudget::new(128, 128, 128, 128),
            ))
            .outcome(),
        SettleOutcome::Idle
    );
}

fn target(harness: &TestHarness<MenuApp>, id: &str) -> runenui_core::MountedNodeId {
    harness
        .publication()
        .unwrap_or_else(|| unreachable!("test frame is published"))
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|value| value.as_str() == id))
        .unwrap_or_else(|| unreachable!("authored menu node is mounted"))
        .id()
        .clone()
}

fn command(harness: &mut TestHarness<MenuApp>, id: &str, value: SemanticCommand) {
    let mounted = target(harness, id);
    assert!(
        harness
            .submit_command(mounted, value, CommandOrigin::programmatic())
            .is_ok()
    );
    settle(harness);
    assert!(harness.publish().is_ok());
}

fn open_menu(harness: &mut TestHarness<MenuApp>) {
    assert!(harness.publish().is_ok());
    command(harness, "menu-button", SemanticCommand::Activate);
    assert!(harness.state().menu.open);
}

#[test]
fn menu_button_open_close_and_exact_controls_are_application_owned() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    assert!(h.publish().is_ok());
    let closed = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let button = closed
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::Button && n.name() == Some("File"))
        .unwrap_or_else(|| unreachable!());
    assert_eq!(button.state().expanded(), Some(false));

    open_menu(&mut h);
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let owner = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::Button && n.name() == Some("File"))
        .unwrap_or_else(|| unreachable!());
    let menu = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::Menu)
        .unwrap_or_else(|| unreachable!());
    assert_eq!(owner.state().expanded(), Some(true));
    assert!(
        owner
            .relationships()
            .iter()
            .any(|r| { r.kind() == SemanticRelationshipKind::Controls && r.target() == menu.id() })
    );
    assert_eq!(menu.orientation(), Some(SemanticOrientation::Vertical));

    command(
        &mut h,
        "file-menu",
        SemanticCommand::PresentationDismiss(PresentationDismissReason::CancelOrBack),
    );
    assert!(!h.state().menu.open);
    assert_eq!(
        h.state().dismissals,
        vec![PresentationDismissReason::CancelOrBack]
    );
    assert!(
        h.query_semantics(&SemanticQuery::new().with_role(SemanticRole::Menu))
            .unwrap_or_else(|_| unreachable!())
            .is_empty()
    );
}

#[test]
fn menu_checked_items_and_disabled_discoverability_preserve_app_authority() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    open_menu(&mut h);
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    assert!(snapshot.nodes().iter().any(|n| {
        n.role() == SemanticRole::MenuItemCheckbox
            && n.state().checked() == Some(SemanticCheckedState::Unchecked)
    }));
    assert!(snapshot.nodes().iter().any(|n| {
        n.role() == SemanticRole::MenuItemRadio
            && n.state().checked() == Some(SemanticCheckedState::Unchecked)
    }));
    command(&mut h, "checkbox-item", SemanticCommand::Activate);
    assert!(h.state().checkbox);
    command(&mut h, "radio-item", SemanticCommand::Activate);
    assert!(h.state().radio);
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    assert!(snapshot.nodes().iter().any(|n| {
        n.role() == SemanticRole::MenuItemCheckbox
            && n.state().checked() == Some(SemanticCheckedState::Checked)
    }));
    assert!(snapshot.nodes().iter().any(|n| {
        n.role() == SemanticRole::MenuItemRadio
            && n.state().checked() == Some(SemanticCheckedState::Checked)
    }));

    command(&mut h, "disabled-item", SemanticCommand::RequestFocus);
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let disabled = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::MenuItem && node.name() == Some("Unavailable"))
        .unwrap_or_else(|| unreachable!("disabled item remains semantically discoverable"));
    assert_eq!(
        snapshot.focused(),
        Some(disabled.id()),
        "disabled menu item must be the exact current focus target"
    );
    command(&mut h, "disabled-item", SemanticCommand::Activate);
    assert_eq!(h.state().selected, 0);
}

#[test]
fn menu_command_uses_scoped_fifo_and_submenu_expands_through_app_state() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    open_menu(&mut h);
    command(&mut h, "save-item", SemanticCommand::Activate);
    assert_eq!(h.state().saved, 1);
    assert_eq!(h.state().selected, 0);

    command(&mut h, "submenu-owner", SemanticCommand::RequestFocus);
    command(&mut h, "submenu-owner", SemanticCommand::FocusRight);
    assert!(
        h.state().menu.submenu_open,
        "right arrow opens the focused owning item"
    );
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let owner = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::MenuItem && n.name() == Some("More"))
        .unwrap_or_else(|| unreachable!());
    assert_eq!(owner.state().expanded(), Some(true));
    let controlled = owner
        .relationships()
        .iter()
        .find(|r| r.kind() == SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("mounted submenu is controlled"));
    let nested = snapshot
        .nodes()
        .iter()
        .find(|n| n.id() == controlled.target())
        .unwrap_or_else(|| unreachable!("controlled submenu resolves"));
    assert_eq!(nested.role(), SemanticRole::Menu);
    command(&mut h, "nested-action", SemanticCommand::Activate);
    assert_eq!(h.state().selected, 1);
    command(&mut h, "nested-action", SemanticCommand::FocusLeft);
    assert!(
        !h.state().menu.submenu_open,
        "submenu back action closes only that subtree"
    );
    assert!(h.state().menu.open, "the parent menu remains open");
    command(&mut h, "submenu-owner", SemanticCommand::Expand);
    assert!(h.state().menu.submenu_open);
    command(&mut h, "submenu-owner", SemanticCommand::FocusLeft);
    assert!(
        !h.state().menu.submenu_open,
        "left arrow collapses the owning item"
    );
}

#[test]
fn inline_menu_bar_uses_neutral_horizontal_semantics() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    assert!(h.publish().is_ok());
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    assert!(snapshot.nodes().iter().any(|n| {
        n.role() == SemanticRole::MenuBar
            && n.orientation() == Some(SemanticOrientation::Horizontal)
    }));
    command(&mut h, "bar-view", SemanticCommand::RequestFocus);
    command(&mut h, "bar-view", SemanticCommand::FocusRight);
    assert_eq!(h.state().selected, 0, "focus traversal never activates");
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let tools = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::Button && n.name() == Some("Tools"))
        .unwrap_or_else(|| unreachable!("Tools is mounted"));
    assert_eq!(snapshot.focused(), Some(tools.id()));
    command(&mut h, "bar-tools", SemanticCommand::FocusDown);
    assert_eq!(h.state().selected, 1, "down opens the focused MenuButton");
}

#[test]
fn menu_type_ahead_reuses_mounted_focus_group_without_activation() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    open_menu(&mut h);
    command(&mut h, "disabled-item", SemanticCommand::RequestFocus);
    assert!(
        h.submit_keyboard(runenui_core::KeyboardEvent::new(
            runenui_core::KeyboardPhase::Down,
            runenui_core::PhysicalKey::Code(String::from("KeyS")),
            runenui_core::LogicalKey::Character(String::from("s")),
            runenui_core::KeyModifiers::NONE,
            false,
            runenui_core::KeyLocation::Standard,
            runenui_core::KeyboardCompositionState::Inactive,
            None,
        ))
        .is_ok()
    );
    settle(&mut h);
    assert!(h.publish().is_ok());
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let save = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::MenuItem && n.name() == Some("Save"))
        .unwrap_or_else(|| unreachable!("search target is mounted"));
    assert_eq!(snapshot.focused(), Some(save.id()));
    assert_eq!(h.state().saved, 0, "focus search never invokes command");
}

fn assert_focused_menu_name(h: &TestHarness<MenuApp>, expected: &str) {
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let focused = snapshot
        .focused()
        .unwrap_or_else(|| unreachable!("menu traversal establishes a focus target"));
    let target = snapshot
        .nodes()
        .iter()
        .find(|node| node.id() == focused)
        .unwrap_or_else(|| unreachable!("focused node is in semantic publication"));
    assert_eq!(target.name(), Some(expected));
}

#[test]
fn ordinary_menu_uses_group_arrow_and_absolute_keyboard_navigation_without_activation() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    open_menu(&mut h);
    command(&mut h, "save-item", SemanticCommand::RequestFocus);
    assert_focused_menu_name(&h, "Save");

    command(&mut h, "save-item", SemanticCommand::FocusDown);
    assert_focused_menu_name(&h, "Autosave");
    command(&mut h, "checkbox-item", SemanticCommand::FocusUp);
    assert_focused_menu_name(&h, "Save");

    command(&mut h, "disabled-item", SemanticCommand::RequestFocus);
    for (key, expected) in [
        (runenui_core::LogicalKey::Home, "Save"),
        (runenui_core::LogicalKey::End, "More"),
    ] {
        assert!(
            h.submit_keyboard(runenui_core::KeyboardEvent::new(
                runenui_core::KeyboardPhase::Down,
                runenui_core::PhysicalKey::Code(format!("{key:?}")),
                key,
                runenui_core::KeyModifiers::NONE,
                false,
                runenui_core::KeyLocation::Standard,
                runenui_core::KeyboardCompositionState::Inactive,
                None,
            ))
            .is_ok()
        );
        settle(&mut h);
        assert!(h.publish().is_ok());
        assert_focused_menu_name(&h, expected);
    }

    assert_eq!(h.state().saved, 0, "menu traversal never invokes commands");
    assert!(!h.state().checkbox, "menu traversal never toggles items");
    assert!(!h.state().radio, "menu traversal never selects radio items");
}

#[test]
fn menu_escape_dismisses_and_restores_the_exact_prior_focus() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    assert!(h.publish().is_ok());
    command(&mut h, "menu-button", SemanticCommand::RequestFocus);
    let original = h
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!())
        .focused()
        .cloned()
        .unwrap_or_else(|| unreachable!("focused menu trigger exists"));
    command(&mut h, "menu-button", SemanticCommand::Activate);
    assert!(h.state().menu.open);
    let escape = runenui_core::KeyboardEvent::new(
        runenui_core::KeyboardPhase::Down,
        runenui_core::PhysicalKey::Escape,
        runenui_core::LogicalKey::Escape,
        runenui_core::KeyModifiers::NONE,
        false,
        runenui_core::KeyLocation::Standard,
        runenui_core::KeyboardCompositionState::Inactive,
        None,
    );
    assert!(h.submit_keyboard(escape).is_ok());
    settle(&mut h);
    assert!(!h.state().menu.open);
    assert_eq!(
        h.state().dismissals,
        vec![PresentationDismissReason::CancelOrBack]
    );
    assert!(h.publish().is_ok());
    settle(&mut h);
    assert!(h.publish().is_ok());
    assert_eq!(
        h.semantic_snapshot()
            .unwrap_or_else(|_| unreachable!())
            .focused(),
        Some(&original)
    );
}

#[test]
fn menu_outside_pointer_dismisses_without_dispatching_menu_activation() {
    let mut h = TestHarness::<MenuApp>::mount(Model::default());
    open_menu(&mut h);
    let outside = runenui_core::LogicalPoint::new(790.0, 590.0)
        .unwrap_or_else(|_| unreachable!("finite surface position"));
    let menu = h
        .publication()
        .unwrap_or_else(|| unreachable!("published menu"))
        .frame()
        .nodes()
        .iter()
        .find(|n| n.authored_id().is_some_and(|id| id.as_str() == "file-menu"))
        .unwrap_or_else(|| unreachable!("menu is mounted"));
    assert!(
        !menu.bounds().contains(outside),
        "the outside-click probe must not overlap published menu bounds"
    );
    let context = h
        .input_context()
        .unwrap_or_else(|_| unreachable!("surface is published"))
        .clone();
    let pointer = runenui_core::PointerEvent::new(
        runenui_core::PointerId::new(330).unwrap_or_else(|| unreachable!("nonzero pointer id")),
        runenui_core::PointerDeviceKind::Mouse,
        runenui_core::PointerPhase::Down,
        outside,
        context,
    )
    .with_buttons(runenui_core::PointerButtons::new([
        runenui_core::PointerButton::Primary,
    ]))
    .with_changed_button(runenui_core::PointerButton::Primary);
    assert!(h.submit_pointer(pointer).is_ok());
    settle(&mut h);
    assert!(!h.state().menu.open);
    assert_eq!(
        h.state().dismissals,
        vec![PresentationDismissReason::OutsidePointer]
    );
    assert_eq!(h.state().selected, 0);
}

struct LongMenuApp;

impl UiApp for LongMenuApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        use runenui_core::{
            Axis, LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength, OverflowPolicy,
            OverflowStyle, ScrollControlBinding, scroll_bar, scroll_container,
        };

        let fixed = |width: u16, height: u16| {
            LayoutStyle::default()
                .with_width(LayoutDimension::length(LogicalLength::from(width)))
                .with_height(LayoutDimension::length(LogicalLength::from(height)))
        };
        let items = (0_u8..8)
            .map(|i| {
                MenuItem::new(format!("Choice {i}"))
                    .id(format!("long-item-{i}"))
                    .with_layout(fixed(100, 20))
                    .on_activate(|| ())
                    .into_element()
            })
            .collect::<Vec<_>>();
        let overflow = OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll);
        let content = runenui_core::column(items);
        let binding = ScrollControlBinding::new(Axis::Vertical, LogicalLength::from(5_u8))
            .unwrap_or_else(|_| unreachable!("bounded step"));
        let scroll = scroll_container(content, overflow)
            .id("long-menu-viewport")
            .with_layout(
                fixed(110, 40)
                    .with_container(LayoutContainer::Block)
                    .with_overflow(overflow),
            )
            .scroll_bar(
                scroll_bar(
                    "Long menu vertical scroll",
                    binding,
                    LogicalLength::from(10_u8),
                    LogicalLength::from(20_u8),
                )
                .id("long-menu-scrollbar")
                .exclude_from_focus_group(true),
            );
        runenui_core::column(vec![
            runenui_core::button("Anchor").into_element(),
            Menu::new(vec![scroll.into_element()])
                .id("long-menu")
                .into_element(),
        ])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn long_menu_items_compose_standard_scroll_container_and_scrollbar() {
    let mut h = TestHarness::<LongMenuApp>::mount(());
    assert!(h.publish().is_ok());
    let first = h
        .publication()
        .unwrap_or_else(|| unreachable!("menu publication exists"))
        .frame();
    let viewport = first
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "long-menu-viewport")
        })
        .unwrap_or_else(|| unreachable!("standard scroll viewport is mounted"))
        .bounds();
    assert_eq!(
        viewport.height(),
        40.0,
        "fixed authored scroll viewport height"
    );
    let last_before = h
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("initial menu semantics are published"))
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::MenuItem && node.name() == Some("Choice 7"))
        .unwrap_or_else(|| unreachable!("last item is semantically mounted"))
        .bounds();

    let last_id = first
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "long-item-7")
        })
        .unwrap_or_else(|| unreachable!("last menu item retains identity"))
        .id()
        .clone();
    assert!(
        h.submit_command(
            last_id.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .is_ok()
    );
    assert!(
        h.submit_command(
            last_id.clone(),
            SemanticCommand::ScrollIntoView,
            CommandOrigin::programmatic(),
        )
        .is_ok()
    );
    assert_eq!(
        h.run_until_idle(SettleBudget::new(
            NonZeroUsize::new(12).unwrap_or(NonZeroUsize::MIN),
            PumpBudget::new(128, 128, 128, 128),
        ))
        .outcome(),
        SettleOutcome::Idle
    );
    assert!(h.publish().is_ok());
    let semantics = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let last = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::MenuItem && node.name() == Some("Choice 7"))
        .unwrap_or_else(|| unreachable!("last item still semantically mounted"));
    assert!(
        last.bounds().y() < last_before.y(),
        "shared scroll must move the last item's published semantic bounds; before={last_before:?}, after={:?}, viewport={viewport:?}",
        last.bounds()
    );
    let scrollbar = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ScrollBar)
        .unwrap_or_else(|| unreachable!("standard ScrollBar publishes semantic bounds"));
    assert!(
        last.bounds().y() < scrollbar.bounds().max_y()
            && last.bounds().max_y() > scrollbar.bounds().y(),
        "last menu item must intersect the published standard scrollbar viewport: item={:?}, bar={:?}",
        last.bounds(),
        scrollbar.bounds()
    );
    assert!(
        semantics
            .nodes()
            .iter()
            .any(|n| { n.role() == SemanticRole::MenuItem && n.name() == Some("Choice 7") })
    );

}

#[test]
fn long_menu_group_navigation_stops_at_last_item_without_hiding_scrollbar() {
    let mut h = TestHarness::<LongMenuApp>::mount(());
    assert!(h.publish().is_ok());
    let frame = h
        .publication()
        .unwrap_or_else(|| unreachable!("long menu publication exists"))
        .frame();
    let id_for = |name: &str| {
        frame
            .nodes()
            .iter()
            .find(|node| node.authored_id().is_some_and(|id| id.as_str() == name))
            .unwrap_or_else(|| unreachable!("authored long menu node remains mounted"))
            .id()
            .clone()
    };
    let last_id = id_for("long-item-7");
    let scrollbar_id = id_for("long-menu-scrollbar");
    assert!(
        h.submit_command(
            last_id.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .is_ok()
    );
    settle_long_menu(&mut h);
    let last_semantic_id = h
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("menu semantic publication exists"))
        .focused()
        .cloned()
        .unwrap_or_else(|| unreachable!("last menu item gained focus"));

    assert!(
        h.submit_command(
            last_id,
            SemanticCommand::FocusDown,
            CommandOrigin::programmatic(),
        )
        .is_ok()
    );
    settle_long_menu(&mut h);
    assert_eq!(
        h.semantic_snapshot()
            .unwrap_or_else(|_| unreachable!())
            .focused(),
        Some(&last_semantic_id),
        "Down at last menu item must stop before auxiliary scrollbar chrome"
    );

    assert!(
        h.submit_command(
            scrollbar_id,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .is_ok()
    );
    settle_long_menu(&mut h);
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!());
    let scrollbar = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ScrollBar)
        .unwrap_or_else(|| unreachable!("scrollbar retains semantic accessibility"));
    assert_ne!(Some(scrollbar.id()), Some(&last_semantic_id));
    assert_eq!(snapshot.focused(), Some(scrollbar.id()));
}

fn settle_long_menu(h: &mut TestHarness<LongMenuApp>) {
    assert_eq!(
        h.run_until_idle(SettleBudget::new(
            NonZeroUsize::new(12).unwrap_or(NonZeroUsize::MIN),
            PumpBudget::new(128, 128, 128, 128),
        ))
        .outcome(),
        SettleOutcome::Idle,
    );
    assert!(h.publish().is_ok());
}
