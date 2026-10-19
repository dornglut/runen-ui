#![allow(refining_impl_trait)]

//! Public, executable nested-presentations causal cancellation conformance.

use runenui_core::{
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LogicalKey, Menu, MenuButton, MenuItem, NoHostProtocol, PhysicalKey, StyleEnvironment,
    UiApp, View, column,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

const FULL: PumpBudget = PumpBudget::new(128, 128, 128, 128);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Debug, Clone, Copy)]
enum Action {
    ShowChild,
    HideChild,
    DismissParent,
    DismissChild,
    Noop,
}

#[derive(Debug, Clone)]
struct Model {
    child_open: bool,
    parent_dismissals: usize,
    child_dismissals: usize,
}

struct NestedApp;

impl UiApp for NestedApp {
    type State = Model;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Model) -> impl View<Action> {
        let mut child_trigger = MenuItem::new("Child trigger")
            .id("child-trigger")
            .submenu_expanded(state.child_open);
        if state.child_open {
            child_trigger = child_trigger.with_submenu(
                Menu::new(vec![
                    MenuItem::new("Child action")
                        .id("child-action")
                        .on_activate(|| Action::Noop),
                ])
                .submenu()
                .id("child-menu")
                .on_dismiss(|_| Action::DismissChild),
                true,
            );
        }
        let parent = MenuButton::new("Parent", true)
            .id("parent-trigger")
            .with_submenu(
                Menu::new(vec![
                    MenuItem::new("Parent action")
                        .id("parent-action")
                        .on_activate(|| Action::Noop)
                        .into_element(),
                    child_trigger.into_element(),
                ])
                .id("parent-menu")
                .on_dismiss(|_| Action::DismissParent),
                true,
            );
        column(vec![parent.into_element()])
    }

    fn update(state: &mut Model, action: Action) {
        match action {
            Action::ShowChild => state.child_open = true,
            Action::HideChild => state.child_open = false,
            Action::DismissParent => state.parent_dismissals += 1,
            Action::DismissChild => {
                state.child_open = false;
                state.child_dismissals += 1;
            }
            Action::Noop => {}
        }
    }
}

fn escape() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Escape,
        LogicalKey::Escape,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn publish(runtime: &mut AppRuntime<NestedApp>, context: &SurfaceBuildContext<'_>) {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("nested publication"));
}

fn full(runtime: &mut AppRuntime<NestedApp>) {
    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("bounded full drain"))
            .report()
            .is_quiescent()
    );
}

fn one(runtime: &mut AppRuntime<NestedApp>) {
    assert_eq!(
        runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("one envelope"))
            .report()
            .processed_envelopes(),
        1
    );
}

fn mounted(runtime: &mut AppRuntime<NestedApp>, name: &str) -> runenui_core::MountedNodeId {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == name))
        .unwrap_or_else(|| unreachable!("mounted authored ID"))
        .id()
        .clone()
}

#[test]
fn c6_child_origin_never_dismisses_parent_after_child_retirement() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<NestedApp>::mount(Model {
        child_open: true,
        parent_dismissals: 0,
        child_dismissals: 0,
    });
    publish(&mut runtime, &context);
    full(&mut runtime);
    let child_item = mounted(&mut runtime, "child-action");
    assert_eq!(runtime.focus().focused_node(), Some(&child_item));
    runtime
        .submit_keyboard(escape())
        .unwrap_or_else(|_| unreachable!("keyboard under child"));
    runtime
        .submit_action(Action::HideChild)
        .unwrap_or_else(|_| unreachable!("retire child behind keyboard"));
    one(&mut runtime); // keyboard derives a bound command
    one(&mut runtime); // close child before derived command processes
    publish(&mut runtime, &context);
    full(&mut runtime);
    assert!(!runtime.state().child_open);
    assert_eq!(runtime.state().parent_dismissals, 0);
    assert_eq!(runtime.state().child_dismissals, 0);
    let parent_item = mounted(&mut runtime, "parent-action");
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&parent_item),
        "nested closure restores focus inside the surviving parent"
    );
}

#[test]
fn c6_parent_origin_cannot_dismiss_newly_published_nested_child() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<NestedApp>::mount(Model {
        child_open: false,
        parent_dismissals: 0,
        child_dismissals: 0,
    });
    publish(&mut runtime, &context);
    full(&mut runtime);
    let parent_item = mounted(&mut runtime, "parent-action");
    assert_eq!(runtime.focus().focused_node(), Some(&parent_item));
    runtime
        .submit_keyboard(escape())
        .unwrap_or_else(|_| unreachable!("keyboard under parent"));
    runtime
        .submit_action(Action::ShowChild)
        .unwrap_or_else(|_| unreachable!("create child behind keyboard"));
    one(&mut runtime);
    one(&mut runtime);
    publish(&mut runtime, &context);
    full(&mut runtime);
    assert!(runtime.state().child_open);
    assert_eq!(runtime.state().parent_dismissals, 0);
    assert_eq!(runtime.state().child_dismissals, 0);
}
