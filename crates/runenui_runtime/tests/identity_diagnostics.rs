#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, button, children, column,
    text,
};
use runenui_runtime::{AppRuntime, DuplicateIdentityKind};

struct App;
impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The authored tree has at most 5 initial owners and no callback output.
        runenui_core::ProducerBudget::new(5, 0)
            .with_reconciliation(5, 0, 5)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 0)
            .with_reconciliation(5, 5, 5)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 5)
            .with_application_outputs(0)
            .with_reconciliation(5, 5, 5)
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root((): &()) -> Element<()> {
        column(children![
            button("A").id("same").key("same"),
            button("B")
                .id(String::from("same"))
                .key(String::from("same")),
            text("invalid id").id("bad\nvalue"),
            text("invalid key").key("bad\tkey"),
        ])
        .into_element()
    }
    fn update((): &mut (), (): ()) {}
}

#[test]
fn authored_duplicates_are_deterministic_in_mounted_preorder() {
    let mut runtime = AppRuntime::<App>::mount(())
        .unwrap_or_else(|_| unreachable!("identity diagnostics fixture admission is valid"));
    let index = runtime.index();
    assert_eq!(index.nodes().len(), 5);
    let exact: Vec<_> = index
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.kind(),
                diagnostic.value().to_owned(),
                diagnostic.first_path().to_owned(),
                diagnostic.duplicate_path().to_owned(),
                diagnostic.preorder_index(),
                diagnostic.to_string(),
            )
        })
        .collect();
    assert_eq!(
        exact,
        vec![
            (
                DuplicateIdentityKind::SiblingKey,
                "same".to_owned(),
                "root/0".to_owned(),
                "root/1".to_owned(),
                2,
                "SiblingKey \"same\": root/0 -> root/1".to_owned(),
            ),
            (
                DuplicateIdentityKind::ElementId,
                "same".to_owned(),
                "root/0".to_owned(),
                "root/1".to_owned(),
                2,
                "ElementId \"same\": root/0 -> root/1".to_owned(),
            ),
            (
                DuplicateIdentityKind::InvalidElementId,
                "bad\nvalue".to_owned(),
                "root/2".to_owned(),
                "root/2".to_owned(),
                3,
                "InvalidElementId \"bad\\nvalue\": root/2 -> root/2".to_owned(),
            ),
            (
                DuplicateIdentityKind::InvalidElementKey,
                "bad\tkey".to_owned(),
                "root/3".to_owned(),
                "root/3".to_owned(),
                4,
                "InvalidElementKey \"bad\\tkey\": root/3 -> root/3".to_owned(),
            ),
        ]
    );

    let exact_target = index.nodes()[1].id().clone();
    drop(index);
    runtime
        .submit_command(
            exact_target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    let repeated: Vec<_> = runtime
        .index()
        .diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        repeated,
        exact.into_iter().map(|entry| entry.5).collect::<Vec<_>>()
    );
}
