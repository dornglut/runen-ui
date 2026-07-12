use runenui_core::{Element, ElementId, IntoElement, button, column, element_id, element_key};
use runenui_runtime::{ActivationResult, AppRuntime, DuplicateIdentityKind, UiApp};

#[derive(Clone)]
enum Action {
    Hit,
}

struct App;

impl UiApp for App {
    type State = ();
    type Action = Action;

    fn root((): &()) -> Element<Action> {
        let mut children = Vec::new();
        children.push(
            button("first")
                .id(element_id!("mixed.identity"))
                .key(element_key!("mixed.key"))
                .on_press(Action::Hit)
                .into_element(),
        );
        children.push(button("invalid-before-ten").id("\u{00A0}").into_element());
        children.push(
            button("all-categories")
                .id("\u{00A0}bad")
                .id("mixed.identity")
                .key("bad\u{2003}")
                .key("mixed.key")
                .on_press(Action::Hit)
                .into_element(),
        );
        for index in 3..10 {
            children.push(button(format!("child-{index}")).into_element());
        }
        children.push(
            button("ten")
                .id("mixed.identity")
                .key("mixed.key")
                .on_press(Action::Hit)
                .into_element(),
        );
        children.push(button("eleven").into_element());
        children.push(button("twelve").into_element());
        column(children).into_element()
    }

    fn update((): &mut (), _: Action) {}
}

#[test]
fn mixed_storage_identity_and_true_preorder_diagnostics_are_stable()
-> Result<(), runenui_core::IdentifierError> {
    let mut runtime = AppRuntime::<App>::mount(());
    let index = runtime.index();

    let dynamic_lookup = ElementId::new("mixed.identity")?;
    assert_eq!(
        index
            .node_by_authored_id(&dynamic_lookup)
            .map(|node| node.id().as_usize()),
        Some(1)
    );

    let diagnostics = index.diagnostics();
    let observed: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.preorder_index(),
                diagnostic.duplicate_path(),
                diagnostic.kind(),
            )
        })
        .collect();
    assert_eq!(
        observed,
        vec![
            (2, "root/1", DuplicateIdentityKind::InvalidElementId),
            (3, "root/2", DuplicateIdentityKind::InvalidElementId),
            (3, "root/2", DuplicateIdentityKind::InvalidElementKey),
            (3, "root/2", DuplicateIdentityKind::ElementId),
            (3, "root/2", DuplicateIdentityKind::SiblingKey),
            (11, "root/10", DuplicateIdentityKind::ElementId),
            (11, "root/10", DuplicateIdentityKind::SiblingKey),
        ]
    );
    assert_eq!(diagnostics[3].first_path(), "root/0");
    assert_eq!(diagnostics[4].first_path(), "root/0");
    assert_eq!(
        diagnostics[4].to_string(),
        "duplicate SiblingKey \"mixed.key\": first at root/0, duplicate at root/2"
    );

    let repeated = AppRuntime::<App>::mount(());
    assert_eq!(diagnostics, repeated.index().diagnostics());
    drop(index);
    assert_eq!(
        runtime.activate("mixed.identity"),
        ActivationResult::AmbiguousId
    );
    Ok(())
}
