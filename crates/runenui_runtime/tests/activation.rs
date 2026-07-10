use runenui_core::{Element, ElementKind, button, column, text};
use runenui_runtime::{ActivationResult, AppRuntime, RuntimeEvent, UiApp};

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
fn known_button_id_dispatches_action_and_rebuilds_root() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });

    assert_eq!(
        runtime.activate("counter.increment"),
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
fn unknown_id_returns_not_found() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });

    assert_eq!(
        runtime.activate("counter.missing"),
        ActivationResult::NotFound
    );
    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn non_button_id_returns_not_activatable() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });

    assert_eq!(
        runtime.activate("counter.value"),
        ActivationResult::NotActivatable
    );
    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn button_without_action_returns_no_action() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });

    assert_eq!(
        runtime.activate("counter.no_action"),
        ActivationResult::NoAction
    );
    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn repeated_activation_uses_rebuilt_root() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });

    runtime.activate("counter.increment");
    runtime.activate("counter.increment");
    runtime.activate("counter.reset");

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(value_text(&runtime)?, "0");
    assert_eq!(runtime.trace().events().len(), 10);
    Ok(())
}
