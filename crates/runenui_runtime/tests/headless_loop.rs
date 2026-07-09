use runenui_core::{ElementKind, button, column, text};
use runenui_runtime::{Runtime, RuntimeEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterAction {
    Increment,
    Reset,
}

fn counter_root(counter: &Counter) -> runenui_core::Element<CounterAction> {
    column((
        text("Counter"),
        text(counter.count.to_string()).id("counter.value"),
        button("+").on_press(CounterAction::Increment),
    ))
    .gap(8_u16)
}

const fn update_counter(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Increment => counter.count += 1,
        CounterAction::Reset => counter.count = 0,
    }
}

#[test]
fn mount_builds_initial_root_and_records_trace() {
    let runtime = Runtime::mount(Counter { count: 0 }, counter_root);

    assert_eq!(runtime.state(), &Counter { count: 0 });
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn dispatch_runs_update_and_rebuilds_root() -> Result<(), &'static str> {
    let mut runtime = Runtime::mount(Counter { count: 0 }, counter_root);

    runtime.dispatch(CounterAction::Increment, update_counter, counter_root);

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

    let ElementKind::Container(container) = runtime.root().kind() else {
        return Err("expected root container");
    };

    let Some(value_element) = container.children().get(1) else {
        return Err("expected value element");
    };

    let ElementKind::Text(value_text) = value_element.kind() else {
        return Err("expected value text");
    };

    assert_eq!(value_text.content(), "1");
    Ok(())
}

#[test]
fn multiple_dispatches_keep_state_and_root_in_sync() -> Result<(), &'static str> {
    let mut runtime = Runtime::mount(Counter { count: 0 }, counter_root);

    runtime.dispatch(CounterAction::Increment, update_counter, counter_root);
    runtime.dispatch(CounterAction::Increment, update_counter, counter_root);
    runtime.dispatch(CounterAction::Reset, update_counter, counter_root);

    assert_eq!(runtime.state(), &Counter { count: 0 });

    let ElementKind::Container(container) = runtime.root().kind() else {
        return Err("expected root container");
    };

    let Some(value_element) = container.children().get(1) else {
        return Err("expected value element");
    };

    let ElementKind::Text(value_text) = value_element.kind() else {
        return Err("expected value text");
    };

    assert_eq!(value_text.content(), "0");
    assert_eq!(runtime.trace().events().len(), 10);
    Ok(())
}
