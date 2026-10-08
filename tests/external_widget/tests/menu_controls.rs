#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;

use runenui_core::{
    ApplicationCommand, ApplicationCommandId, CommandBinding, CommandOrigin, CommandScope,
    Menu, MenuBar, MenuButton, MenuItem, MenuItemCheckbox, MenuItemRadio, NoHostProtocol,
    PresentationDismissReason, SemanticCheckedState, SemanticCommand, SemanticOrientation,
    SemanticRelationshipKind, SemanticRole, UiApp, View, column,
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
    open: bool,
    checkbox: bool,
    radio: bool,
    submenu_open: bool,
    selected: usize,
    saved: usize,
    dismissals: Vec<PresentationDismissReason>,
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
        let mut button = MenuButton::new("File", state.open)
            .id("menu-button")
            .on_activate(|| Action::Toggle)
            .on_expand(|| Action::Toggle)
            .on_collapse(|| Action::Toggle);

        if state.open {
            let mut more = MenuItem::new("More")
                .id("submenu-owner")
                .submenu_expanded(state.submenu_open)
                .on_expand(|| Action::ExpandSubmenu)
                .on_collapse(|| Action::CollapseSubmenu);
            if state.submenu_open {
                more = more.with_submenu(
                    Menu::new(vec![
                        MenuItem::new("Nested action")
                            .id("nested-action")
                            .on_activate(|| Action::Selected),
                    ])
                    .submenu()
                    .id("nested-menu")
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
            Action::Toggle => state.open = !state.open,
            Action::ToggleCheckbox => state.checkbox = !state.checkbox,
            Action::SelectRadio => state.radio = true,
            Action::Selected => state.selected += 1,
            Action::Saved => state.saved += 1,
            Action::ExpandSubmenu => state.submenu_open = true,
            Action::CollapseSubmenu => state.submenu_open = false,
            Action::Dismiss(reason) => {
                state.open = false;
                state.submenu_open = false;
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
    assert!(harness.state().open);
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
    assert!(owner.relationships().iter().any(|r| {
        r.kind() == SemanticRelationshipKind::Controls && r.target() == menu.id()
    }));
    assert_eq!(menu.orientation(), Some(SemanticOrientation::Vertical));

    command(
        &mut h,
        "file-menu",
        SemanticCommand::PresentationDismiss(PresentationDismissReason::CancelOrBack),
    );
    assert!(!h.state().open);
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
    assert!(h.semantic_snapshot().unwrap_or_else(|_| unreachable!()).focused().is_some());
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

    command(&mut h, "submenu-owner", SemanticCommand::Expand);
    assert!(h.state().submenu_open);
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
    command(&mut h, "submenu-owner", SemanticCommand::Collapse);
    assert!(!h.state().submenu_open);
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
}
