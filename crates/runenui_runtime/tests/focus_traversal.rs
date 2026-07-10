use runenui_core::prelude::{button, column, row, text};
use runenui_runtime::prelude::{AppRuntime, RuntimeNodeId, RuntimeNodeRef, UiApp};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Hit,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State;

struct FocusApp;

impl UiApp for FocusApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title").id("title"),
            button("First").id("first").on_press(Action::Hit),
            button("Disabled")
                .id("disabled")
                .on_press(Action::Hit)
                .disabled(),
            row((
                button("Second").id("second").on_press(Action::Hit),
                text("Nested text").id("nested.text"),
            )),
            button("No action").id("no-action"),
        ))
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

struct NoFocusableApp;

impl UiApp for NoFocusableApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title").id("title"),
            button("Disabled")
                .id("disabled")
                .on_press(Action::Hit)
                .disabled(),
        ))
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

fn node_id(
    runtime: &AppRuntime<FocusApp>,
    authored_id: &str,
) -> Result<RuntimeNodeId, &'static str> {
    runtime
        .index()
        .node_by_authored_id(authored_id)
        .map(RuntimeNodeRef::id)
        .ok_or("expected authored node")
}

fn focusable_authored_ids(runtime: &AppRuntime<FocusApp>) -> Vec<String> {
    runtime
        .index()
        .focusable_nodes()
        .filter_map(RuntimeNodeRef::authored_id)
        .map(|id| id.as_str().to_owned())
        .collect()
}

#[test]
fn index_reports_focusable_nodes_in_traversal_order() {
    let runtime = AppRuntime::<FocusApp>::mount(State);

    assert_eq!(
        focusable_authored_ids(&runtime),
        ["first", "second", "no-action"]
    );
}

#[test]
fn set_focus_rejects_existing_non_focusable_nodes() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<FocusApp>::mount(State);
    let title_id = node_id(&runtime, "title")?;
    let disabled_id = node_id(&runtime, "disabled")?;

    assert!(!runtime.set_focus(title_id));
    assert_eq!(runtime.focus().focused_node(), None);

    assert!(!runtime.set_focus(disabled_id));
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn focus_first_and_last_use_focusable_order() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<FocusApp>::mount(State);
    let first_id = node_id(&runtime, "first")?;
    let no_action_id = node_id(&runtime, "no-action")?;

    assert_eq!(runtime.focus_first(), Some(first_id));
    assert_eq!(runtime.focus().focused_node(), Some(first_id));

    assert_eq!(runtime.focus_last(), Some(no_action_id));
    assert_eq!(runtime.focus().focused_node(), Some(no_action_id));

    Ok(())
}

#[test]
fn focus_next_walks_focusable_nodes_and_wraps() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<FocusApp>::mount(State);
    let first_id = node_id(&runtime, "first")?;
    let second_id = node_id(&runtime, "second")?;
    let no_action_id = node_id(&runtime, "no-action")?;

    assert_eq!(runtime.focus_next(), Some(first_id));
    assert_eq!(runtime.focus_next(), Some(second_id));
    assert_eq!(runtime.focus_next(), Some(no_action_id));
    assert_eq!(runtime.focus_next(), Some(first_id));

    Ok(())
}

#[test]
fn focus_previous_walks_focusable_nodes_and_wraps() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<FocusApp>::mount(State);
    let first_id = node_id(&runtime, "first")?;
    let second_id = node_id(&runtime, "second")?;
    let no_action_id = node_id(&runtime, "no-action")?;

    assert_eq!(runtime.focus_previous(), Some(no_action_id));
    assert_eq!(runtime.focus_previous(), Some(second_id));
    assert_eq!(runtime.focus_previous(), Some(first_id));
    assert_eq!(runtime.focus_previous(), Some(no_action_id));

    Ok(())
}

#[test]
fn traversal_returns_none_and_clears_focus_without_focusable_nodes() {
    let mut runtime = AppRuntime::<NoFocusableApp>::mount(State);

    assert_eq!(runtime.focus_first(), None);
    assert_eq!(runtime.focus_last(), None);
    assert_eq!(runtime.focus_next(), None);
    assert_eq!(runtime.focus_previous(), None);
    assert_eq!(runtime.focus().focused_node(), None);
}
