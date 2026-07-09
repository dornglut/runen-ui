use runenui_core::{Element, ElementKind, button, column, text};
use runenui_runtime::{AppRuntime, RuntimeEvent, UiApp};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

impl Counter {
    const fn new() -> Self {
        Self { count: 0 }
    }
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
            text(state.count.to_string()).id("counter.value"),
            button("+").on_press(CounterAction::Increment),
        ))
        .gap(8_u16)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
}

fn value_text(root: &Element<CounterAction>) -> Result<&str, &'static str> {
    let ElementKind::Container(container) = root.kind() else {
        return Err("expected root container");
    };
    let Some(value) = container.children().get(1) else {
        return Err("expected value element");
    };
    let ElementKind::Text(text) = value.kind() else {
        return Err("expected value text");
    };
    Ok(text.content())
}

#[test]
fn mount_builds_root_once() -> Result<(), &'static str> {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::new());

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(value_text(runtime.root())?, "0");
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
    Ok(())
}

#[test]
fn dispatch_updates_state_and_rebuilds_root_without_repassing_handlers() -> Result<(), &'static str>
{
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

    runtime.dispatch(CounterAction::Increment);

    assert_eq!(runtime.state(), &Counter { count: 1 });
    assert_eq!(value_text(runtime.root())?, "1");
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
fn repeated_dispatches_keep_state_and_root_in_sync() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

    runtime.dispatch(CounterAction::Increment);
    runtime.dispatch(CounterAction::Increment);
    runtime.dispatch(CounterAction::Reset);

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(value_text(runtime.root())?, "0");
    assert_eq!(runtime.trace().events().len(), 10);
    Ok(())
}
