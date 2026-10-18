#![allow(refining_impl_trait)]
//! One real `ComboBox` focus owner; losing focus dismisses via application action.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticCommand, UiApp, View,
    button, combo_box, option_item, row,
};
use runenui_runtime::{AppRuntime, MountedNodeId, PumpBudget};
struct State {
    open: bool,
    committed: u8,
    requests: Vec<ComboBoxRequest>,
}
enum Action {
    Popup(ComboBoxRequest),
    Other,
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        row(vec![
            combo_box(
                "Choose",
                ElementId::new("focus.combo.list").unwrap_or_else(|_| unreachable!()),
                [option_item("Alpha", state.committed == 1)
                    .id("focus.combo.alpha")
                    .on_activate(|| Action::Other)],
                Action::Popup,
            )
            .id("focus.combo.owner")
            .open(state.open)
            .into_element(),
            button("Next")
                .id("focus.other")
                .on_activate(|| Action::Other)
                .into_element(),
        ])
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        if let Action::Popup(request) = action {
            state.open = matches!(request, ComboBoxRequest::Open);
            state.requests.push(request);
        }
    }
}
fn settle(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
}
fn id(runtime: &mut AppRuntime<App>, text: &str) -> MountedNodeId {
    let authored = ElementId::new(text).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("focus fixture node exists"))
        .id()
        .clone()
}
fn focus(runtime: &mut AppRuntime<App>, target: MountedNodeId) {
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus command admitted"));
    settle(runtime);
}
#[test]
fn leaving_combo_box_real_focus_closes_popup_only_via_application_fifo() {
    let mut runtime = AppRuntime::<App>::mount(State {
        open: true,
        committed: 0,
        requests: Vec::new(),
    });
    settle(&mut runtime);
    let owner = id(&mut runtime, "focus.combo.owner");
    let other = id(&mut runtime, "focus.other");
    focus(&mut runtime, owner);
    assert!(runtime.state().open);
    assert_eq!(runtime.state().requests.as_slice(), &[]);
    focus(&mut runtime, other.clone());
    assert_eq!(runtime.focus().focused_node(), Some(&other));
    assert!(
        !runtime.state().open,
        "popup follows app's committed Close update"
    );
    assert_eq!(runtime.state().requests, [ComboBoxRequest::Close]);
    assert_eq!(runtime.state().committed, 0);
    assert!(!runtime.index().nodes().iter().any(|node| {
        node.authored_id()
            .is_some_and(|id| id.as_str() == "focus.combo.alpha")
    }));
}
