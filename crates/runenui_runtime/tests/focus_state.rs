use runenui_core::prelude::{button, column, text};
use runenui_runtime::prelude::{AppRuntime, FocusState, RuntimeNodeId, UiApp};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Counter {
    count: i32,
}

struct CounterApp;

impl UiApp for CounterApp {
    type State = Counter;
    type Action = Action;

    fn root(state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text(format!("Count: {}", state.count)).id("counter.value"),
            button("+")
                .id("counter.increment")
                .on_press(Action::Increment),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Increment => state.count += 1,
        }
    }
}

#[test]
fn focus_state_starts_empty() {
    let focus = FocusState::new();

    assert_eq!(focus.focused_node(), None);
    assert!(!focus.is_focused(RuntimeNodeId::ROOT));
}

#[test]
fn focus_state_can_set_and_clear_a_runtime_node() {
    let mut focus = FocusState::new();
    let node_id = RuntimeNodeId::from_index(2);

    focus.set(node_id);

    assert_eq!(focus.focused_node(), Some(node_id));
    assert!(focus.is_focused(node_id));
    assert!(!focus.is_focused(RuntimeNodeId::ROOT));

    focus.clear();

    assert_eq!(focus.focused_node(), None);
}

#[test]
fn mounted_runtime_starts_unfocused() {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert_eq!(runtime.focus().focused_node(), None);
}

#[test]
fn app_runtime_sets_focus_to_existing_node() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let increment_id = runtime
        .index()
        .node_by_authored_id("counter.increment")
        .map(runenui_runtime::RuntimeNodeRef::id)
        .ok_or("expected increment node")?;

    assert!(runtime.set_focus(increment_id));

    assert_eq!(runtime.focus().focused_node(), Some(increment_id));
    assert!(runtime.focus().is_focused(increment_id));

    Ok(())
}

#[test]
fn app_runtime_rejects_focus_for_missing_node() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert!(!runtime.set_focus(RuntimeNodeId::from_index(99)));

    assert_eq!(runtime.focus().focused_node(), None);
}

#[test]
fn app_runtime_can_clear_focus() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let increment_id = runtime
        .index()
        .node_by_authored_id("counter.increment")
        .map(runenui_runtime::RuntimeNodeRef::id)
        .ok_or("expected increment node")?;

    assert!(runtime.set_focus(increment_id));
    runtime.clear_focus();

    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn dispatch_clears_focus_because_runtime_node_ids_are_tree_local() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let increment_id = runtime
        .index()
        .node_by_authored_id("counter.increment")
        .map(runenui_runtime::RuntimeNodeRef::id)
        .ok_or("expected increment node")?;

    assert!(runtime.set_focus(increment_id));
    runtime.dispatch(Action::Increment);

    assert_eq!(runtime.state().count, 1);
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}
