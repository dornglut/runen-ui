use runenui_core::{Element, IntoElement, button, children, column};
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
        column(children![
            button("A")
                .id("duplicate")
                .key("same")
                .on_press(Action::Hit),
            button("B")
                .id("duplicate")
                .key("same")
                .on_press(Action::Hit),
            button("Invalid").id(" "),
        ])
        .into_element()
    }
    fn update((): &mut (), _: Action) {}
}

#[test]
fn duplicate_and_invalid_identity_diagnostics_are_stable_and_no_first_match_wins() {
    let mut runtime = AppRuntime::<App>::mount(());
    let index = runtime.index();
    let diagnostics = index.diagnostics();
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[0].kind(), DuplicateIdentityKind::ElementId);
    assert_eq!(diagnostics[0].first_path(), "root/0");
    assert_eq!(diagnostics[0].duplicate_path(), "root/1");
    assert_eq!(diagnostics[1].kind(), DuplicateIdentityKind::SiblingKey);
    assert_eq!(
        diagnostics[1].to_string(),
        "duplicate SiblingKey \"same\": first at root/0, duplicate at root/1"
    );
    assert_eq!(
        diagnostics[2].kind(),
        DuplicateIdentityKind::InvalidElementId
    );
    assert_eq!(diagnostics[2].duplicate_path(), "root/2");
    drop(index);
    assert_eq!(runtime.activate("duplicate"), ActivationResult::AmbiguousId);
}
