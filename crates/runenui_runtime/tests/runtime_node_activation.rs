use runenui_core::{Element, ElementKind, button, column, text};
use runenui_runtime::{
    ActivationResult, AppRuntime, RuntimeEvent, RuntimeNodeId, RuntimeNodeRef, UiApp,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterAction {
    Increment,
    Reset,
}

struct CounterApp;

impl UiApp for CounterApp {
    type State = Counter;
    type Action = CounterAction;

    fn root(state: &Self::State) -> Element<Self::Action> {
        column((
            text("Counter").id("counter.title"),
            text(state.count.to_string()).id("counter.value"),
            button("+")
                .id("counter.increment")
                .on_press(CounterAction::Increment),
            button("Reset")
                .id("counter.reset")
                .on_press(CounterAction::Reset),
            button("No action").id("counter.no_action"),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
}

fn node_id(
    runtime: &AppRuntime<CounterApp>,
    authored_id: &str,
) -> Result<RuntimeNodeId, &'static str> {
    runtime
        .index()
        .node_by_authored_id(authored_id)
        .map(RuntimeNodeRef::id)
        .ok_or("expected authored node")
}

fn value_text(runtime: &AppRuntime<CounterApp>) -> Result<String, &'static str> {
    let ElementKind::Container(container) = runtime.root().kind() else {
        return Err("expected root container");
    };
    let Some(value) = container.children().get(1) else {
        return Err("expected value element");
    };
    let ElementKind::Text(text) = value.kind() else {
        return Err("expected value text");
    };

    Ok(text.content().to_owned())
}

#[test]
fn runtime_node_activation_dispatches_action_and_rebuilds_root() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let increment = node_id(&runtime, "counter.increment")?;

    assert_eq!(
        runtime.activate_node(increment),
        ActivationResult::Dispatched
    );

    assert_eq!(runtime.state(), &Counter { count: 1 });
    assert_eq!(value_text(&runtime)?, "1");
    assert_eq!(
        runtime.trace().events(),
        &[
            RuntimeEvent::Mounted,
            RuntimeEvent::ActionDispatched,
            RuntimeEvent::StateUpdated,
            RuntimeEvent::RootRebuilt,
        ]
    );
    Ok(())
}

#[test]
fn authored_activation_uses_the_same_node_activation_path() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let increment = node_id(&runtime, "counter.increment")?;

    assert_eq!(
        runtime.activate("counter.increment"),
        ActivationResult::Dispatched
    );
    assert_eq!(
        runtime.activate_node(increment),
        ActivationResult::Dispatched
    );

    assert_eq!(runtime.state(), &Counter { count: 2 });
    assert_eq!(value_text(&runtime)?, "2");
    Ok(())
}

#[test]
fn unknown_runtime_node_id_returns_not_found() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });

    assert_eq!(
        runtime.activate_node(RuntimeNodeId::from_index(99)),
        ActivationResult::NotFound
    );
    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn non_button_runtime_node_id_returns_not_activatable() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let value = node_id(&runtime, "counter.value")?;

    assert_eq!(
        runtime.activate_node(value),
        ActivationResult::NotActivatable
    );
    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
    Ok(())
}

#[test]
fn button_without_action_runtime_node_id_returns_no_action() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let no_action = node_id(&runtime, "counter.no_action")?;

    assert_eq!(runtime.activate_node(no_action), ActivationResult::NoAction);
    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
    Ok(())
}

#[test]
fn repeated_runtime_node_activation_uses_rebuilt_root() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let increment = node_id(&runtime, "counter.increment")?;
    let reset = node_id(&runtime, "counter.reset")?;

    runtime.activate_node(increment);
    runtime.activate_node(increment);
    runtime.activate_node(reset);

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(value_text(&runtime)?, "0");
    assert_eq!(runtime.trace().events().len(), 10);
    Ok(())
}
