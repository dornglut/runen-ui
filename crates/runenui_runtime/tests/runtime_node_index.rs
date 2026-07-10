use runenui_core::{Element, ElementKind, button, column, row, text};
use runenui_runtime::{AppRuntime, RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex, UiApp};

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
        counter_root(state)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
            CounterAction::Reset => state.count = 0,
        }
    }
}

fn counter_root(counter: &Counter) -> Element<CounterAction> {
    column((
        text("Counter").id("counter.title"),
        text(counter.count.to_string()).id("counter.value"),
        row((
            button("+")
                .id("counter.increment")
                .on_press(CounterAction::Increment),
            button("Reset")
                .id("counter.reset")
                .on_press(CounterAction::Reset),
        ))
        .id("counter.controls")
        .gap(8_u16),
    ))
    .id("counter.root")
    .gap(8_u16)
}

#[test]
fn runtime_tree_index_assigns_preorder_node_ids() {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let index = runtime.index();

    let ids: Vec<RuntimeNodeId> = index.nodes().iter().map(RuntimeNodeRef::id).collect();

    assert_eq!(
        ids,
        vec![
            RuntimeNodeId::ROOT,
            RuntimeNodeId::from_index(1),
            RuntimeNodeId::from_index(2),
            RuntimeNodeId::from_index(3),
            RuntimeNodeId::from_index(4),
            RuntimeNodeId::from_index(5),
        ]
    );
}

#[test]
fn runtime_tree_index_records_parent_node_ids() {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let index = runtime.index();

    assert_eq!(
        index.node(RuntimeNodeId::ROOT).map(RuntimeNodeRef::parent),
        Some(None)
    );
    assert_eq!(
        index
            .node(RuntimeNodeId::from_index(1))
            .map(RuntimeNodeRef::parent),
        Some(Some(RuntimeNodeId::ROOT))
    );
    assert_eq!(
        index
            .node(RuntimeNodeId::from_index(4))
            .map(RuntimeNodeRef::parent),
        Some(Some(RuntimeNodeId::from_index(3)))
    );
}

#[test]
fn runtime_tree_index_finds_nodes_by_runtime_id() -> Result<(), &'static str> {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let index = runtime.index();

    let Some(node) = index.node(RuntimeNodeId::from_index(4)) else {
        return Err("expected increment button node");
    };

    assert_eq!(
        node.authored_id().map(runenui_core::ElementId::as_str),
        Some("counter.increment")
    );
    Ok(())
}

#[test]
fn runtime_tree_index_finds_nodes_by_authored_id() -> Result<(), &'static str> {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let index = runtime.index();

    let Some(node) = index.node_by_authored_id("counter.controls") else {
        return Err("expected controls node");
    };

    assert_eq!(node.id(), RuntimeNodeId::from_index(3));

    let ElementKind::Container(container) = node.element().kind() else {
        return Err("expected controls container");
    };

    assert_eq!(container.children().len(), 2);
    Ok(())
}

#[test]
fn runtime_tree_index_returns_none_for_unknown_ids() {
    let runtime = AppRuntime::<CounterApp>::mount(Counter::default());
    let index = runtime.index();

    assert!(index.node(RuntimeNodeId::from_index(99)).is_none());
    assert!(index.node_by_authored_id("counter.missing").is_none());
}

#[test]
fn runtime_tree_index_can_be_built_from_a_plain_root_element() {
    let root = counter_root(&Counter::default());
    let index = RuntimeTreeIndex::new(&root);

    assert_eq!(index.nodes().len(), 6);
    assert_eq!(
        index
            .node_by_authored_id("counter.root")
            .map(RuntimeNodeRef::id),
        Some(RuntimeNodeId::ROOT)
    );
}
