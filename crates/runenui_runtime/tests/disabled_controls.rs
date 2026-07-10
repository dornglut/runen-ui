use runenui_core::{Element, button, column, text};
use runenui_runtime::prelude::{
    ActivationResult, AppRuntime, InputIntent, InputIntentHandler, RuntimeEvent, RuntimeNodeId,
    UiApp,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
            text("Counter"),
            button("+")
                .id("counter.increment")
                .on_press(CounterAction::Increment),
            button("Locked")
                .id("counter.locked")
                .on_press(CounterAction::Increment)
                .disabled(),
            button("Reset")
                .id("counter.reset")
                .on_press(CounterAction::Reset)
                .enabled(state.count > 0),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
}

fn runtime_node_id(
    runtime: &AppRuntime<CounterApp>,
    authored_id: &str,
) -> Result<RuntimeNodeId, &'static str> {
    let index = runtime.index();
    let Some(node) = index.node_by_authored_id(authored_id) else {
        return Err("expected authored node");
    };
    Ok(node.id())
}

#[test]
fn disabled_authored_activation_returns_disabled_without_dispatching() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert_eq!(
        runtime.activate("counter.locked"),
        ActivationResult::Disabled
    );

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn disabled_runtime_node_activation_returns_disabled_without_dispatching()
-> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let node_id = runtime_node_id(&runtime, "counter.locked")?;

    assert_eq!(runtime.activate_node(node_id), ActivationResult::Disabled);

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
    Ok(())
}

#[test]
fn disabled_resolved_intent_returns_disabled_without_dispatching() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let node_id = runtime_node_id(&runtime, "counter.locked")?;

    assert_eq!(
        runtime.handle_intent(InputIntent::activate_node(node_id)),
        ActivationResult::Disabled
    );

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
    Ok(())
}

#[test]
fn enabled_button_still_dispatches() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert_eq!(
        runtime.activate("counter.increment"),
        ActivationResult::Dispatched
    );

    assert_eq!(runtime.state(), &Counter { count: 1 });
    assert_eq!(
        runtime.trace().events(),
        &[
            RuntimeEvent::Mounted,
            RuntimeEvent::ActionDispatched,
            RuntimeEvent::StateUpdated,
            RuntimeEvent::RootRebuilt,
        ]
    );
}

#[test]
fn conditionally_disabled_button_can_become_enabled_after_rebuild() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert_eq!(
        runtime.activate("counter.reset"),
        ActivationResult::Disabled
    );
    assert_eq!(
        runtime.activate("counter.increment"),
        ActivationResult::Dispatched
    );
    assert_eq!(
        runtime.activate("counter.reset"),
        ActivationResult::Dispatched
    );

    assert_eq!(runtime.state(), &Counter { count: 0 });
}
