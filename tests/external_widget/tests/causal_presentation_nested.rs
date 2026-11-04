#![allow(refining_impl_trait)]

//! Public, executable nested-presentations causal cancellation conformance.

use runenui_core::{
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    Menu, MenuButton, MenuItem, NoHostProtocol, PhysicalKey, StyleEnvironment, UiApp, View, column,
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
    fn initial_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        let initial_nodes = if state.child_open { 7 } else { 5 };
        runenui_core::ProducerBudget::new(initial_nodes, 0)
            .with_application_outputs(0)
            .with_reconciliation(initial_nodes, 0, initial_nodes)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let mut queue_credits = 1;
        let mut outputs = 0;
        let mut mounted_nodes = 0;
        let mut retained_updates = 0;
        let mut dirty_owners = 0;
        for old_open in [false, true] {
            let child_open = match action {
                Action::ShowChild => true,
                Action::HideChild | Action::DismissChild => false,
                Action::DismissParent | Action::Noop => old_open,
            };
            let changed = child_open != old_open;
            let old_nodes: usize = if old_open { 7 } else { 5 };
            let new_nodes: usize = if child_open { 7 } else { 5 };
            let added_nodes = new_nodes.saturating_sub(old_nodes);
            let widget_events = if old_open { 10 } else { 6 };
            let candidate_outputs = if changed { 4 * widget_events + 1 } else { 0 };
            // Retiring the child queues focus restoration through the surviving
            // five-node Menu tree. Its measured route and deferred focus callbacks
            // require 33 root credits after the action's own reconciliation.
            let focus_restore_outputs = usize::from(old_open && !child_open) * 33;
            let candidate_queue_credits =
                added_nodes + usize::from(changed) + focus_restore_outputs;
            queue_credits = queue_credits.max(candidate_queue_credits.max(1));
            outputs = outputs.max(candidate_outputs);
            mounted_nodes = mounted_nodes.max(added_nodes);
            retained_updates = retained_updates.max(old_nodes.min(new_nodes));
            dirty_owners = dirty_owners.max(added_nodes);
        }
        runenui_core::ProducerBudget::new(queue_credits.max(1), outputs)
            .with_application_outputs(0)
            .with_reconciliation(mounted_nodes, retained_updates, dirty_owners)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        let widget_events = 6 + if state.child_open { 4 } else { 0 };
        let max_route_outputs = if state.child_open { 17 } else { 9 };
        // A native key may run its ordinary route, enqueue a semantic default,
        // and trigger focus notifications while the application retires nodes.
        let interaction_outputs = 4 * widget_events + 2 * max_route_outputs + 1;
        let routed_actions = [
            Action::ShowChild,
            Action::HideChild,
            Action::DismissParent,
            Action::DismissChild,
            Action::Noop,
        ]
        .map(|action| Self::action_producer_budget(state, &action));
        let routed_queue_credits = routed_actions
            .iter()
            .map(|profile| profile.queue_credits())
            .max()
            .unwrap_or(1);
        let routed_outputs = routed_actions
            .iter()
            .map(|profile| profile.outputs())
            .max()
            .unwrap_or(0);
        let mounted_nodes = routed_actions
            .iter()
            .map(|profile| profile.mounted_nodes())
            .max()
            .unwrap_or(0);
        let retained_updates = routed_actions
            .iter()
            .map(|profile| profile.retained_updates())
            .max()
            .unwrap_or(0);
        let dirty_owners = routed_actions
            .iter()
            .map(|profile| profile.dirty_subscription_owners())
            .max()
            .unwrap_or(0);
        runenui_core::ProducerBudget::new(
            interaction_outputs.max(routed_queue_credits),
            interaction_outputs.max(routed_outputs),
        )
        .with_application_outputs(0)
        .with_reconciliation(mounted_nodes, retained_updates, dirty_owners)
        .with_subscriptions(0, 0)
        .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

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

const fn escape() -> KeyboardEvent {
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
    })
    .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
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
    })
    .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
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
