use runenui_core::{Element, ElementId, button, column, text};
use runenui_runtime::{
    ActivationResult, AppRuntime, RuntimeEvent, RuntimeNodeId, TraceRecord, UiApp,
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
        .id("counter.root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
}

struct AnonymousButtonApp;

impl UiApp for AnonymousButtonApp {
    type State = Counter;
    type Action = CounterAction;

    fn root(_state: &Self::State) -> Element<Self::Action> {
        anonymous_button_root()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
}

fn anonymous_button_root() -> Element<CounterAction> {
    column((
        text("Counter"),
        button("+").on_press(CounterAction::Increment),
    ))
}

fn trace_events(runtime: &AppRuntime<CounterApp>) -> Vec<RuntimeEvent> {
    runtime
        .trace()
        .records()
        .iter()
        .map(TraceRecord::event)
        .collect()
}

#[test]
fn direct_dispatch_records_untargeted_trace_records() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    runtime.dispatch(CounterAction::Increment);

    assert_eq!(
        trace_events(&runtime),
        vec![
            RuntimeEvent::Mounted,
            RuntimeEvent::ActionDispatched,
            RuntimeEvent::StateUpdated,
            RuntimeEvent::RootRebuilt,
        ]
    );
    assert!(
        runtime
            .trace()
            .records()
            .iter()
            .all(|record| record.target().is_none())
    );
}

#[test]
fn runtime_node_activation_records_target_details() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let increment_id = {
        let index = runtime.index();
        let Some(node) = index.node_by_authored_id("counter.increment") else {
            return Err("expected increment node");
        };
        node.id()
    };

    assert_eq!(
        runtime.activate_node(increment_id),
        ActivationResult::Dispatched
    );

    let records = runtime.trace().records();
    assert_eq!(records.len(), 4);
    assert!(records[0].target().is_none());

    for record in &records[1..] {
        let Some(target) = record.target() else {
            return Err("expected targeted activation trace record");
        };
        assert_eq!(target.runtime_node_id(), increment_id);
        assert_eq!(
            target.authored_id().map(ElementId::as_str),
            Some("counter.increment")
        );
    }

    Ok(())
}

#[test]
fn authored_activation_records_the_resolved_runtime_node_target() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let increment_id = {
        let index = runtime.index();
        let Some(node) = index.node_by_authored_id("counter.increment") else {
            return Err("expected increment node");
        };
        node.id()
    };

    assert_eq!(
        runtime.activate("counter.increment"),
        ActivationResult::Dispatched
    );

    let Some(target) = runtime.trace().records()[1].target() else {
        return Err("expected targeted action dispatch record");
    };
    assert_eq!(target.runtime_node_id(), increment_id);
    assert_eq!(
        target.authored_id().map(ElementId::as_str),
        Some("counter.increment")
    );

    Ok(())
}

#[test]
fn failed_runtime_node_activation_does_not_append_targeted_records() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::default());

    assert_eq!(
        runtime.activate_node(RuntimeNodeId::from_index(99)),
        ActivationResult::NotFound
    );

    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
    assert_eq!(runtime.trace().records().len(), 1);
    assert!(runtime.trace().records()[0].target().is_none());
}

#[test]
fn trace_target_can_represent_nodes_without_authored_ids() -> Result<(), &'static str> {
    let root = anonymous_button_root();
    let mut runtime = AppRuntime::<AnonymousButtonApp>::mount(Counter::default());
    let button_id = {
        let index = runenui_runtime::RuntimeTreeIndex::new(&root);
        let Some(node) = index
            .nodes()
            .iter()
            .find(|node| matches!(node.element().kind(), runenui_core::ElementKind::Button(_)))
        else {
            return Err("expected anonymous button node");
        };
        node.id()
    };

    assert_eq!(
        runtime.activate_node(button_id),
        ActivationResult::Dispatched
    );

    let Some(target) = runtime.trace().records()[1].target() else {
        return Err("expected targeted action dispatch record");
    };
    assert_eq!(target.runtime_node_id(), button_id);
    assert!(target.authored_id().is_none());

    Ok(())
}
