use runenui_core::prelude::{button, column};
use runenui_runtime::prelude::{RuntimeNodeId, RuntimeTreeIndex};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Press,
}

#[test]
fn runtime_tree_index_preserves_element_keys() -> Result<(), &'static str> {
    let root = column((
        button("First")
            .id("list.first")
            .key("item-a")
            .on_press(Action::Press),
        button("Second").id("list.second").key("item-b"),
    ));
    let index = RuntimeTreeIndex::new(&root);

    let Some(first) = index.node(RuntimeNodeId::from_index(1)) else {
        return Err("expected first child node");
    };
    let Some(second) = index.node(RuntimeNodeId::from_index(2)) else {
        return Err("expected second child node");
    };

    assert_eq!(
        first.authored_id().map(runenui_core::ElementId::as_str),
        Some("list.first")
    );
    assert_eq!(
        first.element_key().map(runenui_core::ElementKey::as_str),
        Some("item-a")
    );
    assert_eq!(
        second.authored_id().map(runenui_core::ElementId::as_str),
        Some("list.second")
    );
    assert_eq!(
        second.element_key().map(runenui_core::ElementKey::as_str),
        Some("item-b")
    );

    Ok(())
}
