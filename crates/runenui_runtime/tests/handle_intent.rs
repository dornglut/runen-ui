use runenui_core::{Element, ElementKind, button, column, text};
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
            text("Counter").id("counter.title"),
            text(state.count.to_string()).id("counter.value"),
            button("+")
                .id("counter.increment")
                .on_press(CounterAction::Increment),
            button("Reset")
                .id("counter.reset")
                .on_press(CounterAction::Reset),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
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
fn handle_intent_activates_runtime_node() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let increment_node_id = {
        let index = runtime.index();
        let Some(node) = index.node_by_authored_id("counter.increment") else {
            return Err("expected increment node");
        };
        node.id()
    };

    assert_eq!(
        runtime.handle_intent(InputIntent::activate_node(increment_node_id)),
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
fn failed_intent_does_not_mutate_state_or_trace() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert_eq!(
        runtime.handle_intent(InputIntent::activate_node(RuntimeNodeId::from_index(99))),
        ActivationResult::NotFound
    );

    assert_eq!(runtime.state(), &Counter::default());
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}
